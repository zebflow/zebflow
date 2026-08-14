# Contract Stability Matrix

This matrix is Zebflow's north star for stable storage and data transfer. A
project created with a stable Zebflow release must remain readable, movable,
executable, and safely upgradable.

The rows are review areas. One area may contain more than one independently
versioned contract. Zebflow must not use one global format version for unrelated
formats.

## Status Words

- **Critical** means a known conflict, missing identity, or missing version can
  make future compatibility unsafe.
- **Partial** means a format or mechanism exists, but one or more freeze checks
  are not enforced.
- **Review** means the area is recorded but its code and tests still need a full
  contract review.
- **Candidate** means every freeze check passes and the contract is ready to be
  declared stable.
- **Frozen** means Zebflow has published the contract and changes must follow the
  versioning contract.

No row becomes **Frozen** because it looks complete. It becomes frozen only
after its validator, compatibility behavior, migration behavior, and tests have
been verified.

## Priority Matrix

| No. | Priority | Review area | Contract or format today | Current version | Main owner | Current state | Required next decision |
| ---: | :---: | --- | --- | --- | --- | :---: | --- |
| 1 | P0 | Contract governance | Contract identity, compatibility, and release rules | Policy only | `docs/contracts/` | Review | Approve one lifecycle and freeze checklist for every contract |
| 2 | P0 | Project directory | Project repository, data, files, runtime, and generated paths | None | `src/platform/adapters/file/mod.rs`, project services | Critical | Define the permanent project tree and classify durable, generated, and temporary paths |
| 3 | P0 | Project configuration | `zebflow.json` | `1.0` | `src/platform/model.rs`, `src/platform/services/project_config.rs` | Partial | Strict version parsing and atomic replacement are implemented; define migration and prevent concurrent read-modify-write loss |
| 4 | P0 | Pipeline source | Pipeline graph and `.zf.json` DSL source | `zebflow.pipeline` `0.1` | `src/pipeline/model.rs`, `src/pipeline/contract.rs`, pipeline services | Partial | The envelope is enforced and source/snapshot writes are atomic; make source, metadata, and activation one recoverable operation |
| 5 | P0 | Dependency lock | `zeb.lock` | `1` | `src/platform/model.rs`, `src/platform/services/zeb_lock.rs` | Partial | Strict version parsing and atomic replacement are implemented; prevent concurrent read-modify-write loss |
| 6 | P0 | Installed node definition | Native, composite, and WASM node definitions after installation | No single persisted format version | `src/pipeline/model.rs`, node registry | Critical | Define one installed definition contract shared by every node implementation |
| 7 | P0 | Node bundle | Installable single-node and multi-node source | Intended `zebflow-package-v2` | `src/platform/model.rs`, `src/platform/services/node_registry.rs` | Critical | Keep one bundle format, enforce its identity, and remove alternate canonical paths before release |
| 8 | P0 | File reference | FileRef passed between native, composite, and WASM nodes | No explicit version | `src/pipeline/nodes/basic/file_ref.rs` | Critical | Freeze a backend-independent FileRef and remove silent shape guessing |
| 9 | P0 | Project file storage | ZebFS paths, object identity, lifecycle, and ACL manifest | ACL `1` | `src/zebfs/` | Partial | ACL validation and durable atomic local writes are implemented; verify FileRef integrity and define temporary-object cleanup |
| 10 | P0 | Project data stores | Platform SQLite catalog and project Sekejap schema/store | SQLite migration `15`; Sekejap schema `v1` | `src/platform/adapters/data/sqlite.rs`, `src/platform/sekejap.rs` | Critical | Reject future schemas, add consistent backup/recovery, recover or reopen after lock poisoning, and make schema application atomic |
| 11 | P0 | Project transfer | Project bundle and files-only export/import | Manifest `0.3.0` | `src/platform/services/project_transfer.rs` | Critical | Validate versions and digests before mutation; checkpoint live stores; harden archive extraction; stage and atomically replace with rollback |
| 12 | P0 | Runtime synchronization | Materialization bundle, snapshot, secret requirements, and migration plan | Schema `1` | `src/infra/execution/sync/` | Critical | Reject unsupported bundles before mutation; validate hashes; stage materialization; add rollback and content-based incremental sync |
| 13 | P1 | Reactive web protocol | RWE compile, render, event, and error messages | `rwe.v1` | `src/rwe/protocol.rs` | Partial | Separate source, compiled artifact, and wire protocol contracts and test version rejection |
| 14 | P1 | Hub package | Published package metadata, media, manifest, and content | Package format `v2`; package release version is independent | `src/platform/services/hub.rs`, platform models | Critical | Artifact schema and local hashes are now enforced; make releases immutable and installation staged with rollback |
| 15 | P1 | Public and cluster protocols | HTTP API, webhook, MCP, controller-worker, and service messages | Mostly route-based or unversioned | Platform routes, MCP, cluster services | Critical | Group protocol families, define compatibility windows, and version durable wire envelopes |
| 16 | P2 | Invocation and temporary state | Invocation records, traces, node payloads, caches, and temporary files | No common version | Pipeline runtime and invocation services | Critical | Make it bounded and disposable; version only records that may survive a runtime upgrade |
| 17 | P1 | Library definition | Installed library manifests, versions, URLs, integrity, and project enablement | No explicit format version | `src/platform/services/library.rs`, library catalog files | Critical | Define one versioned library manifest and verify source identity and integrity before enabling it |
| 18 | P1 | Map publish manifest | Published layer identity, source, generated artifacts, styles, and caches | Several internal shapes | `src/mapserver/publish/`, mapserver nodes | Review | Separate durable publish intent from generated map artifacts and freeze the durable manifest |

## Verified Audit, 2026-08-14

The source audit and an independent read-only review reached the same release
assessment:

- Zebflow is ready for demonstrations and controlled single-node development.
- A pinned single-node production deployment is possible with external backups,
  trusted inputs, and conservative payload sizes.
- Zebflow is not ready to declare a stable v1 storage and transfer contract.

The full library suite passed: 400 default tests and three tests ignored by
default, run separately. This proves the currently tested behavior. Focused
failure tests now cover atomic-write interruption and forward-version rejection
for the contracts listed in the implementation record below. It does not yet
prove multi-file rollback, full package rollback, or every tamper boundary.

The strongest verified mechanisms are transactional platform SQLite migrations,
path-confined atomic ZebFS object writes, graph and node-definition validation,
remote Hub artifact hash checks, and staged map artifact publishing.

The highest remaining risks are multi-file operations without rollback,
destructive project import and runtime materialization, non-atomic node and Hub
installation, unverified FileRef reads, poisoned Sekejap locks, and repeated
cloning of large JSON node payloads. Other authoritative JSON formats still
need to adopt the shared strict reader and durable writer as their rows are
reviewed.

## Implementation Record, 2026-08-14

The first shared persistence foundation is implemented in
`src/infra/io/durable.rs`:

- Same-directory temporary writes, file sync, atomic replacement, and parent
  directory sync on Unix.
- Reusable root-field contract validation before deserialization or writing.
- Missing files are distinct from malformed, unreadable, and unsupported files.
- Injected partial-write tests prove that a failed replacement preserves the
  previous valid file and removes the temporary file.

The foundation now protects these paths:

- `zebflow.json` project configuration.
- `zeb.lock` dependency locks.
- ZebFS local objects and the ACL manifest.
- Pipeline source and active runtime snapshots.
- Installed node files and Hub artifact files at the individual-file boundary.

Canonical strict readers now reject missing or unsupported versions for project
configuration, dependency locks, ACL manifests, pipeline graphs, and Hub
artifacts. Hub artifact reads also verify the stored SHA-256 digest before use.

This record does not claim whole-operation atomicity. Pipeline source plus
metadata, node bundle installation, Hub installation, project transfer, and
runtime materialization still span multiple durable objects and need staging,
commit, and rollback mechanisms.

## Freeze Checklist

Every row must pass all applicable checks before its state changes to
**Candidate** or **Frozen**.

- [ ] The contract has one permanent identity.
- [ ] Its durable or transmitted root contains an explicit version.
- [ ] The owner module and storage or transmission locations are recorded.
- [ ] One canonical writer emits the current version.
- [ ] Readers list the exact versions they support.
- [ ] A central validator runs before storage, activation, installation, or
      execution.
- [ ] Unknown and newer versions fail with a stable, useful error.
- [ ] Unknown-field and missing-field behavior is explicit.
- [ ] Defaults are deterministic and do not change stored meaning silently.
- [ ] Size, integrity, trust, and access rules are explicit where applicable.
- [ ] Migration is atomic or has a written recovery procedure.
- [ ] Failed migration leaves the previous valid state usable.
- [ ] Golden files cover the current version and every readable older version.
- [ ] Negative tests cover malformed, unsupported, truncated, and tampered data.
- [ ] The versioning effect of future field changes is documented.
- [ ] Usage and developer documentation link to the same machine-readable
      definition.

## Review Record

The one-by-one review of each row must answer these questions:

1. What exact object or behavior does this contract protect?
2. Which parts are durable, transmitted, generated, cached, or temporary?
3. What is the canonical identity and envelope?
4. Who writes it, who reads it, and where does it live?
5. Which existing shapes must be removed before the first stable release?
6. Which fields are required, optional, computed, or forbidden?
7. How are large values, files, credentials, and untrusted input handled?
8. What changes are patch, minor, or major changes?
9. How does migration work, and what happens when it fails?
10. Which fixtures and tests prove the promise?

Each completed review updates this matrix and adds a focused contract document
or machine-readable schema. The matrix remains the summary; exact field rules
belong to the contract that owns them.

## Reference Practices

Zebflow uses Kubernetes as the primary reference for independently versioned
API families, validation, conversion, storage versions, and safe removal. It
uses Git for immutable identity and repository safety, OCI for typed bundles and
digest verification, Protocol Buffers for field compatibility discipline, S3
for object and access behavior, and PostgreSQL for transactional migration and
recovery.

These are design references, not formats to copy blindly. Zebflow keeps its own
small contracts and adopts the safety rules that match each responsibility.
