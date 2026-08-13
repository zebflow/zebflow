# Format Contract

Zebflow has several formats because source, stored data, transport values, and
runtime state have different jobs. Each format needs one owner and one version
rule.

## Format Rule

Before a format is called stable, Zebflow must record:

- its permanent name
- the source module that owns it
- its format version
- its parser and validator
- where it is stored or sent
- which runtime versions can read it
- how invalid or newer versions are rejected
- how migration works when the format changes

## Current Format Areas

- Project settings use `zebflow.json`. They are owned by
  `src/platform/model.rs` and `src/platform/services/project_config.rs`.
- The project lock uses `zeb.lock`. It is owned by `src/platform/model.rs` and
  `src/platform/services/zeb_lock.rs`.
- Pipeline graphs use `.zf.json`. They are owned by `src/pipeline/model.rs`.
- Pipeline DSL is owned by the platform pipeline parser and shell.
- Node definitions are owned by `src/pipeline/model.rs`.
- Node bundles are owned by `src/platform/model.rs` and
  `src/platform/services/node_registry.rs`.
- Hub packages are owned by `src/platform/model.rs` and
  `src/platform/services/hub.rs`.
- FileRef is owned by `src/pipeline/nodes/basic/file_ref.rs`.
- ZebFS access rules are owned by `src/zebfs/acl.rs`.
- RWE compile and render messages are owned by `src/rwe/protocol.rs`.
- Map publish data is owned by `src/mapserver/publish/manifest.rs`.
- Invocation data is owned by the pipeline model and pipeline runtime service.
- Project transfer is owned by `src/platform/services/project_transfer.rs`.
- The platform catalog is owned by `src/platform/sqlite_schema.rs`.
- The Sekejap store is owned by the Sekejap crate and
  `src/platform/sekejap.rs`.

## FileRef Shape

The current FileRef uses `__zf_type: "file_ref"`, `backend`, `ref`, `size`,
`sha256`, `mime`, `kind`, and `lifecycle`. It may also include `path`, `url`,
`filename`, `origin`, and `trust` for use and inspection.

`temporary` means the file belongs to a run and may be cleaned. `durable` means
the object belongs to project file storage. A backend name tells consumers how
to resolve the reference.

## No Silent Guessing

Unknown format versions must fail with a clear error. A reader must not guess
that a newer object has the same meaning. Legacy import support, when present,
must be separate from the main format written by the current runtime.
