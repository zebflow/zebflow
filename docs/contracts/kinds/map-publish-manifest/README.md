# MapPublishManifest

Status: **review** — spec settled 2026-08-29; code caught up 2026-09-01, when the audit found the `source_path` refusal missing. 2026-10-03: the registry moved to the store tier and a layer pins its `store`.

What a project has published as map layers: which layers exist, where each is
served, what it was built from, and what the public may see of it.

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `MapPublishManifest` |
| Spec | a list of layer records, one per published layer |
| Adapter | `src/contracts/kinds/map_publish_manifest.rs` |
| Written by | the project MapServer service and `mapserver.layer.publish` |
| Lives at | `data/store/mapserver/{instance}.layers.json` — store tier, never a user object |

## Shape

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "MapPublishManifest",
  "metadata": { "name": "joseph/myblog" },
  "spec": [
    {
      "layer_id": "roads",
      "path": "roads",
      "store": "local",
      "source_path": "mapserver/roads.geojson",
      "source_kind": "geojson_artifact",
      "artifact_manifest_path": "data/cache/mapserver-artifacts/roads/manifest.json",
      "mode": "features",
      "min_zoom": 6,
      "max_zoom": 16,
      "bbox_required": true,
      "max_features": 5000,
      "allowed_properties": ["name", "class", "surface"],
      "feature_count": 18432,
      "chunk_count": 12,
      "style": { "color": "#c33", "width": 2 },
      "filter": "class = 'primary'",
      "cache_ttl_secs": 3600
    }
  ]
}
```

| Field | Rule |
| --- | --- |
| `layer_id` | 1–64 of `A-Z a-z 0-9 - _`, unique within the manifest — it is a folder and file name in the cache and the store |
| `path` | where the layer is served, **stored without a leading slash**; one writer adding one and another not is how a published layer fails to serve |
| `store` | the project store `source_path` is a key in — `local` or an `s3` credential id, pinned at publish like a node's `--store` |
| `source_path` | the store key the layer serves; an engine reads it through the project's bounded mirror (`data/cache/zebfs-mirror/`), which answers a directory store's own file in place |
| `source_kind` | how to read that source |
| `artifact_manifest_path` | exactly `mapserver-artifacts/{instance}/{layer}/manifest.json`, in the CACHE tier: rebuildable from `source_path`, never the only copy; a delete removes its folder |
| `mode`, `min_zoom`, `max_zoom`, `bbox_required`, `max_features` | serving limits |
| `allowed_properties` | see below |
| `feature_count`, `chunk_count` | counts of the built artifact |
| `style`, `filter`, `function_slug`, `cache_ttl_secs` | optional presentation and caching |

`mapserver.layer.publish` writes a record from its flags and its answer reads it back
in the flags' words: `--name` → `layer_id`, `--route` → `path`, `--max-items` →
`max_features`, each `--field` → one of `allowed_properties`, `--function` →
`function_slug`, `--ttl 60s` → `cache_ttl_secs: 60`.

## What the public may see

`allowed_properties` decides which columns of the source data leave the server.
It is the security-bearing field of this contract.

| `allowed_properties` | Public sees |
| --- | --- |
| `[]` or absent | **geometry only, no properties** |
| `["name", "class"]` | exactly those two |

There is no wildcard, deliberately. A "select all" control in the publish UI
writes the real column names it found in the source. A wildcard would expose a
column added by a later re-upload without anyone choosing it; an explicit list
keeps a new column hidden until someone ticks it.

The two failure directions are not equal. A forgotten field that yields a map
without labels is visible and fixed in seconds. A forgotten field that leaks a
column is invisible and cannot be undone, because the data has already been
fetched. This is the same closed default as
[`ZebFsAcl`](../zebfs-acl/README.md).

## Rejections

A `layer_id` outside its slug shape, or a duplicate. An `artifact_manifest_path`
of any other shape. A `source_path` that escapes the store — a
project bundle carries the registry, so a record can arrive already written. A
record without `store`. An unknown field at any level.

## Serving

A caller's `?filter=` only narrows: its conditions are added to the
publisher's `filter` (all must hold), and it may name only columns in
`allowed_properties` — a filter on a hidden column is refused (400), since its
yes/no answer would reveal the column. A layer answers only while the
project's `ms` surface is on; publishing says which (`serving`).

A `path` carrying a leading slash is **normalised on read**, not refused: both
publishers strip it before writing, and stripping it on read repairs a record
written before they agreed rather than making the layer unreadable.

## Statistics

`/ms/{owner}/{project}/{path}/stats` (public) and
`/api/projects/{owner}/{project}/mapserver/{instance}/layers/{layer_id}/stats`
(operator) answer `200` for every known layer in every state, `404` only for
an unknown layer:

```json
{
  "layer_id": "sites",
  "source_kind": "geoparquet",
  "state": "ready",
  "row_count": 123,
  "computed_at": "2026-11-03T10:00:00Z",
  "extent": [100.0, -5.5, 101.0, -5.0],
  "columns": [
    {
      "name": "suburb",
      "data_type": "Utf8",
      "null_count": 0,
      "cardinality": 3,
      "cardinality_exact": true,
      "top_values": [["north", 812], ["south", 400], ["east", 11]]
    },
    {
      "name": "floors",
      "data_type": "Int64",
      "null_count": 2,
      "cardinality": 14210,
      "cardinality_exact": false,
      "min": 1.0, "max": 9.0,
      "top_values": [],
      "top_values_skipped": "too_many_distinct",
      "histogram": { "min": 1.0, "max": 9.0, "counts": [20 integers] }
    }
  ],
  "capabilities": { "mvt": "experimental", "png": true, "geojson": true, "zxy_tiles": true, "point_query": true },
  "allowed_properties": ["suburb"]
}
```

| Field | Rule |
| --- | --- |
| `state` | `ready`, `computing` or `failed` |
| `row_count` | a GeoParquet layer's is always present, from the file's footer while `computing` or `failed`; `null` when the footer cannot be read |
| `computed_at` | `ready` only; UTC, RFC 3339 |
| `error` | `failed` only |
| `extent` | the GeoParquet `geo` metadata `bbox`, else the bbox covering columns, else `null` |
| `columns` | `[]` while `computing` or `failed`; the public view lists only `allowed_properties`, the operator view every attribute column and `allowed_properties` beside it. Geometry and bbox columns are not attributes |
| `cardinality` | distinct non-null values: exact when `cardinality_exact`, else a HyperLogLog estimate |
| `min`, `max`, `histogram` | numeric columns only, over their finite values; 20 equal-width bins, the last one includes `max` |
| `top_values` | at most 10 `[value, count]`, most frequent first, ties by value; counts are exact. Counted only when the estimated distinct count is at most 10,000; otherwise `[]` with `top_values_skipped: "too_many_distinct"`. A nested column (list, struct, map) has only its null count and `top_values_skipped: "unsupported_type"` |

A GeoParquet layer's statistics are computed once, in the background, when the
layer is published or its file changes, one computation at a time per process,
and saved at `data/cache/mapserver-artifacts/{instance}/{layer}/stats.json`
(CACHE: rebuilt from the file, removed with the layer's folder) with the size
of the file they describe and a SHA-256 of its last 64 KiB, where a Parquet
file keeps its footer; not its modification time, which a bucket's mirror
copy moves on every read. `/stats` only reads that
file; when it is missing or describes another version of the file, the answer
is `computing` and a computation is queued. A `failed` result stays until the
file changes or the layer is published again. A GeoJSON file layer is scanned
when asked and every number is exact; a file over 50 MB answers `failed`. An
artifact or function layer reports no columns.

## Open

- **Rebuild.** `artifact_manifest_path` points at CACHE-tier artifacts that must
  be rebuildable from `source_path`. Nothing tests that they are
  (`stability-matrix.md` row 18).
- **A deleted source.** Whether a layer whose `source_path` no longer exists is
  refused at publish, dropped at read, or served from artifacts alone.
- **Style shape.** `style` is free-form JSON with no schema. It travels between
  publisher and renderer with nothing checking that both mean the same thing.
