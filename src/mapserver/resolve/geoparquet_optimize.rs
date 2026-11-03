//! GeoParquet spatial optimizer.
//!
//! Reads a GeoParquet file, adds bbox covering columns (`xmin`, `ymin`, `xmax`, `ymax`),
//! sorts rows by Hilbert curve index for spatial locality, and writes an optimized
//! parquet file with small row groups ([`OPTIMIZED_ROW_GROUP_ROWS`]) and ZSTD compression.
//!
//! After optimization, DataFusion's `pushdown_filters` can prune 90%+ of row groups
//! via column statistics on the bbox columns, dramatically reducing query time for
//! spatial tile queries.

use std::path::Path;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::{DataType, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

/// The most rows in one row group of an optimized copy. DataFusion reads a
/// whole row group of each column it needs into memory before decoding it,
/// outside its memory pool, so this must stay small (at most 65,536) for a
/// large layer to be read with flat memory.
pub const OPTIMIZED_ROW_GROUP_ROWS: usize = 10_000;
const _: () = assert!(OPTIMIZED_ROW_GROUP_ROWS <= 65_536);

/// Statistics reported after optimization.
pub struct OptimizeReport {
    pub rows: usize,
    pub row_groups: usize,
    pub extent: [f64; 4], // xmin, ymin, xmax, ymax
    pub source_bytes: u64,
    pub dest_bytes: u64,
    pub geom_col: String,
    /// Whether rows were sorted along the Hilbert curve. A layer too large
    /// for the sort's memory budget is written in source order instead.
    pub hilbert_sorted: bool,
}

/// A map layer is served in longitude/latitude: a box from a map is compared
/// with these coordinates as they are. A source whose extent leaves ±180° /
/// ±90° is in another unit (metres of a projected CRS such as EPSG:3857) and
/// would answer every box with nothing, so it is refused with the fix.
pub fn require_lon_lat(extent: [f64; 4], source: &str) -> Result<(), String> {
    let [xmin, ymin, xmax, ymax] = extent;
    let finite = extent.iter().all(|v| v.is_finite());
    if !finite || (xmin >= -180.0 && xmax <= 180.0 && ymin >= -90.0 && ymax <= 90.0) {
        return Ok(());
    }
    Err(format!(
        "'{source}' is not in longitude/latitude (its extent is x {xmin:.0}..{xmax:.0}, y {ymin:.0}..{ymax:.0}): \
         a map layer is served in degrees. Reproject it first: \
         geo.dataset.convert --from {source} --crs EPSG:4326, then publish what that writes"
    ))
}

/// Optimize a GeoParquet file for spatial tile queries.
///
/// Delegates spatial optimization (bbox columns, Hilbert sort, ZSTD compression,
/// small row groups) to `geonative_geoparquet::optimize()`, then reads the
/// extent from the optimized output's bbox columns. Column statistics are not
/// computed here: the map server computes them after publishing
/// (`resolve::stats`).
pub fn optimize_geoparquet(source: &Path, dest: &Path) -> Result<OptimizeReport, String> {
    let source_bytes = std::fs::metadata(source).map(|m| m.len()).unwrap_or(0);

    // Core optimization: geonative handles bbox columns, Hilbert sort, ZSTD,
    // row groups. The sort holds every feature in memory up to its budget;
    // a larger layer is written in source order, which keeps the small row
    // groups and the bbox columns with flat memory.
    let (rows, hilbert_sorted) = match geonative_geoparquet::optimize(
        source,
        dest,
        geonative_geoparquet::OptimizeOptions { batch_size: OPTIMIZED_ROW_GROUP_ROWS },
    ) {
        Ok(report) => (report.features as usize, true),
        Err(geonative_geoparquet::GeoParquetError::HilbertBudgetExceeded { .. }) => {
            (rewrite_unsorted(source, dest)?, false)
        }
        Err(e) => return Err(format!("optimize failed: {e}")),
    };

    let (geom_col, extent, row_groups) = read_optimized_layout(dest)?;

    Ok(OptimizeReport {
        rows,
        row_groups,
        extent,
        source_bytes,
        dest_bytes: std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0),
        geom_col,
        hilbert_sorted,
    })
}

/// The optimized copy without the Hilbert sort: bbox columns and row groups
/// of [`OPTIMIZED_ROW_GROUP_ROWS`], streamed feature by feature. Answers the
/// rows written.
pub fn rewrite_unsorted(source: &Path, dest: &Path) -> Result<usize, String> {
    use geonative_geoparquet::{GeoParquetReader, GeoParquetWriter, WriterOptions};
    let reader = GeoParquetReader::open(source).map_err(|e| format!("optimize failed: {e}"))?;
    let schema = reader.schema().clone();
    let file = std::fs::File::create(dest).map_err(|e| format!("optimize failed: {e}"))?;
    let options = WriterOptions {
        batch_size: OPTIMIZED_ROW_GROUP_ROWS,
        add_bbox_columns: true,
        hilbert_sort: false,
        ..WriterOptions::default()
    };
    let mut writer = GeoParquetWriter::create(file, &schema, options).map_err(|e| format!("optimize failed: {e}"))?;
    let mut rows = 0usize;
    for feature in reader.into_features() {
        let feature = feature.map_err(|e| format!("optimize failed: {e}"))?;
        writer.write(&feature).map_err(|e| format!("optimize failed: {e}"))?;
        rows += 1;
    }
    writer.close().map_err(|e| format!("optimize failed: {e}"))?;
    Ok(rows)
}

/// The geometry column, the extent and the row group count of an optimized
/// file. Only the four bbox columns are read.
fn read_optimized_layout(path: &Path) -> Result<(String, [f64; 4], usize), String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("failed to open optimized parquet: {e}"))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("failed to create parquet reader: {e}"))?;
    let schema = builder.schema().clone();
    let geom_idx =
        detect_geom_column_index(&schema).ok_or("no geometry column found in optimized file")?;
    let geom_col = schema.field(geom_idx).name().clone();
    let row_groups = builder.metadata().num_row_groups();
    if builder.metadata().file_metadata().num_rows() == 0 {
        return Err("optimized parquet has no data".to_string());
    }

    let parquet_schema = builder.parquet_schema();
    let bbox_leaves: Vec<usize> = ["xmin", "ymin", "xmax", "ymax"]
        .iter()
        .filter_map(|name| (0..parquet_schema.num_columns()).find(|&i| parquet_schema.column(i).name() == *name))
        .collect();
    let mut extent = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    if bbox_leaves.len() == 4 {
        let projection = ProjectionMask::leaves(parquet_schema, bbox_leaves);
        let reader = builder
            .with_projection(projection)
            .with_batch_size(65_536)
            .build()
            .map_err(|e| format!("failed to build parquet reader: {e}"))?;
        for batch in reader {
            let batch = batch.map_err(|e| format!("failed to read parquet batches: {e}"))?;
            let part = read_extent_from_bbox_columns(&batch);
            extent = [extent[0].min(part[0]), extent[1].min(part[1]), extent[2].max(part[2]), extent[3].max(part[3])];
        }
    }
    Ok((geom_col, extent, row_groups))
}

/// Read the dataset extent from the xmin/ymin/xmax/ymax bbox columns.
fn read_extent_from_bbox_columns(batch: &RecordBatch) -> [f64; 4] {
    let schema = batch.schema();
    let mut extent = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];

    let xmin_idx = schema.column_with_name("xmin").map(|(i, _)| i);
    let ymin_idx = schema.column_with_name("ymin").map(|(i, _)| i);
    let xmax_idx = schema.column_with_name("xmax").map(|(i, _)| i);
    let ymax_idx = schema.column_with_name("ymax").map(|(i, _)| i);

    let (Some(xi), Some(yi), Some(xa), Some(ya)) = (xmin_idx, ymin_idx, xmax_idx, ymax_idx) else {
        return extent;
    };

    let xmin_arr = batch.column(xi).as_any().downcast_ref::<Float64Array>();
    let ymin_arr = batch.column(yi).as_any().downcast_ref::<Float64Array>();
    let xmax_arr = batch.column(xa).as_any().downcast_ref::<Float64Array>();
    let ymax_arr = batch.column(ya).as_any().downcast_ref::<Float64Array>();

    let (Some(xmn), Some(ymn), Some(xmx), Some(ymx)) = (xmin_arr, ymin_arr, xmax_arr, ymax_arr)
    else {
        return extent;
    };

    for i in 0..batch.num_rows() {
        if xmn.is_null(i) {
            continue;
        }
        let x0 = xmn.value(i);
        let y0 = ymn.value(i);
        let x1 = xmx.value(i);
        let y1 = ymx.value(i);
        if x0 < extent[0] {
            extent[0] = x0;
        }
        if y0 < extent[1] {
            extent[1] = y0;
        }
        if x1 > extent[2] {
            extent[2] = x1;
        }
        if y1 > extent[3] {
            extent[3] = y1;
        }
    }
    extent
}

// ── Geometry column detection ─────────────────────────────────────────

/// Column names read as the geometry column, in order of preference.
pub(crate) const GEOM_NAMES: &[&str] = &["geometry", "geom", "wkb_geometry", "the_geom", "shape"];

fn detect_geom_column_index(schema: &Schema) -> Option<usize> {
    // Try known names first
    for name in GEOM_NAMES {
        if let Some((idx, _)) = schema.column_with_name(name) {
            return Some(idx);
        }
    }
    // Fall back to first Binary/LargeBinary column
    for (idx, field) in schema.fields().iter().enumerate() {
        if matches!(field.data_type(), DataType::Binary | DataType::LargeBinary) {
            return Some(idx);
        }
    }
    None
}

// ── GeoJSON geometry → WKB encoder (via geonative) ────────────────────

/// Encode a GeoJSON geometry object to WKB binary.
/// Delegates to geonative_geojson → geonative_core WKB codec.
pub fn geojson_geometry_to_wkb(geometry: &serde_json::Value) -> Option<Vec<u8>> {
    let ir = geonative_geojson::geometry::from_json(geometry).ok()?;
    Some(ir.to_wkb())
}

// ── Already-optimized checker ─────────────────────────────────────────

/// Check if a parquet file already has spatial optimization columns
/// (xmin, ymin, xmax, ymax as Float64), a geometry column, and row groups of
/// at most [`OPTIMIZED_ROW_GROUP_ROWS`] rows.
pub fn is_already_optimized(path: &Path) -> bool {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let schema = builder.schema();

    // Check for all four bbox columns as Float64
    let bbox_names = ["xmin", "ymin", "xmax", "ymax"];
    let has_bbox = bbox_names.iter().all(|name| {
        schema
            .column_with_name(name)
            .map(|(_, f)| matches!(f.data_type(), DataType::Float64))
            .unwrap_or(false)
    });
    if !has_bbox {
        return false;
    }

    // Check for a geometry column
    if detect_geom_column_index(schema.as_ref()).is_none() {
        return false;
    }

    // Bbox columns alone do not make a file servable: every reader (tiles,
    // box queries, statistics) loads a whole row group per column, so a
    // file with larger row groups gets the optimized copy.
    builder
        .metadata()
        .row_groups()
        .iter()
        .all(|group| usize::try_from(group.num_rows()).is_ok_and(|rows| rows <= OPTIMIZED_ROW_GROUP_ROWS))
}

// ── GeoJSON → GeoParquet converter ────────────────────────────────────

/// Report from converting a GeoJSON file to GeoParquet.
pub struct ConvertReport {
    pub feature_count: usize,
    pub column_count: usize,
    pub dest_bytes: u64,
}

/// Convert a GeoJSON FeatureCollection file to a raw GeoParquet file.
///
/// Delegates to `geonative_convert::convert()`. The output is NOT spatially
/// optimized — call `optimize_geoparquet` on the output for bbox columns,
/// Hilbert sort, and small row groups.
pub fn convert_geojson_to_geoparquet(source: &Path, dest: &Path) -> Result<ConvertReport, String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create output directory: {e}"))?;
    }

    let stats =
        geonative_convert::convert(source, dest, geonative_convert::ConvertOptions::default())
            .map_err(|e| format!("convert failed: {e}"))?;

    Ok(ConvertReport {
        feature_count: stats.features as usize,
        column_count: 0,
        dest_bytes: stats.output_bytes,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Longitude/latitude check ──────────────────────────────────────

    #[test]
    fn a_source_in_metres_is_refused_with_the_fix_and_degrees_pass() {
        // An extent in EPSG:3857 metres, then the same kind of box in degrees.
        let metres = require_lon_lat([1_000_000.0, 2_000_000.0, 1_100_000.0, 2_100_000.0], "geo/site-a.parquet");
        let message = metres.expect_err("metres are refused");
        assert!(message.contains("geo.dataset.convert --from geo/site-a.parquet --crs EPSG:4326"), "{message}");
        assert!(require_lon_lat([10.0, 20.0, 11.0, 21.0], "geo/site-a.parquet").is_ok());
        assert!(require_lon_lat([-180.0, -90.0, 180.0, 90.0], "geo/world.parquet").is_ok());
        assert!(require_lon_lat([f64::NAN; 4], "geo/empty.parquet").is_ok(), "an empty source has no extent to judge");
    }

    // ── GeoJSON → WKB round-trip tests (via geonative) ────────────────

    #[test]
    fn geojson_to_wkb_point_roundtrip() {
        let geojson = serde_json::json!({"type": "Point", "coordinates": [144.9, -37.8]});
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = geonative_core::bbox_from_bytes(&wkb).unwrap();
        assert!((bbox[0] - 144.9).abs() < 1e-10);
        assert!((bbox[1] - (-37.8)).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_polygon_roundtrip() {
        let geojson = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = geonative_core::bbox_from_bytes(&wkb).unwrap();
        assert!((bbox[0]).abs() < 1e-10);
        assert!((bbox[1]).abs() < 1e-10);
        assert!((bbox[2] - 10.0).abs() < 1e-10);
        assert!((bbox[3] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_multipolygon_roundtrip() {
        let geojson = serde_json::json!({
            "type": "MultiPolygon",
            "coordinates": [
                [[[0.0, 0.0], [5.0, 0.0], [5.0, 5.0], [0.0, 5.0], [0.0, 0.0]]],
                [[[10.0, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 20.0], [10.0, 10.0]]]
            ]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = geonative_core::bbox_from_bytes(&wkb).unwrap();
        assert!((bbox[0]).abs() < 1e-10);
        assert!((bbox[1]).abs() < 1e-10);
        assert!((bbox[2] - 20.0).abs() < 1e-10);
        assert!((bbox[3] - 20.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_null_returns_none() {
        assert!(geojson_geometry_to_wkb(&serde_json::Value::Null).is_none());
    }

    // ── convert_geojson_to_geoparquet test ─────────────────────────────

    #[test]
    fn convert_geojson_to_geoparquet_basic() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let parquet_path = tmp.path().join("test.parquet");

        std::fs::write(
            &geojson_path,
            r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": {"name": "A", "value": 42},
                    "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
                },
                {
                    "type": "Feature",
                    "properties": {"name": "B", "value": 99},
                    "geometry": {"type": "Point", "coordinates": [145.0, -37.7]}
                }
            ]
        }"#,
        )
        .expect("write geojson");

        let report = convert_geojson_to_geoparquet(&geojson_path, &parquet_path).unwrap();
        assert_eq!(report.feature_count, 2);
        assert!(report.dest_bytes > 0);
        assert!(parquet_path.exists());

        // Verify it's a valid parquet with a geometry column
        let file = std::fs::File::open(&parquet_path).unwrap();
        let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
        let schema = builder.schema();
        assert!(schema.column_with_name("geometry").is_some());
    }

    // ── is_already_optimized test ──────────────────────────────────────

    #[test]
    fn convert_produces_optimizable_parquet() {
        // geonative_convert produces parquet with bbox columns by default,
        // so convert output is detected as already optimized.
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let parquet_path = tmp.path().join("test.parquet");

        std::fs::write(
            &geojson_path,
            r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"name": "A"},
                "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
            }]
        }"#,
        )
        .expect("write geojson");

        convert_geojson_to_geoparquet(&geojson_path, &parquet_path).unwrap();
        // geonative's converter includes bbox columns, so it's detected as optimized
        assert!(is_already_optimized(&parquet_path));
    }

    #[test]
    fn is_already_optimized_true_after_optimize() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let raw_path = tmp.path().join("test.parquet");
        let opt_path = tmp.path().join("test.spatial.parquet");

        std::fs::write(
            &geojson_path,
            r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"name": "A"},
                "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
            }]
        }"#,
        )
        .expect("write geojson");

        convert_geojson_to_geoparquet(&geojson_path, &raw_path).unwrap();
        optimize_geoparquet(&raw_path, &opt_path).unwrap();
        assert!(is_already_optimized(&opt_path));
    }

    /// `n` invented points in a small box near 100E 5S.
    fn write_points_geojson(path: &Path, n: usize) {
        let features: Vec<String> = (0..n)
            .map(|i| {
                let (x, y) = (100.0 + (i % 100) as f64 * 0.01, -5.0 - (i % 50) as f64 * 0.01);
                format!(r#"{{"type":"Feature","properties":{{"n":{i}}},"geometry":{{"type":"Point","coordinates":[{x},{y}]}}}}"#)
            })
            .collect();
        std::fs::write(path, format!(r#"{{"type":"FeatureCollection","features":[{}]}}"#, features.join(",")))
            .expect("write geojson");
    }

    fn row_groups(path: &Path) -> Vec<usize> {
        let builder = ParquetRecordBatchReaderBuilder::try_new(std::fs::File::open(path).unwrap()).unwrap();
        builder.metadata().row_groups().iter().map(|g| g.num_rows() as usize).collect()
    }

    #[test]
    fn bbox_columns_with_large_row_groups_are_not_already_optimized() {
        // A file made elsewhere can carry the bbox columns in one huge row
        // group; every reader would load that group whole, so it is
        // optimized like any other source.
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("points.geojson");
        let small_groups = tmp.path().join("points.parquet");
        let one_group = tmp.path().join("one-group.parquet");
        let rewritten = tmp.path().join("rewritten.parquet");
        write_points_geojson(&geojson_path, 25_000);
        convert_geojson_to_geoparquet(&geojson_path, &small_groups).unwrap();

        let builder = ParquetRecordBatchReaderBuilder::try_new(std::fs::File::open(&small_groups).unwrap()).unwrap();
        let schema = builder.schema().clone();
        let properties = parquet::file::properties::WriterProperties::builder().set_max_row_group_row_count(Some(50_000)).build();
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(std::fs::File::create(&one_group).unwrap(), schema, Some(properties)).unwrap();
        for batch in builder.build().unwrap() {
            writer.write(&batch.unwrap()).unwrap();
        }
        writer.close().unwrap();

        assert!(is_already_optimized(&small_groups), "bbox columns in small row groups");
        assert_eq!(row_groups(&one_group), vec![25_000]);
        assert!(!is_already_optimized(&one_group), "one row group of 25,000 rows");

        // The copy written without the Hilbert sort, for a layer past the
        // sort's memory budget, has the small row groups and bbox columns.
        assert_eq!(rewrite_unsorted(&one_group, &rewritten).unwrap(), 25_000);
        assert_eq!(row_groups(&rewritten), vec![OPTIMIZED_ROW_GROUP_ROWS, OPTIMIZED_ROW_GROUP_ROWS, 5_000]);
        assert!(is_already_optimized(&rewritten));
    }

    #[test]
    fn an_optimized_copy_is_written_in_small_row_groups() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("points.geojson");
        let raw_path = tmp.path().join("points.parquet");
        let opt_path = tmp.path().join("points.spatial.parquet");
        write_points_geojson(&geojson_path, 25_000);

        convert_geojson_to_geoparquet(&geojson_path, &raw_path).unwrap();
        let report = optimize_geoparquet(&raw_path, &opt_path).unwrap();
        let file = std::fs::File::open(&opt_path).unwrap();
        let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
        let groups: Vec<usize> = builder.metadata().row_groups().iter().map(|g| g.num_rows() as usize).collect();
        assert_eq!((report.rows, report.row_groups), (25_000, groups.len()));
        assert_eq!(groups, vec![OPTIMIZED_ROW_GROUP_ROWS, OPTIMIZED_ROW_GROUP_ROWS, 5_000]);
        for (got, want) in report.extent.iter().zip([100.0, -5.49, 100.99, -5.0]) {
            assert!((got - want).abs() < 1e-9, "extent {:?}", report.extent);
        }
    }
}
