use std::sync::Arc;
use std::time::Duration;

use datafusion::arrow::array::{Int64Array, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::MemTable;
use datafusion::prelude::{SessionConfig, SessionContext};
use tokio::sync::Semaphore;

use super::{MAX_PARTITIONS, admit_on, bounded, is_out_of_memory, runtime_with};

/// `rows` rows, each with its own long string: every one is a distinct value.
fn distinct_strings(rows: usize) -> MemTable {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("s", DataType::Utf8, false),
    ]));
    let ids = Int64Array::from_iter_values(0..rows as i64);
    let strings = StringArray::from_iter_values((0..rows).map(|i| format!("an address that is unique, number {i:08}")));
    let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(ids), Arc::new(strings)]).unwrap();
    MemTable::try_new(schema, vec![vec![batch]]).unwrap()
}

#[test]
fn a_query_is_split_into_at_most_the_capped_partitions() {
    assert_eq!(bounded(SessionConfig::new().with_target_partitions(64)).target_partitions(), MAX_PARTITIONS);
    assert_eq!(bounded(SessionConfig::new().with_target_partitions(1)).target_partitions(), 1);
}

#[tokio::test]
async fn a_query_past_the_pool_fails_on_its_own_and_the_next_one_runs() {
    // What took an office down: an exact COUNT(DISTINCT) over a column whose
    // every value differs. Unbounded, it holds them all; here it is refused.
    let runtime = runtime_with(256 * 1024);
    let ctx = SessionContext::new_with_config_rt(bounded(SessionConfig::new()), runtime.clone());
    ctx.register_table("t", Arc::new(distinct_strings(200_000))).unwrap();
    let err = ctx
        .sql("SELECT COUNT(DISTINCT s) FROM t")
        .await
        .unwrap()
        .collect()
        .await
        .expect_err("past a 256 KiB pool");
    assert!(is_out_of_memory(&err), "an out-of-memory error, got: {err}");

    // The pool is whole again: a small query on the same runtime answers.
    let next = SessionContext::new_with_config_rt(bounded(SessionConfig::new()), runtime);
    next.register_table("t", Arc::new(distinct_strings(10))).unwrap();
    let batches = next.sql("SELECT COUNT(*) AS n FROM t").await.unwrap().collect().await.unwrap();
    assert_eq!(batches[0].num_rows(), 1);
}

#[tokio::test]
async fn a_query_waits_for_a_turn_and_is_turned_away_when_none_frees() {
    let turns = Semaphore::new(2);
    let first = admit_on(&turns, Duration::from_millis(50)).await.expect("a first turn");
    let _second = admit_on(&turns, Duration::from_millis(50)).await.expect("a second turn");
    assert!(admit_on(&turns, Duration::from_millis(50)).await.is_none(), "no third turn while two run");
    drop(first);
    assert!(admit_on(&turns, Duration::from_millis(50)).await.is_some(), "a turn once one ends");
}
