//! Layer statistics endpoint.
//!
//! Computes per-column statistics for a published mapserver layer.
//! GeoParquet: reads parquet footer metadata (zero data scan for row_count + min/max).
//! GeoJSON: scans features in memory (skipped for files >50MB).

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};
use crate::mapserver::resolve::geoparquet_optimize::ColumnStats;

/// Compute layer statistics and capabilities from a published manifest.
pub fn compute_layer_stats(manifest: &PublishedLayerManifest) -> Result<Value, String> {
    let (row_count, columns) = match manifest.source_kind {
        SourceKind::GeoParquet => stats_from_geoparquet(Path::new(&manifest.source_ref))?,
        SourceKind::GeoJsonFile => stats_from_geojson_file(Path::new(&manifest.source_ref))?,
        SourceKind::GeoJsonArtifact => {
            // Artifact manifests don't have a single file to scan — return empty stats
            (0, Vec::new())
        }
        SourceKind::GeoJsonFunction => {
            // Function-backed layers have no static source — return empty stats
            (0, Vec::new())
        }
    };

    let columns_json: Vec<Value> = columns
        .into_iter()
        .map(|cs| {
            let mut obj = json!({
                "name": cs.name,
                "data_type": cs.data_type,
                "null_count": cs.null_count,
                "cardinality": cs.cardinality,
            });
            if let Some(min) = cs.min {
                obj["min"] = json!(min);
            }
            if let Some(max) = cs.max {
                obj["max"] = json!(max);
            }
            if !cs.top_values.is_empty() {
                obj["top_values"] = json!(cs.top_values);
            }
            obj
        })
        .collect();

    let source_kind_str = match manifest.source_kind {
        SourceKind::GeoParquet => "geoparquet",
        SourceKind::GeoJsonFile => "geojson",
        SourceKind::GeoJsonArtifact => "geojson_artifact",
        SourceKind::GeoJsonFunction => "geojson_function",
    };

    Ok(json!({
        "layer_id": manifest.layer_id,
        "source_kind": source_kind_str,
        "row_count": row_count,
        "columns": columns_json,
        "capabilities": {
            "mvt": "experimental",
            "png": true,
            "geojson": true,
            "zxy_tiles": true,
            "point_query": true
        }
    }))
}

/// Read statistics from parquet footer metadata — zero data scan.
fn stats_from_geoparquet(source_path: &Path) -> Result<(usize, Vec<ColumnStats>), String> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let file =
        std::fs::File::open(source_path).map_err(|e| format!("failed to open parquet: {e}"))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("failed to read parquet metadata: {e}"))?;

    let metadata = builder.metadata().clone();
    let schema = builder.schema().clone();
    let parquet_schema = builder.parquet_schema().clone();

    // Total row count from metadata
    let row_count: usize = (0..metadata.num_row_groups())
        .map(|i| metadata.row_group(i).num_rows() as usize)
        .sum();

    // Skip geometry and bbox columns
    let geom_names: &[&str] = &["geometry", "geom", "wkb_geometry", "the_geom", "shape"];
    let bbox_names: &[&str] = &[
        "xmin", "ymin", "xmax", "ymax", "min_x", "min_y", "max_x", "max_y",
    ];

    let mut columns = Vec::new();

    for (_col_idx, field) in schema.fields().iter().enumerate() {
        let name = field.name().as_str();
        if geom_names.contains(&name) || bbox_names.contains(&name) {
            continue;
        }
        use datafusion::arrow::datatypes::DataType;
        if matches!(field.data_type(), DataType::Binary | DataType::LargeBinary) {
            continue;
        }

        let data_type = format!("{:?}", field.data_type());

        // Find matching parquet leaf column
        let pq_col_idx =
            (0..parquet_schema.num_columns()).find(|&i| parquet_schema.column(i).name() == name);

        let mut null_count: usize = 0;
        let mut min_val: Option<f64> = None;
        let mut max_val: Option<f64> = None;

        if let Some(pq_idx) = pq_col_idx {
            for rg_idx in 0..metadata.num_row_groups() {
                let rg = metadata.row_group(rg_idx);
                if pq_idx >= rg.num_columns() {
                    continue;
                }
                let col_meta = rg.column(pq_idx);
                // Null count
                if let Some(stats) = col_meta.statistics() {
                    null_count += stats.null_count_opt().unwrap_or(0) as usize;
                    use parquet::file::statistics::Statistics;
                    match stats {
                        Statistics::Double(s) => {
                            if let (Some(&mn), Some(&mx)) = (s.min_opt(), s.max_opt()) {
                                min_val = Some(min_val.map_or(mn, |m: f64| m.min(mn)));
                                max_val = Some(max_val.map_or(mx, |m: f64| m.max(mx)));
                            }
                        }
                        Statistics::Int64(s) => {
                            if let (Some(&mn), Some(&mx)) = (s.min_opt(), s.max_opt()) {
                                min_val =
                                    Some(min_val.map_or(mn as f64, |m: f64| m.min(mn as f64)));
                                max_val =
                                    Some(max_val.map_or(mx as f64, |m: f64| m.max(mx as f64)));
                            }
                        }
                        Statistics::Int32(s) => {
                            if let (Some(&mn), Some(&mx)) = (s.min_opt(), s.max_opt()) {
                                min_val =
                                    Some(min_val.map_or(mn as f64, |m: f64| m.min(mn as f64)));
                                max_val =
                                    Some(max_val.map_or(mx as f64, |m: f64| m.max(mx as f64)));
                            }
                        }
                        Statistics::Float(s) => {
                            if let (Some(&mn), Some(&mx)) = (s.min_opt(), s.max_opt()) {
                                min_val =
                                    Some(min_val.map_or(mn as f64, |m: f64| m.min(mn as f64)));
                                max_val =
                                    Some(max_val.map_or(mx as f64, |m: f64| m.max(mx as f64)));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Cardinality: sample via get_field_distinct for string/int columns
        let cardinality = crate::mapserver::resolve::geoparquet_direct::get_field_distinct(
            source_path,
            name,
            1000,
        )
        .map(|d| d.values.len())
        .unwrap_or(0);

        let top_values = if cardinality > 0 && cardinality <= 500 {
            crate::mapserver::resolve::geoparquet_direct::get_field_distinct(source_path, name, 50)
                .map(|d| {
                    d.values
                        .into_iter()
                        .map(|v| (v, 0usize)) // count not tracked in distinct
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        columns.push(ColumnStats {
            name: name.to_string(),
            data_type,
            null_count,
            cardinality,
            min: min_val,
            max: max_val,
            top_values,
        });
    }

    Ok((row_count, columns))
}

/// Scan a GeoJSON file for stats. Skips files >50MB.
fn stats_from_geojson_file(source_path: &Path) -> Result<(usize, Vec<ColumnStats>), String> {
    const MAX_SCAN_SIZE: u64 = 50 * 1024 * 1024;

    let file_size = std::fs::metadata(source_path).map(|m| m.len()).unwrap_or(0);
    if file_size > MAX_SCAN_SIZE {
        return Ok((0, Vec::new()));
    }

    let mut features: Vec<Value> = Vec::new();
    crate::mapserver::infra::geojson_stream::stream_feature_collection_from_path(
        source_path,
        |feature| {
            features.push(feature);
            Ok(())
        },
    )?;

    let row_count = features.len();
    if row_count == 0 {
        return Ok((0, Vec::new()));
    }

    // Scan all features to collect property stats
    let mut col_order: Vec<String> = Vec::new();
    let mut col_seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    struct ColAccum {
        data_type: String,
        null_count: usize,
        min_val: Option<f64>,
        max_val: Option<f64>,
        distinct: HashMap<String, usize>,
        cardinality_capped: bool,
    }

    let mut accum: HashMap<String, ColAccum> = HashMap::new();

    for feature in &features {
        let Some(props) = feature.get("properties").and_then(|v| v.as_object()) else {
            continue;
        };
        for (key, val) in props {
            if !col_seen.contains(key) {
                col_order.push(key.clone());
                col_seen.insert(key.clone());
                accum.insert(
                    key.clone(),
                    ColAccum {
                        data_type: String::new(),
                        null_count: 0,
                        min_val: None,
                        max_val: None,
                        distinct: HashMap::new(),
                        cardinality_capped: false,
                    },
                );
            }
            let acc = accum.get_mut(key).unwrap();

            if val.is_null() {
                acc.null_count += 1;
                continue;
            }

            // Infer type
            let dt = if val.is_boolean() {
                "Boolean"
            } else if val.is_i64() || val.is_u64() {
                "Int64"
            } else if val.is_f64() {
                "Float64"
            } else {
                "Utf8"
            };
            if acc.data_type.is_empty() {
                acc.data_type = dt.to_string();
            } else if acc.data_type != dt {
                // Promote to most general
                if (acc.data_type == "Int64" && dt == "Float64")
                    || (acc.data_type == "Float64" && dt == "Int64")
                {
                    acc.data_type = "Float64".to_string();
                } else {
                    acc.data_type = "Utf8".to_string();
                }
            }

            // Numeric min/max
            if let Some(n) = val.as_f64() {
                if n.is_finite() {
                    acc.min_val = Some(acc.min_val.map_or(n, |m| m.min(n)));
                    acc.max_val = Some(acc.max_val.map_or(n, |m| m.max(n)));
                }
            }

            // Distinct tracking
            if !acc.cardinality_capped {
                let key_str = if let Some(s) = val.as_str() {
                    s.to_string()
                } else {
                    val.to_string()
                };
                let entry = acc.distinct.entry(key_str).or_insert(0);
                *entry += 1;
                if acc.distinct.len() > 1000 {
                    acc.cardinality_capped = true;
                }
            }
        }
    }

    let columns: Vec<ColumnStats> = col_order
        .into_iter()
        .filter_map(|name| {
            let acc = accum.remove(&name)?;
            let cardinality = acc.distinct.len();
            let top_values = if cardinality <= 500 && !acc.cardinality_capped {
                let mut pairs: Vec<(String, usize)> = acc.distinct.into_iter().collect();
                pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                pairs.truncate(50);
                pairs
            } else {
                Vec::new()
            };
            Some(ColumnStats {
                name,
                data_type: if acc.data_type.is_empty() {
                    "Null".to_string()
                } else {
                    acc.data_type
                },
                null_count: acc.null_count,
                cardinality,
                min: acc.min_val,
                max: acc.max_val,
                top_values,
            })
        })
        .collect();

    Ok((row_count, columns))
}
