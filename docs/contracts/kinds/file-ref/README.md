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
| `backend` | who owns the bytes: the project's **native** store, declared in [`spec.files.backend`](../project-configuration/README.md#specfiles) and spelled by the same constant. `zebfs` today; `s3` and other stores add a backend, not a format. Never the place bytes were fetched *from* |
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

## The format survives a change of backend

ZebFS is one object-storage model whose first backend is the local filesystem;
the settings page already offers S3, R2, MinIO, B2 and Tigris as the next ones.
The shape above is what makes that swap a backend change rather than a format
change.

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
- **How a URL is obtained.** The format carries none, deliberately. What does
  not exist yet is the operation that answers for one — local returning its
  `/fs/{owner}/{project}/{ref}` path, S3 a presigned link — nor the choice
  between proxying bytes through Zebflow, which keeps the ACL authoritative,
  and presigning, which does not.
- **The backend seam is cut but not widened.** Which backend a project uses is
  now declared in `spec.files.backend` and resolved in one place
  (`zebfs::backend::open`, reached through `ProjectFileLayout::open_files`), so
  no caller constructs an implementation by name. What is still missing is the
  implementation behind it: `LocalZebFs` is a concrete struct and there is no
  trait for a second backend to implement. The format is ready for one, the
  declaration is ready for one, the storage code is not.
