# FileRef

Status: **review** — spec settled 2026-08-29, code catch-up owed.

The note that travels between pipeline nodes when a file moves through a run.
Bytes are written to storage; this small object is what the nodes pass around.

## Identity

| | |
| --- | --- |
| Marker | `__zf_type: "file_ref"` |
| Representation | inline payload — an object inside node data, never a file of its own |
| No envelope | it is not a document: no `apiVersion`, no `metadata`, no `spec` |
| Adapter | `src/pipeline/nodes/basic/file_ref.rs` |

## Shape

```json
{
  "__zf_type": "file_ref",
  "backend": "zebfs",
  "ref": "tmp/runs/abc123/files/9f2c8d.jpg",
  "filename": "photo.jpg",
  "mime": "image/jpeg",
  "kind": "image",
  "size": 51234,
  "sha256": "sha256:e3b0c44298fc1c14a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718",
  "lifecycle": "temporary",
  "origin": "webhook",
  "trust": "untrusted"
}
```

All eleven fields are required.

| Field | Rule |
| --- | --- |
| `__zf_type` | exactly `file_ref`. Path-shaped JSON is never guessed to be one |
| `backend` | who owns the bytes. `zebfs` today; `s3` and other stores add a backend, not a format |
| `ref` | **opaque.** Only the named backend may interpret it — no node parses it, joins it to a path, or assumes a local file |
| `filename` | the uploader's own name, for display and for what a save writes |
| `mime` | content type |
| `kind` | one of `geojson`, `json`, `csv`, `image`, `pdf`, `archive`, `parquet`, `binary`; derived from `mime` and `filename`. A shorthand pipeline authors branch on, so the eight values are the promise |
| `size` | byte count, verified before bytes are returned |
| `sha256` | `sha256:` + exactly 64 lowercase hex digits, verified before bytes are returned |
| `lifecycle` | `temporary` — deleted after the run — or `durable`, a project file that stays |
| `origin` | where it entered: `webhook`, `http.response`, `node-output`, `fs.thumbnail`, `project.files.upload`. Open: a new producer adds a word |
| `trust` | how far the bytes are trusted: `untrusted` (arrived from outside), `sanitized` (re-encoded by a node that discards what it did not understand), `generated` (a node produced them), `user` (a signed-in operator uploaded them). Open: no consumer branches on it yet |

## Why it has no envelope

A FileRef sits beside ordinary values in a run's data:

```json
{ "email": "joseph@example.com",
  "avatar": { "__zf_type": "file_ref", "backend": "zebfs", "…": "…" } }
```

It is never written on its own, never transferred on its own, and exists only
during a run. `__zf_type` is its discriminator, doing the work `kind` does in
document contracts.

## Rejections

Missing any required field. A `sha256` without the prefix or without exactly 64
lowercase hex digits. A `size` or digest that disagrees with the stored bytes.
An unknown `kind` or `lifecycle`. A `ref` read by anything other than its
backend.

## Open

- **`trust` is written but never read.** Four values are produced —
  `untrusted` (`http_request.rs`, webhook ingress), `sanitized`
  (`fs_thumbnail.rs`), `generated` (node output files), `user`
  (`web/mod.rs` upload) — and no consumer branches on any of them. The field
  carries a real distinction; nothing spends it yet. Corrected 2026-08-31: an
  earlier draft said the field had one value and proposed removing it.
- **Temporary cleanup.** `lifecycle: temporary` promises deletion after the
  run. `tmp/runs/{request_id}/files/` has no writer that removes it
  (`stability-matrix.md` row 8).
- **Remote streaming.** With a non-`zebfs` backend, whether a consumer streams
  or must hold whole bytes in memory is undefined.
