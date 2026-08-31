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

/// Who is asking for a layer's column statistics.
///
/// Column names, min/max, and sampled top values are source data. Contract
/// `MapPublishManifest` puts them behind `allowed_properties` exactly as it
/// puts feature properties there, so the public endpoint reports only the
/// columns the operator chose to expose. The operator view is what the publish
/// UI needs to make that choice, and it is reached through an authenticated
/// project route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsAudience {
    /// `/ms/{owner}/{project}/{layer}/stats` — pruned to `allowed_properties`.
    Public,
    /// The project's own publish UI — every column in the source.
    Operator,
}

/// Compute layer statistics and capabilities from a published manifest.
pub fn compute_layer_stats(
    manifest: &PublishedLayerManifest,
    audience: StatsAudience,
) -> Result<Value, String> {
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

    let columns: Vec<ColumnStats> = match audience {
        StatsAudience::Operator => columns,
        StatsAudience::Public => columns
            .into_iter()
            .filter(|column| {
                crate::mapserver::resolve::property_is_public(
                    &manifest.allowed_properties,
                    &column.name,
                )
            })
            .collect(),
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

    let mut out = json!({
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
    });
    if audience == StatsAudience::Operator {
        // The operator must be able to see which columns are exposed today
        // before ticking another one.
        out["allowed_properties"] = json!(manifest.allowed_properties);
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn geojson_layer(dir: &Path, allowed: Vec<String>) -> PublishedLayerManifest {
        let source = dir.join("layer.geojson");
        fs::write(
            &source,
            serde_json::to_string(&json!({
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "properties": { "name": "A", "owner_phone": "+61400000000" },
                        "geometry": { "type": "Point", "coordinates": [106.0, -6.0] }
                    },
                    {
                        "type": "Feature",
                        "properties": { "name": "B", "owner_phone": "+61400000001" },
                        "geometry": { "type": "Point", "coordinates": [107.0, -6.1] }
                    }
                ]
            }))
            .expect("source json"),
        )
        .expect("write source");
        PublishedLayerManifest {
            layer_id: "adm1".to_string(),
            path: "adm1".to_string(),
            source_kind: SourceKind::GeoJsonFile,
            source_ref: source.display().to_string(),
            mode: "features".to_string(),
            min_zoom: None,
            max_zoom: None,
            bbox_required: false,
            max_features: 100,
            allowed_properties: allowed,
            style: None,
            filter: None,
            function_slug: None,
            cache_ttl_secs: None,
        }
    }

    fn column_names(stats: &Value) -> Vec<String> {
        stats["columns"]
            .as_array()
            .expect("columns array")
            .iter()
            .map(|column| column["name"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    #[test]
    fn public_stats_report_only_the_columns_the_layer_exposes() {
        // Column names, min/max, and sampled top values are source data, so
        // contract `MapPublishManifest` puts them behind `allowed_properties`
        // exactly as it puts feature properties there.
        let tmp = tempfile::tempdir().expect("tmp");

        let named = geojson_layer(tmp.path(), vec!["name".to_string()]);
        let public = compute_layer_stats(&named, StatsAudience::Public).expect("public stats");
        assert_eq!(column_names(&public), vec!["name".to_string()]);
        assert!(public.get("allowed_properties").is_none());

        let operator =
            compute_layer_stats(&named, StatsAudience::Operator).expect("operator stats");
        let mut operator_columns = column_names(&operator);
        operator_columns.sort();
        assert_eq!(
            operator_columns,
            vec!["name".to_string(), "owner_phone".to_string()]
        );
        assert_eq!(operator["allowed_properties"], json!(["name"]));

        // An unconfigured layer exposes nothing, the same closed default the
        // feature paths use.
        let closed = geojson_layer(tmp.path(), Vec::new());
        let public = compute_layer_stats(&closed, StatsAudience::Public).expect("public stats");
        assert!(column_names(&public).is_empty());
        assert_eq!(public["row_count"], json!(2));
    }
}
