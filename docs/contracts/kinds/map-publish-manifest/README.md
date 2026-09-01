# MapPublishManifest

Status: **review** — spec settled 2026-08-29; code caught up 2026-09-01, when the audit found the `source_path` refusal missing.

What a project has published as map layers: which layers exist, where each is
served, what it was built from, and what the public may see of it.

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `MapPublishManifest` |
| Spec | a list of layer records, one per published layer |
| Adapter | `src/contracts/kinds/map_publish_manifest.rs` |
| Written by | the project MapServer service and `n.mapserver.crud` |

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
      "source_path": "files/uploads/roads.geojson",
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
| `layer_id` | non-empty, unique within the manifest |
| `path` | where the layer is served, **stored without a leading slash**; one writer adding one and another not is how a published layer fails to serve |
| `source_path` | the project file the layer was built from |
| `source_kind` | how to read that source |
| `artifact_manifest_path` | the generated artifacts, in the CACHE tier: rebuildable from `source_path`, never the only copy |
| `mode`, `min_zoom`, `max_zoom`, `bbox_required`, `max_features` | serving limits |
| `allowed_properties` | see below |
| `feature_count`, `chunk_count` | counts of the built artifact |
| `style`, `filter`, `function_slug`, `cache_ttl_secs` | optional presentation and caching |

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

An empty or duplicate `layer_id`. A `source_path` that escapes the project —
the registry is an object a project member may upload over and a file a project
bundle carries, so a record can arrive already written, and the serving path
joins `source_path` onto the project's files directory. An unknown field at any
level.

A `path` carrying a leading slash is **normalised on read**, not refused: both
publishers strip it before writing, and stripping it on read repairs a record
written before they agreed rather than making the layer unreadable.

## Open

- **Rebuild.** `artifact_manifest_path` points at CACHE-tier artifacts that must
  be rebuildable from `source_path`. Nothing tests that they are
  (`stability-matrix.md` row 18).
- **A deleted source.** Whether a layer whose `source_path` no longer exists is
  refused at publish, dropped at read, or served from artifacts alone.
- **Style shape.** `style` is free-form JSON with no schema. It travels between
  publisher and renderer with nothing checking that both mean the same thing.
