//! Layer statistics: what `/stats` answers for a published layer.
//!
//! A GeoParquet layer's statistics are computed once, in the background, when
//! the layer is published or its file changes ([`queue`], [`compute`]), and
//! saved as a small JSON file in the cache tier ([`saved`]). `/stats` only
//! reads that file. Until it exists, or while it describes an older version
//! of the file, the answer is `"state": "computing"` and a computation is
//! queued; a computation that failed answers `"failed"` until the file
//! changes or the layer is published again.
//!
//! A GeoJSON file layer is small, so its statistics are computed when asked
//! ([`geojson`]).

mod compute;
mod geojson;
mod queue;
mod saved;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};

pub use compute::{HISTOGRAM_BINS, TOP_VALUES, TOP_VALUES_MAX_DISTINCT};
pub use saved::{ColumnStats, Histogram, LayerStats};

use queue::{Job, StatsQueue};
use saved::{Fingerprint, Outcome};

/// Who is asking for a layer's column statistics.
///
/// Column names, min/max, and top values are source data. Contract
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

/// Where a layer's saved statistics live:
/// `{artifacts home}/{instance}/{layer}/stats.json`, the cache-tier folder a
/// layer's delete removes. `None` unless both names are plain slugs.
pub fn layer_stats_file(artifacts_home: &Path, instance: &str, layer_id: &str) -> Option<PathBuf> {
    let valid = crate::contracts::kinds::valid_layer_id;
    (valid(instance) && valid(layer_id)).then(|| {
        artifacts_home
            .join(instance)
            .join(layer_id)
            .join(saved::STATS_FILE_NAME)
    })
}

/// Queues the statistics of the GeoParquet file at `source`, saved at
/// `stats_file`. Publishing calls this: the computation runs even when a
/// saved result for the same version of the file exists.
pub fn queue_layer_stats(source: &Path, stats_file: &Path) {
    queue::global().enqueue(Job {
        source: source.to_path_buf(),
        stats_file: stats_file.to_path_buf(),
        force: true,
    });
}

/// A layer's statistics as `/stats` answers them. A GeoParquet layer's are
/// read from `stats_file`, never computed here.
/// The statistics saved at `stats_file` as they were written, or `None`
/// while none are saved in this format.
pub fn saved_layer_stats(stats_file: &Path) -> Option<Value> {
    saved::read(stats_file).and_then(|stats| serde_json::to_value(stats).ok())
}

pub fn read_layer_stats(
    manifest: &PublishedLayerManifest,
    audience: StatsAudience,
    stats_file: Option<&Path>,
) -> Value {
    read_with(manifest, audience, stats_file, queue::global())
}

/// What was found for a layer.
enum Found {
    Ready {
        stats: LayerStats,
        computed_at: Option<String>,
    },
    Computing {
        row_count: Option<u64>,
    },
    Failed {
        row_count: Option<u64>,
        error: String,
    },
}

fn read_with(
    manifest: &PublishedLayerManifest,
    audience: StatsAudience,
    stats_file: Option<&Path>,
    queue: &StatsQueue,
) -> Value {
    let source = Path::new(&manifest.source_ref);
    let found = match manifest.source_kind {
        SourceKind::GeoParquet => geoparquet(source, stats_file, queue),
        SourceKind::GeoJsonFile => match geojson::stats(source).as_ref() {
            Ok((stats, at)) => Found::Ready {
                stats: stats.clone(),
                computed_at: Some(at.clone()),
            },
            Err(error) => Found::Failed {
                row_count: None,
                error: error.clone(),
            },
        },
        // Neither has one file to describe.
        SourceKind::GeoJsonArtifact | SourceKind::GeoJsonFunction => Found::Ready {
            stats: LayerStats {
                row_count: 0,
                extent: None,
                columns: Vec::new(),
            },
            computed_at: None,
        },
    };
    answer(manifest, audience, found)
}

fn geoparquet(source: &Path, stats_file: Option<&Path>, queue: &StatsQueue) -> Found {
    let Some(now) = Fingerprint::of(source) else {
        return Found::Failed {
            row_count: None,
            error: "the layer's file cannot be read".to_string(),
        };
    };
    let Some(stats_file) = stats_file else {
        return Found::Failed {
            row_count: footer_row_count(source),
            error: "this layer has no place to save statistics".to_string(),
        };
    };
    match saved::read(stats_file) {
        Some(saved) if saved.source == now => match saved.outcome {
            Outcome::Ready(stats) => Found::Ready {
                stats,
                computed_at: Some(saved.computed_at),
            },
            Outcome::Failed { error } => Found::Failed {
                row_count: footer_row_count(source),
                error,
            },
        },
        _ => {
            queue.enqueue(Job {
                source: source.to_path_buf(),
                stats_file: stats_file.to_path_buf(),
                force: false,
            });
            Found::Computing {
                row_count: footer_row_count(source),
            }
        }
    }
}

/// The row count the file's footer records, without reading any data.
fn footer_row_count(source: &Path) -> Option<u64> {
    let file = std::fs::File::open(source).ok()?;
    let builder =
        parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file).ok()?;
    u64::try_from(builder.metadata().file_metadata().num_rows()).ok()
}

fn answer(manifest: &PublishedLayerManifest, audience: StatsAudience, found: Found) -> Value {
    let shown = |name: &str| match audience {
        StatsAudience::Operator => true,
        StatsAudience::Public => {
            crate::mapserver::resolve::property_is_public(&manifest.allowed_properties, name)
        }
    };
    let (state, row_count, extent, columns, computed_at, error) = match found {
        Found::Ready { stats, computed_at } => {
            let columns: Vec<Value> = stats
                .columns
                .iter()
                .filter(|c| shown(&c.name))
                .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
                .collect();
            (
                "ready",
                Some(stats.row_count),
                stats.extent,
                columns,
                computed_at,
                None,
            )
        }
        Found::Computing { row_count } => ("computing", row_count, None, Vec::new(), None, None),
        Found::Failed { row_count, error } => {
            ("failed", row_count, None, Vec::new(), None, Some(error))
        }
    };
    let source_kind = match manifest.source_kind {
        SourceKind::GeoParquet => "geoparquet",
        SourceKind::GeoJsonFile => "geojson",
        SourceKind::GeoJsonArtifact => "geojson_artifact",
        SourceKind::GeoJsonFunction => "geojson_function",
    };
    let mut out = json!({
        "layer_id": manifest.layer_id,
        "source_kind": source_kind,
        "state": state,
        "row_count": row_count,
        "extent": extent,
        "columns": columns,
        "capabilities": {
            "mvt": "experimental",
            "png": true,
            "geojson": true,
            "zxy_tiles": true,
            "point_query": true
        }
    });
    if let Some(at) = computed_at {
        out["computed_at"] = json!(at);
    }
    if let Some(error) = error {
        out["error"] = json!(error);
    }
    if audience == StatsAudience::Operator {
        // The operator must be able to see which columns are exposed today
        // before ticking another one.
        out["allowed_properties"] = json!(manifest.allowed_properties);
    }
    out
}
