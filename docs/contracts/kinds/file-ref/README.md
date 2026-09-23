# FileRef

Status: **review** — spec settled 2026-08-29; code caught up. Audited 2026-09-01: all eleven fields are required, `kind` and `lifecycle` are closed, the digest is checked for the `sha256:` prefix and 64 lowercase hex digits, and size is verified against the stored bytes before they are returned. The entries under Open are open, not owed.

The note that travels between pipeline nodes when a file moves through a run.
Bytes are written to storage; this small object is what the nodes pass around.

## Identity

| | |
| --- | --- |
| Marker | `__zf_type: "file_ref"` |
| Representation | inline payload — an object inside node data, never a file of its own |
| No envelope | it is not a document: no `apiVersion`, no `metadata`, no `spec` |
| Adapter | `src/pipeline/nodes/shared/file_ref.rs` |

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
| `backend` | who owns the bytes: the project's **native** store, declared in [`spec.files.backend`](../project-configuration/README.md#specfiles) and spelled by the same constant. `zebfs` today; `s3` and other stores add a backend, not a format. Never the place bytes were fetched *from* |
| `ref` | **opaque.** Only the named backend may interpret it — no node parses it, joins it to a path, or assumes a local file |
| `filename` | the uploader's own name, for display and for what a save writes |
| `mime` | content type |
| `kind` | one of `geojson`, `json`, `csv`, `image`, `audio`, `video`, `pdf`, `spreadsheet`, `archive`, `parquet`, `binary`; derived from `mime` and `filename`. A shorthand pipeline authors branch on, so the eleven values are the promise. `spreadsheet` is xlsx/xls/ods by mime and extension; csv stays `csv` |
| `size` | byte count, verified before bytes are returned |
| `sha256` | `sha256:` + exactly 64 lowercase hex digits, verified before bytes are returned |
| `lifecycle` | `temporary` — deleted after the run — or `durable`, a project file that stays |
| `origin` | where it entered: `webhook`, `manual` (a signed-in operator's run — an upload on the Run form, or a store path named in the JSON form), `http.response`, `node-output`, `fs.image.thumbnail`, `fs.image.chromakey`, `fs.svg.convert`, `fs.save`, `fs.put`, `fs.copy`, `fs.move`, `fs.compress`, `project.files.upload`. Open: a new producer adds a word |
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

## The format survives a change of backend

ZebFS is one object-storage model with two backends: the local filesystem
(`zebfs`) and any S3-compatible bucket (`s3` -- AWS S3, Cloudflare R2, MinIO,
SeaweedFS, Garage, B2, Tigris). The shape above is what makes the swap a
backend change rather than a format change: a project moved to a bucket writes
the same eleven fields with `backend: s3`, and `ref` is the key the bucket
knows it by, under the project's prefix.

Every field is true wherever the bytes live. `sha256`, `size`, `mime`, `kind`
and `filename` describe the object, not its host. `backend` names who owns it.
`ref` is that backend's own key, opaque to everyone else.

**Nothing here locates the bytes on a network.** The endpoint, the bucket, and
the credential are configured once on the connection, never repeated per file —
so moving a bucket, changing an endpoint, or swapping MinIO for R2 changes one
setting and no stored FileRef.

**The native store and an outside bucket are different questions.** Which store
this project keeps its files in is declared once per project in
`spec.files.backend`; that declaration is what this field names, and
`src/zebfs/backend.rs` is the one place it becomes an implementation. A bucket a
pipeline explicitly reads and writes -- someone else's S3, MinIO, or SeaweedFS
-- is a connection with a credential, chosen per pipeline, and no such node
exists yet. A node that reads from one produces bytes that land in the native
store, so the FileRef it emits carries the **native** backend value. It never
carries `s3` on account of where the bytes came from: if it did, `ref` would
stop meaning one thing, sometimes a key in the store Zebflow owns and sometimes
a key in a bucket it does not.

That is why there is no `url`. A URL is assembled when a caller needs one, from
the connection plus `ref`, and presigned if the object is private. Stored
instead, it would be either a link that expires while the FileRef sits in run
history, or a permanent public address for a private object — and it would bake
"these bytes are on this machine's disk" into a format meant to outlive that
assumption. For the same reason `path` is gone: it named a filesystem.

## Rejections

Missing any required field. A `sha256` without the prefix or without exactly 64
lowercase hex digits. A `size` or digest that disagrees with the stored bytes.
An unknown `kind` or `lifecycle`. A `ref` read by anything other than its
backend.

## Amendments

| Date | Change | Why it was safe |
| --- | --- | --- |
| 2026-09-19 | `kind` gains `audio`, `video`, `spreadsheet` (eight → eleven). `origin` gains `manual`. | Every producer still derives `kind` through the one classifier, so no writer can emit a twelfth; a consumer that matched the eight exhaustively now sees a value it did not expect only for bytes that were `binary` before, which is the case the closed vocabulary exists to make loud. `origin` is open by its own rule. |
| 2026-09-19 | Temporary cleanup has a writer: `remove_run_temporary_files` deletes `tmp/runs/{request_id}/` after a webhook run and after a manual run, whether the run succeeded or failed. | The promise ("deleted after the run") is unchanged; what changed is that it is kept. A node that needs the bytes past the run copies them out (`fs.save`), which is what it always had to do. |
| 2026-09-24 | `backend` gains `s3`: the project's native store may be an S3-compatible bucket, declared as `spec.files.backend: s3` with the credential (kind `s3`) selected per instance in `data/store/files-backend.json`. Every producer takes the word from the project's layout; `read_file_ref_bytes` refuses a ref whose word is not the project's store; `zebfs_rel_path` accepts either native word and leaves the store to interpret it. | The eleven fields are unchanged and `ref` was already opaque, so a consumer that honoured the contract never joined it to a path. `zebfs::backend::open` was the one seam, and `ZebFs` (an enum, not a trait) is what it now answers; every caller that held a `LocalZebFs` holds a `ZebFs` and reads the same seven verbs. |
| 2026-09-20 | `fs.save` (`saved`), `fs.put` / `fs.copy` / `fs.move` (`fs.object`) and `fs.compress` (`compressed`) answer the stored file as a **bare durable FileRef** — the eleven fields and nothing else; `origin` gains those five words. No `path`, `url`, `original_name`, `content_type`, `modified`, `archive_path`, `archive_url`, `source_path(s)` or `format` beside them. The DSL `execute pipeline` removes `tmp/runs/{request_id}/` too, and writes the record. (Revised 2026-09-21: the first cut carried those plain keys beside the eleven for pipelines written against the old shape; this is a fresh version and nothing carries old shapes, so they are gone from the producers and from every consumer in the tree.) | The eleven fields are all present and derived the one way, so every consumer of a FileRef takes these values as they are. A consumer that needs the store path reads `ref`; a URL is not a node's business — the Studio reads an object at `files/object?ref=`, a site serves `public/` at `/_files/…`. The nodes that read `saved` by default (`fs.image.thumbnail`, `fs.compress`, `fs.decompress`, `fs.pdf_convert`) now default `--source-key` to `saved` — the FileRef itself, which the shared resolver takes — instead of `saved.path`. A consumer that matched `fs.object.kind == "object"` after `fs.put` sees the FileRef word instead; the value was always an object on those three operations. |

## Open

- **`trust` is written but never read.** Four values are produced —
  `untrusted` (`http_request.rs`, webhook ingress), `sanitized`
  (`fs_thumbnail.rs`), `generated` (node output files), `user`
  (`web/mod.rs` upload) — and no consumer branches on any of them. The field
  carries a real distinction; nothing spends it yet. Corrected 2026-08-31: an
  earlier draft said the field had one value and proposed removing it.
- **Temporary cleanup on other triggers.** `tmp/runs/{request_id}/files/` is
  removed after a webhook run, after a manual run on the API route, and
  (2026-09-20) after a DSL `execute pipeline` run (see Amendments). No
  other trigger writes a temporary FileRef today; one that starts to must
  join the same removal.
- **Remote streaming.** On `s3` every consumer holds whole bytes: `get` answers
  the object in memory. The nodes that stream from a *file path* instead --
  `fs.image.thumbnail`, `fs.pdf_convert`, `fs.compress` / `fs.decompress`,
  `geo.convert` / `geo.inspect`, `ai.tts`, `ms.*`, `table.query` and the
  streamed `table.convert` -- refuse a bucket project by name
  (`ZEBFS_LOCAL_ONLY`) rather than reading `files/`, which on that project is
  scratch and not the store. Pulling an object down to a temporary file for
  them is the design not made yet.
- **How a URL is obtained.** The format carries none, deliberately. What does
  not exist yet is the operation that answers for one — local returning a
  path, S3 a presigned link — nor the choice between proxying bytes through
  Zebflow, which keeps the ACL authoritative, and presigning, which does not.
  What does exist (2026-09-20) is one route, not a FileRef operation: the
  Studio and an MCP session read a `zebfs` ref, private or public, through
  `GET /api/projects/{owner}/{project}/files/object?ref=…` with the session;
  the public surfaces (`/_files/…`, `/fs/…`) stay what they are for sites.
- **Moving a project between backends moves no bytes.** Selecting a bucket on
  the Files page changes where new objects go; what was on disk stays on disk
  and a FileRef written before the change still says `zebfs`, which the
  bucket project refuses by name. A copy step is the owner's, by hand, today.
