# Storage

Zebflow separates source, data, files, runtime state, and temporary work. Each
kind has a different lifetime and backup rule.

## Project Storage

- `repo/` is project source and Git work.
- `data/` is durable project data and runtime owned state.
- `files/` is the ZebFS object root.
- temporary paths exist only for one bounded operation.

## Interfaces

`src/infra/io/` defines replaceable cache, catalog, object, runtime data, and
state interfaces. `src/zebfs/` defines the project file object model and local
backend. Platform adapters create the physical project layout.

## Rules

- A durable file or record needs one named owner.
- A temporary file must not be required after restart.
- A FileRef describes a file and must not be mistaken for file bytes.
- Public access is policy metadata, not a path naming trick.
- Backups must use a consistent database or store checkpoint.
- A remote object backend should support range or stream access where the data
  engine can use it.
- Storage code must prevent parent path and symlink escape.

## Related Source

- `src/zebfs/`
- `src/infra/io/`
- `src/infra/storage/`
- `src/platform/adapters/file/`
- `src/platform/adapters/project_data/`
- `src/platform/services/project_transfer.rs`
- `src/pipeline/nodes/shared/file_ref.rs`
- `src/pipeline/nodes/basic/fs/object.rs`
