use std::fs;
use std::path::Path;
use std::sync::Arc;

use datafusion::arrow::array::{BinaryArray, Float64Array, Int64Array, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::execution::memory_pool::FairSpillPool;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::prelude::{SessionConfig, SessionContext};
use parquet::file::metadata::KeyValue;
use parquet::file::properties::WriterProperties;
use serde_json::{Value, json};

use super::*;
use crate::mapserver::resolve::geoparquet_optimize::OPTIMIZED_ROW_GROUP_ROWS;

// ── Invented layers ──────────────────────────────────────────────────

/// What a synthetic layer's footer carries besides its columns.
#[derive(Clone, Copy)]
enum Geo {
    /// `geo` metadata with a `bbox` of its own.
    WithBbox,
    /// `geo` metadata that declares the bbox covering columns but no `bbox`.
    CoveringOnly,
}

/// A GeoParquet layer of `rows` sites, written `group` rows to a row group.
///
/// - `name`: `site {i}`, every value distinct
/// - `suburb`: by `i % 10`, north ×5, south ×3, east ×1, null ×1
/// - `area`: `i * 0.5`, null when `i % 7 == 0`
/// - `floors`: `i % 13`
/// - `geometry` (WKB points) and the bbox columns, which are not attributes
fn write_sites(path: &Path, rows: usize, group: usize, geo: Geo) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("geometry", DataType::Binary, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("suburb", DataType::Utf8, true),
        Field::new("area", DataType::Float64, true),
        Field::new("floors", DataType::Int64, false),
        Field::new("xmin", DataType::Float64, false),
        Field::new("ymin", DataType::Float64, false),
        Field::new("xmax", DataType::Float64, false),
        Field::new("ymax", DataType::Float64, false),
    ]));
    let bbox = match geo {
        Geo::WithBbox => r#","bbox":[1.0,2.0,3.0,4.0]"#,
        Geo::CoveringOnly => "",
    };
    let geo_json = format!(
        r#"{{"version":"1.1.0","primary_column":"geometry","columns":{{"geometry":{{"encoding":"WKB","geometry_types":["Point"]{bbox},"covering":{{"bbox":{{"xmin":["xmin"],"ymin":["ymin"],"xmax":["xmax"],"ymax":["ymax"]}}}}}}}}}}"#
    );
    let props = WriterProperties::builder()
        .set_max_row_group_row_count(Some(group))
        .set_key_value_metadata(Some(vec![KeyValue::new("geo".to_string(), Some(geo_json))]))
        .build();
    let file = fs::File::create(path).expect("create layer");
    let mut writer =
        parquet::arrow::ArrowWriter::try_new(file, schema.clone(), Some(props)).expect("writer");
    let chunk = 100_000;
    for start in (0..rows).step_by(chunk) {
        let range = start..(start + chunk).min(rows);
        let x = |i: usize| 100.0 + (i % 100) as f64 * 0.01;
        let y = |i: usize| -5.0 - (i % 50) as f64 * 0.01;
        let geometry: Vec<Vec<u8>> = range.clone().map(|i| wkb_point(x(i), y(i))).collect();
        let suburbs = range.clone().map(|i| match i % 10 {
            0..=4 => Some("north"),
            5..=7 => Some("south"),
            8 => Some("east"),
            _ => None,
        });
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(BinaryArray::from_iter_values(geometry.iter())),
                Arc::new(StringArray::from_iter_values(
                    range.clone().map(|i| format!("site {i}")),
                )),
                Arc::new(StringArray::from_iter(suburbs)),
                Arc::new(Float64Array::from_iter(
                    range
                        .clone()
                        .map(|i| (i % 7 != 0).then_some(i as f64 * 0.5)),
                )),
                Arc::new(Int64Array::from_iter_values(
                    range.clone().map(|i| (i % 13) as i64),
                )),
                Arc::new(Float64Array::from_iter_values(range.clone().map(x))),
                Arc::new(Float64Array::from_iter_values(range.clone().map(y))),
                Arc::new(Float64Array::from_iter_values(range.clone().map(x))),
                Arc::new(Float64Array::from_iter_values(range.clone().map(y))),
            ],
        )
        .expect("batch");
        writer.write(&batch).expect("write");
    }
    writer.close().expect("close");
}

fn wkb_point(x: f64, y: f64) -> Vec<u8> {
    let mut wkb = vec![1u8, 1, 0, 0, 0];
    wkb.extend_from_slice(&x.to_le_bytes());
    wkb.extend_from_slice(&y.to_le_bytes());
    wkb
}

fn geoparquet_layer(source: &Path, allowed: &[&str]) -> PublishedLayerManifest {
    PublishedLayerManifest {
        layer_id: "sites".to_string(),
        path: "sites".to_string(),
        source_kind: SourceKind::GeoParquet,
        source_ref: source.display().to_string(),
        mode: "features".to_string(),
        min_zoom: None,
        max_zoom: None,
        bbox_required: false,
        max_features: 100,
        allowed_properties: allowed.iter().map(|s| s.to_string()).collect(),
        style: None,
        filter: None,
        function_slug: None,
        cache_ttl_secs: None,
    }
}

fn geojson_layer(dir: &Path, allowed: Vec<String>) -> PublishedLayerManifest {
    let source = dir.join("layer.geojson");
    fs::write(
        &source,
        serde_json::to_string(&json!({
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": { "name": "A", "owner_phone": "+61400000000", "floors": 2 },
                    "geometry": { "type": "Point", "coordinates": [106.0, -6.0] }
                },
                {
                    "type": "Feature",
                    "properties": { "name": "B", "owner_phone": "+61400000001", "floors": 4 },
                    "geometry": { "type": "Point", "coordinates": [107.0, -6.1] }
                },
                {
                    "type": "Feature",
                    "properties": { "name": "A", "owner_phone": null },
                    "geometry": { "type": "Point", "coordinates": [106.5, -6.2] }
                }
            ]
        }))
        .expect("source json"),
    )
    .expect("write source");
    PublishedLayerManifest {
        layer_id: "adm1".to_string(),
        source_kind: SourceKind::GeoJsonFile,
        source_ref: source.display().to_string(),
        allowed_properties: allowed,
        ..geoparquet_layer(&source, &[])
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn default_session(config: SessionConfig) -> SessionContext {
    SessionContext::new_with_config(config)
}

/// Sessions sharing one memory pool of `bytes`, split into 4 partitions, the
/// most the platform's runtime allows.
fn small_pool_session(bytes: usize) -> SessionContext {
    let runtime = RuntimeEnvBuilder::new()
        .with_memory_pool(Arc::new(FairSpillPool::new(bytes)))
        .build_arc()
        .expect("runtime");
    SessionContext::new_with_config_rt(SessionConfig::new().with_target_partitions(4), runtime)
}

fn compute_now(source: &Path, ctx: &SessionContext) -> Result<LayerStats, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(compute::compute(source, ctx))
}

fn column<'a>(stats: &'a LayerStats, name: &str) -> &'a ColumnStats {
    stats
        .columns
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("column {name}"))
}

fn names(answer: &Value) -> Vec<String> {
    let mut names: Vec<String> = answer["columns"]
        .as_array()
        .expect("columns array")
        .iter()
        .map(|c| c["name"].as_str().unwrap_or_default().to_string())
        .collect();
    names.sort();
    names
}

fn top(pairs: &[(&str, u64)]) -> Vec<(String, u64)> {
    pairs.iter().map(|(v, n)| (v.to_string(), *n)).collect()
}

// ── What is computed ─────────────────────────────────────────────────

#[test]
fn a_layers_statistics_are_exact_except_large_distinct_counts() {
    let tmp = tempfile::tempdir().expect("tmp");
    let source = tmp.path().join("sites.parquet");
    write_sites(&source, 30_000, 4_096, Geo::WithBbox);
    let stats = compute_now(&source, &SessionContext::new()).expect("statistics");

    assert_eq!(stats.row_count, 30_000);
    let mut attributes: Vec<&str> = stats.columns.iter().map(|c| c.name.as_str()).collect();
    attributes.sort();
    assert_eq!(
        attributes,
        ["area", "floors", "name", "suburb"],
        "geometry and bbox columns are not attributes"
    );
    assert_eq!(
        stats.extent,
        Some([1.0, 2.0, 3.0, 4.0]),
        "the extent recorded in the geo metadata"
    );

    let suburb = column(&stats, "suburb");
    assert_eq!(suburb.data_type, "Utf8");
    assert_eq!(suburb.null_count, 3_000);
    assert_eq!((suburb.cardinality, suburb.cardinality_exact), (3, true));
    assert_eq!(
        suburb.top_values,
        top(&[("north", 15_000), ("south", 9_000), ("east", 3_000)])
    );
    assert_eq!(
        (suburb.min, suburb.max, &suburb.histogram),
        (None, None, &None),
        "text has no min, max or histogram"
    );

    // 30,000 values over 13 floors: 0 to 8 appear 2,308 times, 9 to 12 2,307.
    let floors = column(&stats, "floors");
    assert_eq!(
        (
            floors.null_count,
            floors.cardinality,
            floors.cardinality_exact
        ),
        (0, 13, true)
    );
    let mut expected: Vec<(String, u64)> = (0..9).map(|v| (v.to_string(), 2_308)).collect();
    expected.push(("10".to_string(), 2_307));
    assert_eq!(
        floors.top_values, expected,
        "most frequent first, ties by value, at most {TOP_VALUES}"
    );
    assert_eq!((floors.min, floors.max), (Some(0.0), Some(12.0)));
    let histogram = floors
        .histogram
        .as_ref()
        .expect("a numeric column has a histogram");
    assert_eq!(histogram.counts.len(), HISTOGRAM_BINS);
    assert_eq!(
        histogram.counts.iter().sum::<u64>(),
        30_000 - floors.null_count
    );
    assert_eq!(histogram.counts[0], 2_308, "0 alone falls in the first bin");
    assert_eq!(
        histogram.counts[HISTOGRAM_BINS - 1],
        2_307,
        "the maximum, 12, falls in the last bin"
    );

    // Null where i % 7 == 0: i = 0, 7, …, 29,995 is 4,286 rows.
    let area = column(&stats, "area");
    assert_eq!(area.null_count, 4_286);
    assert_eq!((area.min, area.max), (Some(0.5), Some(14_999.5)));
    let histogram = area.histogram.as_ref().expect("histogram");
    assert_eq!(histogram.counts.iter().sum::<u64>(), 30_000 - 4_286);
    assert_eq!(
        area.top_values_skipped.as_deref(),
        Some("too_many_distinct")
    );

    let name = column(&stats, "name");
    assert!(
        !name.cardinality_exact,
        "a large distinct count is an estimate"
    );
    let error = (name.cardinality as f64 - 30_000.0).abs() / 30_000.0;
    assert!(
        error < 0.05,
        "estimate {} is within 5% of 30,000",
        name.cardinality
    );
    assert!(name.top_values.is_empty());
    assert_eq!(
        name.top_values_skipped.as_deref(),
        Some("too_many_distinct")
    );
    assert_eq!(name.null_count, 0);
}

#[test]
fn without_a_recorded_bbox_the_extent_comes_from_the_bbox_columns() {
    let tmp = tempfile::tempdir().expect("tmp");
    let source = tmp.path().join("sites.parquet");
    write_sites(&source, 1_000, 256, Geo::CoveringOnly);
    let stats = compute_now(&source, &SessionContext::new()).expect("statistics");
    let extent = stats.extent.expect("an extent");
    for (got, want) in extent.iter().zip([100.0, -5.49, 100.99, -5.0]) {
        assert!((got - want).abs() < 1e-9, "extent {extent:?}");
    }
}

// ── Flat memory ──────────────────────────────────────────────────────

/// Two million rows with a `name` column whose every value is distinct, on a
/// 2 MiB memory pool: the statistics complete, while an exact distinct count
/// of that column is refused on the same pool. About 15 s in a debug build.
#[test]
fn statistics_of_two_million_rows_fit_a_small_memory_pool() {
    const ROWS: usize = 2_000_000;
    const POOL_BYTES: usize = 2 * 1024 * 1024;
    let tmp = tempfile::tempdir().expect("tmp");
    let source = tmp.path().join("sites.parquet");
    write_sites(&source, ROWS, OPTIMIZED_ROW_GROUP_ROWS, Geo::WithBbox);

    let ctx = small_pool_session(POOL_BYTES);
    let stats = compute_now(&source, &ctx).expect("statistics within a 2 MiB pool");
    assert_eq!(stats.row_count, ROWS as u64);
    let name = column(&stats, "name");
    let error = (name.cardinality as f64 - ROWS as f64).abs() / ROWS as f64;
    assert!(
        error < 0.05,
        "estimate {} is within 5% of {ROWS}",
        name.cardinality
    );
    assert_eq!(
        name.top_values_skipped.as_deref(),
        Some("too_many_distinct")
    );
    assert_eq!(
        column(&stats, "suburb").top_values[0],
        ("north".to_string(), ROWS as u64 / 2)
    );
    let area = column(&stats, "area")
        .histogram
        .as_ref()
        .expect("histogram");
    assert_eq!(
        area.counts.iter().sum::<u64>(),
        ROWS as u64 - column(&stats, "area").null_count
    );

    // The pool is small enough to matter: the exact count is refused on it.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let exact = runtime.block_on(async {
        ctx.sql(r#"SELECT COUNT(DISTINCT "name") FROM layer"#)
            .await?
            .collect()
            .await
    });
    let err = exact.expect_err("an exact distinct count of every name does not fit 2 MiB");
    assert!(
        matches!(
            err.find_root(),
            datafusion::error::DataFusionError::ResourcesExhausted(_)
        ),
        "refused for memory, got: {err}"
    );
}

// ── /stats reads, never computes ─────────────────────────────────────

#[test]
fn stats_are_read_never_computed_on_a_request() {
    let tmp = tempfile::tempdir().expect("tmp");
    let source = tmp.path().join("sites.parquet");
    let stats_file = layer_stats_file(&tmp.path().join("cache"), "default-mapserver", "sites")
        .expect("a stats file");
    write_sites(&source, 500, 128, Geo::WithBbox);
    let layer = geoparquet_layer(&source, &["suburb"]);
    let queue = StatsQueue::new();

    // Nothing saved yet: the answer says so, with the footer's row count,
    // and one computation is queued however often it is asked.
    for _ in 0..3 {
        let first = read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue);
        assert_eq!(first["state"], json!("computing"));
        assert_eq!(first["row_count"], json!(500));
        assert_eq!(first["columns"], json!([]));
        assert_eq!(first["extent"], Value::Null);
        assert!(first.get("computed_at").is_none());
    }
    assert!(!stats_file.exists(), "nothing was computed on the request");
    assert_eq!(queue.waiting().len(), 1);

    assert_eq!(queue.run_waiting(&default_session), 1);
    let saved = saved::read(&stats_file).expect("saved statistics");
    assert!(matches!(saved.outcome, Outcome::Ready(_)));

    let ready = read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue);
    assert_eq!(ready["state"], json!("ready"));
    assert_eq!(ready["computed_at"], json!(saved.computed_at));
    assert_eq!(ready["row_count"], json!(500));
    assert_eq!(ready["extent"], json!([1.0, 2.0, 3.0, 4.0]));
    assert_eq!(
        names(&ready),
        ["suburb"],
        "the public sees only the allowed columns"
    );
    assert!(ready.get("allowed_properties").is_none());
    let suburb = &ready["columns"][0];
    assert_eq!(
        suburb["top_values"],
        json!([["north", 250], ["south", 150], ["east", 50]])
    );
    assert_eq!(suburb["cardinality_exact"], json!(true));
    assert!(
        queue.waiting().is_empty(),
        "a saved result is not queued again"
    );

    let operator = read_with(&layer, StatsAudience::Operator, Some(&stats_file), &queue);
    assert_eq!(names(&operator), ["area", "floors", "name", "suburb"]);
    assert_eq!(operator["allowed_properties"], json!(["suburb"]));

    // A bucket's mirror copy is marked as used by moving its modification
    // time on every read. That is not a new version of the file: the saved
    // statistics stay ready and nothing is queued.
    std::fs::File::options()
        .append(true)
        .open(&source)
        .expect("source")
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))
        .expect("touch");
    let touched = read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue);
    assert_eq!(touched["state"], json!("ready"), "a touched file keeps its statistics");
    assert!(queue.waiting().is_empty(), "a touched file is not queued again");

    // A changed file is computing again until its own computation runs.
    write_sites(&source, 800, 128, Geo::WithBbox);
    let changed = read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue);
    assert_eq!(changed["state"], json!("computing"));
    assert_eq!(changed["row_count"], json!(800));
    assert_eq!(queue.waiting().len(), 1);
    queue.run_waiting(&default_session);
    let fresh = read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue);
    assert_eq!(fresh["state"], json!("ready"));
    assert_eq!(fresh["row_count"], json!(800));
}

#[test]
fn a_failed_computation_stays_failed_until_the_file_changes_or_is_republished() {
    let tmp = tempfile::tempdir().expect("tmp");
    let source = tmp.path().join("sites.parquet");
    let stats_file = tmp.path().join("cache").join("stats.json");
    fs::write(&source, b"not a parquet file").expect("write");
    let layer = geoparquet_layer(&source, &[]);
    let queue = StatsQueue::new();

    let first = read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue);
    assert_eq!(first["state"], json!("computing"));
    assert_eq!(first["row_count"], Value::Null, "the footer does not read");
    queue.run_waiting(&default_session);

    for _ in 0..2 {
        let failed = read_with(&layer, StatsAudience::Operator, Some(&stats_file), &queue);
        assert_eq!(failed["state"], json!("failed"));
        assert!(
            failed["error"]
                .as_str()
                .is_some_and(|e| e.contains("sites.parquet")),
            "{failed}"
        );
        assert_eq!(failed["columns"], json!([]));
        assert!(
            queue.waiting().is_empty(),
            "a failure is not retried on a request"
        );
    }

    // Republishing forces a new computation of the same file.
    queue.enqueue(Job {
        source: source.clone(),
        stats_file: stats_file.clone(),
        force: true,
    });
    assert_eq!(queue.waiting().len(), 1);
    queue.run_waiting(&default_session);
    assert_eq!(
        read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue)["state"],
        json!("failed")
    );

    // A repaired file is computed again.
    write_sites(&source, 10, 10, Geo::WithBbox);
    assert_eq!(
        read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue)["state"],
        json!("computing")
    );
    queue.run_waiting(&default_session);
    assert_eq!(
        read_with(&layer, StatsAudience::Public, Some(&stats_file), &queue)["state"],
        json!("ready")
    );
}

#[test]
fn a_newer_request_replaces_a_waiting_one_and_the_running_one_is_not_queued_twice() {
    let queue = StatsQueue::new();
    let job = |source: &str, force: bool| Job {
        source: Path::new(source).to_path_buf(),
        stats_file: Path::new("cache/sites/stats.json").to_path_buf(),
        force,
    };
    queue.enqueue(job("a.parquet", true));
    queue.enqueue(job("b.parquet", false));
    assert_eq!(
        queue.waiting(),
        vec![job("b.parquet", true)],
        "the newer source, and the republish's force kept"
    );

    let running = queue.take().expect("a job");
    queue.enqueue(job("b.parquet", false));
    assert!(
        queue.waiting().is_empty(),
        "the same computation is already running"
    );
    queue.enqueue(job("b.parquet", true));
    assert_eq!(
        queue.waiting().len(),
        1,
        "a republish during a run is queued after it"
    );
    assert_eq!(running, job("b.parquet", true));
    queue.finish();
}

#[test]
fn the_saved_file_lives_in_the_layers_cache_folder() {
    let home = Path::new("cache/mapserver-artifacts");
    assert_eq!(
        layer_stats_file(home, "default-mapserver", "sites"),
        Some(
            home.join("default-mapserver")
                .join("sites")
                .join("stats.json")
        )
    );
    assert_eq!(layer_stats_file(home, "..", "sites"), None);
    assert_eq!(layer_stats_file(home, "default-mapserver", "a/b"), None);
}

// ── GeoJSON files ────────────────────────────────────────────────────

#[test]
fn public_stats_report_only_the_columns_the_layer_exposes() {
    // Column names, min/max, and top values are source data, so contract
    // `MapPublishManifest` puts them behind `allowed_properties` exactly as
    // it puts feature properties there.
    let tmp = tempfile::tempdir().expect("tmp");

    let named = geojson_layer(tmp.path(), vec!["name".to_string()]);
    let public = read_layer_stats(&named, StatsAudience::Public, None);
    assert_eq!(public["state"], json!("ready"));
    assert_eq!(names(&public), ["name"]);
    assert!(public.get("allowed_properties").is_none());
    assert_eq!(
        public["columns"][0]["top_values"],
        json!([["A", 2], ["B", 1]])
    );
    assert_eq!(public["extent"], json!([106.0, -6.2, 107.0, -6.0]));

    let operator = read_layer_stats(&named, StatsAudience::Operator, None);
    assert_eq!(names(&operator), ["floors", "name", "owner_phone"]);
    assert_eq!(operator["allowed_properties"], json!(["name"]));
    let floors = operator["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .find(|c| c["name"] == "floors")
        .expect("floors");
    assert_eq!(
        floors["null_count"],
        json!(1),
        "a missing property is a null"
    );
    assert_eq!(
        (floors["min"].clone(), floors["max"].clone()),
        (json!(2.0), json!(4.0))
    );
    let counts = floors["histogram"]["counts"].as_array().expect("counts");
    assert_eq!(counts.len(), HISTOGRAM_BINS);
    assert_eq!(
        (counts[0].clone(), counts[HISTOGRAM_BINS - 1].clone()),
        (json!(1), json!(1))
    );

    // An unconfigured layer exposes nothing, the same closed default the
    // feature paths use.
    let closed = geojson_layer(tmp.path(), Vec::new());
    let public = read_layer_stats(&closed, StatsAudience::Public, None);
    assert!(names(&public).is_empty());
    assert_eq!(public["row_count"], json!(3));
}
