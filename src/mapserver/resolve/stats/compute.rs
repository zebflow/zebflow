//! The statistics of one GeoParquet file, computed with DataFusion.
//!
//! Memory stays flat whatever the file's size:
//!
//! - one scan answers the row count, every column's null count, min and max,
//!   and an estimate of its distinct values (`approx_distinct`, a HyperLogLog
//!   sketch of fixed size);
//! - top values are counted one column at a time, and only for a column whose
//!   estimate is at most [`TOP_VALUES_MAX_DISTINCT`], so no grouping holds
//!   more entries than that;
//! - a histogram is one grouped query of [`HISTOGRAM_BINS`] groups per
//!   numeric column.
//!
//! Exact: the row count, null counts, min and max, top value counts,
//! histogram counts, and the distinct count of a column whose top values were
//! counted. Estimated: the distinct count of any other column.

use std::path::Path;

use datafusion::arrow::datatypes::{DataType, SchemaRef};
use datafusion::common::ScalarValue;
use datafusion::functions_aggregate::expr_fn::{approx_distinct, count, max, min};
use datafusion::prelude::{
    Expr, ParquetReadOptions, SessionContext, cast, floor, ident, lit, when,
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::Value;

use super::saved::{ColumnStats, Histogram, LayerStats};

/// Top values are counted only for a column with at most this many distinct
/// values (by the estimate). Past it they are skipped as `too_many_distinct`.
pub const TOP_VALUES_MAX_DISTINCT: u64 = 10_000;
/// How many top values a column reports.
pub const TOP_VALUES: usize = 10;
/// How many equal-width bins a numeric column's histogram has.
pub const HISTOGRAM_BINS: usize = 20;
/// The most groups a top-values query may return. The estimate is within a
/// few percent; this only guards against one that is far off.
const MAX_GROUPS: usize = 2 * TOP_VALUES_MAX_DISTINCT as usize;

const TABLE: &str = "layer";

/// Bounding box columns (GeoParquet "covering" columns), in the spellings the
/// map server reads. They are the extent's source, not attributes.
const BBOX_COLUMNS: [[&str; 4]; 3] = [
    ["xmin", "ymin", "xmax", "ymax"],
    ["min_x", "min_y", "max_x", "max_y"],
    ["bbox_xmin", "bbox_ymin", "bbox_xmax", "bbox_ymax"],
];

/// Computes the statistics of the GeoParquet file at `source` on `ctx`.
pub async fn compute(source: &Path, ctx: &SessionContext) -> Result<LayerStats, String> {
    let footer = Footer::read(source)?;
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    ctx.register_parquet(
        TABLE,
        source.to_string_lossy().as_ref(),
        ParquetReadOptions {
            file_extension: &extension,
            ..Default::default()
        },
    )
    .await
    .map_err(|e| format!("read '{}': {e}", file_name(source)))?;

    let columns = footer.columns();
    let extent_columns = footer.extent_columns();
    let scan = scan(ctx, &columns, extent_columns.as_ref()).await?;
    let row_count = scan
        .get("rows")
        .and_then(as_u64)
        .unwrap_or(footer.row_count);

    let mut out = Vec::with_capacity(columns.len());
    for (i, column) in columns.iter().enumerate() {
        let non_null = scan.get(&format!("n{i}")).and_then(as_u64).unwrap_or(0);
        let mut stats = ColumnStats {
            name: column.name.clone(),
            data_type: column.data_type.clone(),
            null_count: row_count.saturating_sub(non_null),
            cardinality: 0,
            cardinality_exact: false,
            min: None,
            max: None,
            top_values: Vec::new(),
            top_values_skipped: None,
            histogram: None,
        };
        if column.kind == Kind::Unsupported {
            stats.top_values_skipped = Some("unsupported_type".to_string());
            out.push(stats);
            continue;
        }
        if non_null == 0 {
            // Every value is null: nothing to count, and the count is exact.
            stats.cardinality_exact = true;
            out.push(stats);
            continue;
        }
        let estimate = scan.get(&format!("d{i}")).and_then(as_u64).unwrap_or(0);
        stats.cardinality = estimate;
        if estimate <= TOP_VALUES_MAX_DISTINCT {
            match top_values(ctx, &column.name).await? {
                Some((distinct, top)) => {
                    stats.cardinality = distinct;
                    stats.cardinality_exact = true;
                    stats.top_values = top;
                }
                None => stats.top_values_skipped = Some("too_many_distinct".to_string()),
            }
        } else {
            stats.top_values_skipped = Some("too_many_distinct".to_string());
        }
        if column.kind.is_numeric() {
            stats.min = scan.get(&format!("lo{i}")).and_then(as_f64);
            stats.max = scan.get(&format!("hi{i}")).and_then(as_f64);
            if let (Some(lo), Some(hi)) = (stats.min, stats.max) {
                stats.histogram = Some(histogram(ctx, column, lo, hi).await?);
            }
        }
        out.push(stats);
    }

    let extent = footer.geo.bbox.or_else(|| {
        let corners = ["ext_xmin", "ext_ymin", "ext_xmax", "ext_ymax"]
            .map(|key| scan.get(key).and_then(as_f64));
        match corners {
            [Some(a), Some(b), Some(c), Some(d)] => Some([a, b, c, d]),
            _ => None,
        }
    });
    Ok(LayerStats {
        row_count,
        extent,
        columns: out,
    })
}

/// The one scan over every attribute column (and the bbox columns when the
/// extent must come from them), answered as one row of named values.
async fn scan(
    ctx: &SessionContext,
    columns: &[Column],
    extent_columns: Option<&[String; 4]>,
) -> Result<std::collections::HashMap<String, ScalarValue>, String> {
    let mut aggregates = vec![count(lit(1i64)).alias("rows")];
    for (i, column) in columns.iter().enumerate() {
        aggregates.push(count(ident(&column.name)).alias(format!("n{i}")));
        if column.kind == Kind::Unsupported {
            continue;
        }
        aggregates.push(approx_distinct(distinct_input(column)).alias(format!("d{i}")));
        if column.kind.is_numeric() {
            let value = finite_value(column)?;
            aggregates.push(min(value.clone()).alias(format!("lo{i}")));
            aggregates.push(max(value).alias(format!("hi{i}")));
        }
    }
    if let Some([xmin, ymin, xmax, ymax]) = extent_columns {
        aggregates.push(min(cast(ident(xmin), DataType::Float64)).alias("ext_xmin"));
        aggregates.push(min(cast(ident(ymin), DataType::Float64)).alias("ext_ymin"));
        aggregates.push(max(cast(ident(xmax), DataType::Float64)).alias("ext_xmax"));
        aggregates.push(max(cast(ident(ymax), DataType::Float64)).alias("ext_ymax"));
    }
    let batches = ctx
        .table(TABLE)
        .await
        .and_then(|df| df.aggregate(vec![], aggregates))
        .map_err(|e| format!("plan the scan: {e}"))?
        .collect()
        .await
        .map_err(|e| format!("scan: {e}"))?;
    let mut values = std::collections::HashMap::new();
    if let Some(batch) = batches.iter().find(|b| b.num_rows() > 0) {
        for (field, array) in batch.schema().fields().iter().zip(batch.columns()) {
            if let Ok(value) = ScalarValue::try_from_array(array, 0) {
                values.insert(field.name().clone(), value);
            }
        }
    }
    Ok(values)
}

/// Every distinct non-null value of one column with its count, when there are
/// at most [`MAX_GROUPS`]: the exact distinct count and the [`TOP_VALUES`]
/// most frequent, most frequent first (ties by value).
async fn top_values(
    ctx: &SessionContext,
    name: &str,
) -> Result<Option<(u64, Vec<(String, u64)>)>, String> {
    let batches = ctx
        .table(TABLE)
        .await
        .and_then(|df| df.filter(ident(name).is_not_null()))
        .and_then(|df| {
            df.aggregate(
                vec![ident(name).alias("v")],
                vec![count(lit(1i64)).alias("n")],
            )
        })
        .and_then(|df| {
            df.select(vec![
                cast(ident("v"), DataType::Utf8).alias("value"),
                ident("n"),
            ])
        })
        .and_then(|df| df.limit(0, Some(MAX_GROUPS + 1)))
        .map_err(|e| format!("plan top values of '{name}': {e}"))?
        .collect()
        .await
        .map_err(|e| format!("top values of '{name}': {e}"))?;
    let mut groups: Vec<(String, u64)> = Vec::new();
    for batch in &batches {
        for row in 0..batch.num_rows() {
            let value = match ScalarValue::try_from_array(batch.column(0), row) {
                Ok(ScalarValue::Utf8(Some(s))) => s,
                _ => continue,
            };
            let n = ScalarValue::try_from_array(batch.column(1), row)
                .ok()
                .as_ref()
                .and_then(as_u64)
                .unwrap_or(0);
            groups.push((value, n));
        }
    }
    if groups.len() > MAX_GROUPS {
        return Ok(None);
    }
    let distinct = groups.len() as u64;
    groups.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    groups.truncate(TOP_VALUES);
    Ok(Some((distinct, groups)))
}

/// [`HISTOGRAM_BINS`] equal-width bins of a numeric column's finite values
/// from `lo` to `hi`.
async fn histogram(
    ctx: &SessionContext,
    column: &Column,
    lo: f64,
    hi: f64,
) -> Result<Histogram, String> {
    let value = finite_value(column)?;
    let width = (hi - lo) / HISTOGRAM_BINS as f64;
    let bin = if width > 0.0 && width.is_finite() {
        cast(
            floor((value.clone() - lit(lo)) / lit(width)),
            DataType::Int64,
        )
    } else {
        lit(0i64)
    };
    let name = &column.name;
    let batches = ctx
        .table(TABLE)
        .await
        .and_then(|df| df.filter(value.is_not_null()))
        .and_then(|df| df.aggregate(vec![bin.alias("bin")], vec![count(lit(1i64)).alias("n")]))
        .map_err(|e| format!("plan the histogram of '{name}': {e}"))?
        .collect()
        .await
        .map_err(|e| format!("histogram of '{name}': {e}"))?;
    let mut counts = vec![0u64; HISTOGRAM_BINS];
    for batch in &batches {
        for row in 0..batch.num_rows() {
            let bin = ScalarValue::try_from_array(batch.column(0), row).ok();
            let n = ScalarValue::try_from_array(batch.column(1), row).ok();
            let (Some(ScalarValue::Int64(Some(bin))), Some(n)) = (bin, n.as_ref().and_then(as_u64))
            else {
                continue;
            };
            // A value at `hi`, or one rounding lands past the last edge,
            // belongs to the last bin.
            let bin = usize::try_from(bin.max(0))
                .unwrap_or(0)
                .min(HISTOGRAM_BINS - 1);
            counts[bin] += n;
        }
    }
    Ok(Histogram {
        min: lo,
        max: hi,
        counts,
    })
}

// ── Columns ──────────────────────────────────────────────────────────

/// How a column's values can be counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Integer,
    Float,
    Decimal,
    /// Counted as they are: text, dates and times.
    Plain,
    /// Counted through their text form: booleans, dictionaries, nulls.
    AsText,
    /// Nested values (lists, structs, maps): null count only.
    Unsupported,
}

impl Kind {
    fn of(data_type: &DataType) -> Self {
        use DataType::*;
        match data_type {
            Int8 | Int16 | Int32 | Int64 | UInt8 | UInt16 | UInt32 | UInt64 => Self::Integer,
            Float16 | Float32 | Float64 => Self::Float,
            Decimal32(..) | Decimal64(..) | Decimal128(..) | Decimal256(..) => Self::Decimal,
            Utf8 | LargeUtf8 | Utf8View | Date32 | Date64 | Time32(_) | Time64(_)
            | Timestamp(..) => Self::Plain,
            Boolean | Null => Self::AsText,
            Dictionary(_, value)
                if matches!(
                    Self::of(value),
                    Self::Integer | Self::Float | Self::Decimal | Self::Plain
                ) =>
            {
                Self::AsText
            }
            _ => Self::Unsupported,
        }
    }

    fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Float | Self::Decimal)
    }
}

/// An attribute column: every column but the geometry and the bbox columns.
struct Column {
    name: String,
    data_type: String,
    kind: Kind,
}

/// What `approx_distinct` counts for a column. It takes integers, text, dates
/// and times as they are; the rest are counted through their text form.
fn distinct_input(column: &Column) -> Expr {
    match column.kind {
        Kind::Integer | Kind::Plain => ident(&column.name),
        _ => cast(ident(&column.name), DataType::Utf8),
    }
}

/// A numeric column's value as a float, null when it is not finite (NaN or
/// infinite), so min, max and the histogram describe real numbers.
fn finite_value(column: &Column) -> Result<Expr, String> {
    let value = cast(ident(&column.name), DataType::Float64);
    if column.kind != Kind::Float {
        return Ok(value);
    }
    when(value.clone().between(lit(-f64::MAX), lit(f64::MAX)), value)
        .end()
        .map_err(|e| format!("finite values of '{}': {e}", column.name))
}

// ── The footer ───────────────────────────────────────────────────────

/// What the file's footer says, read without reading any data.
struct Footer {
    schema: SchemaRef,
    row_count: u64,
    geo: GeoMetadata,
}

/// The parts of the GeoParquet `geo` metadata the statistics use.
#[derive(Default)]
struct GeoMetadata {
    geometry_columns: Vec<String>,
    /// The primary column's `bbox`, when the writer recorded one.
    bbox: Option<[f64; 4]>,
    /// The primary column's bbox covering columns, when they are plain columns.
    covering: Option<[String; 4]>,
}

impl Footer {
    fn read(path: &Path) -> Result<Self, String> {
        let file =
            std::fs::File::open(path).map_err(|e| format!("open '{}': {e}", file_name(path)))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| format!("read the footer of '{}': {e}", file_name(path)))?;
        let meta = builder.metadata().file_metadata();
        let geo = meta
            .key_value_metadata()
            .and_then(|pairs| pairs.iter().find(|kv| kv.key == "geo"))
            .and_then(|kv| kv.value.as_deref())
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .map(|geo| GeoMetadata::from_json(&geo))
            .unwrap_or_default();
        Ok(Self {
            schema: builder.schema().clone(),
            row_count: u64::try_from(meta.num_rows()).unwrap_or(0),
            geo,
        })
    }

    /// The attribute columns: not geometry, not a bbox column, not binary.
    fn columns(&self) -> Vec<Column> {
        let geometry_names = crate::mapserver::resolve::geoparquet_optimize::GEOM_NAMES;
        let covering: &[String] = self
            .geo
            .covering
            .as_ref()
            .map(|c| c.as_slice())
            .unwrap_or(&[]);
        self.schema
            .fields()
            .iter()
            .filter(|field| {
                let name = field.name().as_str();
                !geometry_names.contains(&name)
                    && !self.geo.geometry_columns.iter().any(|g| g == name)
                    && !BBOX_COLUMNS.iter().flatten().any(|b| *b == name)
                    && !covering.iter().any(|c| c == name)
                    && !(name == "bbox" && matches!(field.data_type(), DataType::Struct(_)))
                    && !matches!(
                        field.data_type(),
                        DataType::Binary
                            | DataType::LargeBinary
                            | DataType::BinaryView
                            | DataType::FixedSizeBinary(_)
                    )
            })
            .map(|field| Column {
                name: field.name().clone(),
                data_type: format!("{:?}", field.data_type()),
                kind: Kind::of(field.data_type()),
            })
            .collect()
    }

    /// The bbox columns the extent is read from, when the metadata carries no
    /// `bbox` of its own: the declared covering, else a known spelling.
    fn extent_columns(&self) -> Option<[String; 4]> {
        if self.geo.bbox.is_some() {
            return None;
        }
        let is_number = |name: &str| {
            self.schema
                .field_with_name(name)
                .is_ok_and(|f| matches!(f.data_type(), DataType::Float64 | DataType::Float32))
        };
        if let Some(covering) = self
            .geo
            .covering
            .as_ref()
            .filter(|c| c.iter().all(|n| is_number(n)))
        {
            return Some(covering.clone());
        }
        BBOX_COLUMNS
            .iter()
            .find(|names| names.iter().all(|n| is_number(n)))
            .map(|names| names.map(str::to_string))
    }
}

impl GeoMetadata {
    fn from_json(geo: &Value) -> Self {
        let Some(columns) = geo.get("columns").and_then(Value::as_object) else {
            return Self::default();
        };
        let primary = geo
            .get("primary_column")
            .and_then(Value::as_str)
            .or_else(|| columns.keys().next().map(String::as_str));
        let column = primary.and_then(|p| columns.get(p));
        let bbox = column
            .and_then(|c| c.get("bbox"))
            .and_then(Value::as_array)
            .and_then(|b| {
                let n: Vec<f64> = b.iter().filter_map(Value::as_f64).collect();
                let corners = match n.as_slice() {
                    [a, b, c, d] => [*a, *b, *c, *d],
                    // A 3D bbox: x, y, z, then x, y, z.
                    [a, b, _, d, e, _] => [*a, *b, *d, *e],
                    _ => return None,
                };
                corners.iter().all(|v| v.is_finite()).then_some(corners)
            });
        let covering = column
            .and_then(|c| c.pointer("/covering/bbox"))
            .and_then(|b| {
                let name = |key: &str| match b.get(key).and_then(Value::as_array).map(Vec::as_slice)
                {
                    Some([Value::String(name)]) => Some(name.clone()),
                    _ => None,
                };
                Some([name("xmin")?, name("ymin")?, name("xmax")?, name("ymax")?])
            });
        Self {
            geometry_columns: columns.keys().cloned().collect(),
            bbox,
            covering,
        }
    }
}

// ── Values ───────────────────────────────────────────────────────────

fn as_u64(value: &ScalarValue) -> Option<u64> {
    match value {
        ScalarValue::Int64(Some(v)) => u64::try_from(*v).ok(),
        ScalarValue::UInt64(Some(v)) => Some(*v),
        _ => None,
    }
}

fn as_f64(value: &ScalarValue) -> Option<f64> {
    match value {
        ScalarValue::Float64(Some(v)) if v.is_finite() => Some(*v),
        _ => None,
    }
}

/// A file's name without its folders, for messages a caller may see.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
