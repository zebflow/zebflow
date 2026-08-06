# Storage Formal Guide

This guide defines the intended **stable mental model** for Zebflow storage.

Start from zero:

- Zebflow already has a source workspace for pipelines, templates, and app code.
- Zebflow already has database/runtime state.
- Zebflow still needs a first-class place for durable file-like artifacts.

Storage is that third pillar.

Zebflow storage uses an object-storage mental model:

```text
bucket-like namespace + object path
```

For a Zebflow project, the bucket-like namespace is the project itself:

```text
{owner}/{project}
```

So one Zebflow project automatically provides one default object namespace.

## 1. First Rules

### 1.1 Storage is first-class

File/object storage is not an optional plugin concept.

Zebflow needs a first-class storage system for:

- uploads
- generated files
- media
- documents
- archives
- reports
- datasets
- transformed data artifacts
- static output
- hub package/media artifacts

The implementation may be local filesystem, S3-compatible object storage, or
another backend, but the user-facing product concept is one project filesystem:
**Zebflow FS**.

### 1.2 Single installer must work alone

A single Zebflow installer must work without requiring MinIO, S3, Redis, or any
external component.

The default filesystem backend for a fresh standalone install is local.

This local implementation must behave through the same FS interface as any
future external backend. Local is not a lower-class mode.

### 1.3 One interface

Zebflow should have one obvious way to store durable artifacts.

Nodes and platform features should not invent separate file systems, separate
artifact paths, or backend-specific behavior.

They should read and write through Zebflow FS semantics.

## 2. Formal Split

Zebflow storage is not "everything is object storage".

The formal split is:

```text
repo/   source workspace
data/   runtime state
files/  durable artifacts
```

### `repo/`

`repo/` is source.

It contains:

- pipelines
- templates
- docs
- app code
- project manifest
- Git workspace

It is not Store-backed.

### `data/`

`data/` is runtime state.

It contains:

- SQLite
- Sekejap
- runtime indexes
- runtime caches
- lock-sensitive state

It is not normal Store-backed artifact storage.

Embedded databases and lock-heavy runtime state need local/POSIX-style
semantics.

### `files/`

`files/` is durable artifact storage.

It is Zebflow FS-backed.

It contains:

- uploaded files
- generated files
- public assets
- private assets
- data artifacts
- reports
- media
- export bundles

## 3. Vocabulary

### Zebflow FS

**Zebflow FS** is the first-class durable artifact system for one project.

It behaves like object storage:

```text
project namespace + object path
```

The project namespace acts like the bucket:

```text
{owner}/{project}
```

The object path acts like the key:

```text
static/blog/index.html
uploads/source.csv
datasets/crime/raw.csv
```

The HTTP shape is:

```text
/fs/{owner}/{project}/{path}
```

Example:

```text
/fs/superadmin/default/static/blog/index.html
```

This is similar to MinIO/SeaweedFS path-style object URLs:

```text
/{bucket}/{key}
```

In Zebflow:

```text
bucket-like namespace = {owner}/{project}
key/path              = {path}
```

### Backend

A Zebflow FS backend stores the project artifacts physically.

Possible backends:

- local
- S3-compatible
- MinIO
- SeaweedFS
- Cloudflare R2
- Ceph RGW
- Contabo Object Storage
- Backblaze B2
- DigitalOcean Spaces

### ObjectRef

An **ObjectRef** identifies one artifact.

Minimum shape:

```json
{
  "path": "uploads/source.csv",
  "url": "/fs/superadmin/default/uploads/source.csv",
  "content_type": "text/csv",
  "size": 12345,
  "checksum": "sha256:..."
}
```

### Path

`path` is the user-facing object name inside the project filesystem namespace.

Zebflow uses `path` in UI and DSL because it is easier for humans and LLMs.

S3-compatible backends may internally call this an object key.

## 4. Default FS and DSL

Every project has a default Zebflow FS namespace.

Fresh standalone default:

```text
{owner}/{project} -> local backend
```

Production may point that project filesystem to another backend.

Basic node configs should not need to mention a backend.

Example:

```zf
| fs.save --field image --path uploads/avatar.png
```

This writes to:

```text
/fs/{owner}/{project}/uploads/avatar.png
```

## 5. Zebflow FS Contract

Zebflow FS uses an S3-like object contract for Zebflow usage.

The contract is:

```text
namespace + path + metadata
```

For Zebflow:

```text
namespace = {owner}/{project}
path      = object path inside the project FS
```

Minimum operations:

```text
put      write object
get      read object
head     read object metadata
list     list objects by prefix
delete   delete object or prefix
copy     copy object
move     move object
mkdir    create prefix
```

The first backend is local filesystem.

Local ZebFS must use native filesystem calls internally so local standalone
installs remain fast and robust, including static generation that writes
thousands of files.

Internal Zebflow nodes must call the ZebFS Rust/API layer directly.

They must not call the HTTP route internally.

The HTTP route is the external object access surface:

```text
/fs/{owner}/{project}/{path}
```

## 6. Node and DSL Rule

Nodes that read or write durable file-like artifacts should use Zebflow FS.

The DSL should stay easy for humans and LLMs:

```zf
| fs.save --field image --path uploads/avatar.png
| fs.thumbnail --source-path uploads/avatar.png --path thumbnails/avatar.jpg
```

When a node receives an object from an upstream node, the payload should include
an ObjectRef.

Example:

```json
{
  "object": {
    "path": "uploads/avatar.png",
    "url": "/fs/superadmin/default/uploads/avatar.png",
    "content_type": "image/png",
    "size": 12345
  }
}
```

For LLM browsing and authoring, Zebflow should expose simple FS-oriented tools
and DSL-visible operations:

```text
fs.list --path docs
fs.head --path docs/readme.md
fs.get --path docs/readme.md
fs.put --path docs/readme.md --text "hello"
fs.delete --path docs/old.md
fs.copy --from docs/readme.md --to docs/readme-copy.md
fs.move --from docs/draft.md --to docs/published.md
fs.mkdir --path docs/assets
```

The default list output should be compact and directly usable by later commands:

```text
uploads/avatar.png | image/png | 12 KB
datasets/crime/raw.csv | text/csv | 2.1 MB
```

## 7. Project Studio UI

Project Studio should treat storage as a first-class project area, similar in
importance to DB connections.

Initial UI:

```text
Files / FS
  Browse
  Upload
  Create folder/prefix
  Delete
  Inspect metadata
```

The default project FS exists from the beginning:

```text
ZebFS default
```

Future backend setup can add providers:

```text
local
MinIO
AWS S3
SeaweedFS
Cloudflare R2
Ceph RGW
Contabo Object Storage
Backblaze B2
DigitalOcean Spaces
```

External backend setup should attach the credential needed by that backend.

The UI should not force users to understand backend-specific details for common
file workflows. Basic flows should continue to use the default project FS.

## 8. Access Policy

Zebflow FS objects are private by default.

The URL:

```text
/fs/{owner}/{project}/{path}
```

is an object address. It does not mean the object is publicly readable.

Default behavior:

```text
GET /fs/{owner}/{project}/{path}
  -> requires project file read permission
```

Public access must be explicitly enabled.

The public enablement interface should be the same regardless of backend:

- local
- MinIO
- AWS S3
- SeaweedFS
- Cloudflare R2
- Ceph RGW
- other S3-compatible backends

Public policy may target:

- one object path
- one prefix/folder path

Examples:

```text
public object:
  static/blog/index.html

public prefix:
  static/blog/
```

The user-facing UI should allow:

```text
Set public
Remove public
View policy
```

This policy belongs to Zebflow FS, not to a specific storage backend.

Backend-specific delivery can be decided later. A public Zebflow FS object may be
served by:

- Zebflow proxy
- signed URL
- public base URL / CDN
- backend-native policy

The formal product rule is:

```text
private by default
public only by explicit Zebflow FS policy
```
