# Format Contract

Zebflow uses one contract registry for durable documents and transferred
objects across all domains. The registry lives in `src/contracts/registry.rs`.
Adapters live in `src/contracts/kinds/`.

## Canonical Envelope

Authoritative JSON and strict YAML documents use this exact root shape:

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "Pipeline",
  "metadata": {
    "name": "hello"
  },
  "spec": {}
}
```

The only root fields are `apiVersion`, `kind`, `metadata`, and `spec`. Unknown
root fields, missing fields, unknown kinds, future versions, and the wrong kind
are rejected. Structural objects in `spec` reject unknown fields unless the
contract names that part as an extension point.

`metadata.version` is the release version of an independent package or
artifact. It is not the contract format version. The format version is always
`apiVersion`.

## Registered Kinds

The review and freeze status for each kind is maintained in the
[registered kinds index](./kinds/README.md).

| Kind | Representation | Purpose |
| --- | --- | --- |
| `ProjectConfiguration` | Envelope | Project `zebflow.yaml` settings |
| `ProjectManifest` | Reserved | Name held for a future project manifest; no reader accepts it yet |
| `Pipeline` | Envelope | Saved and active pipeline graphs |
| `DependencyLock` | Envelope | Exact RWE library and installed node-bundle pins in project `zeb.lock` |
| `NodeDefinition` | Envelope | One normalized node interface shared by native, composite, and WASM nodes |
| `NodeBundle` | Envelope | One installable bundle containing one or more nodes |
| `FileRef` | Inline payload | Small reference passed inside node JSON |
| `ZebFsAcl` | Envelope | Project file access rules |
| `DatabaseSchema` | Envelope | Portable project database schema |
| `ProjectBundle` | Envelope | Manifest inside a project transfer archive |
| `RuntimeBundle` | Envelope | Office-to-office runtime materialization request |
| `HubPackage` | Envelope | Published Hub package artifact |
| `LibraryManifest` | Envelope | Embedded or distributed RWE library definition |
| `MapPublishManifest` | Envelope | Project MapServer published-layer registry |
| `InvocationRecord` | Database record | Bounded project invocation history row |

Reserved kinds are not usable formats. They keep a name from being assigned a
different meaning before its contract is designed.

## Validation Time

Full document validation runs when Zebflow saves, imports, installs, activates,
materializes, or loads durable state at startup. A normal pipeline hit uses the
already compiled in-memory plan. It does not parse every durable contract again.

Generated caches may use a private format because they can be deleted and
rebuilt. SQL schema files remain SQL. Database rows are protected by database
migrations and typed adapters rather than being wrapped in a JSON envelope.

## FileRef

FileRef stays inline because it is a hot-path node value, not a standalone
document:

```json
{
  "__zf_type": "file_ref",
  "backend": "zebfs",
  "ref": "uploads/roads.geojson",
  "size": 24810,
  "sha256": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "mime": "application/geo+json",
  "lifecycle": "durable"
}
```

Required fields are `__zf_type`, `backend`, `ref`, `size`, `sha256`, `mime`, and
`lifecycle`. The digest contains exactly 64 hexadecimal digits. A consumer checks
the stored byte count and digest before returning bytes. Legacy path-shaped JSON
is not guessed to be a FileRef.

## Change Rule

Each new authoritative document format must:

1. register one permanent kind in `src/contracts/registry.rs`
2. add one typed adapter in `src/contracts/kinds/`
3. use the shared decoder and atomic writer at every boundary
4. define required fields, extension points, size rules, and semantic validation
5. add valid, malformed, unknown-field, wrong-kind, and future-version tests
6. update the stability matrix before release

Legacy import, when needed after v1, must be a separate explicit converter. The
current reader never guesses an old shape.
