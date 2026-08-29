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
| 2 | P0 | [Instance directory](./instance-directory.md) | Project repository, data, files, runtime, and generated paths | Policy only | `src/platform/adapters/file/mod.rs`, project services | Review | Applied and live-verified: `data/` splits into store, cache, recovery, and logs, with a fifth ratified tier, installed (`data/hub/`), holding `nodes/` and `rwe-libraries/`; a pre-tier project migrates transparently on first touch, atomically, idempotently. Remaining: the `data/logs/` retention window (`data/nodes/` → `data/hub/nodes/` shipped, first-touch atomic move) |
| 3 | P0 | [Project configuration](./kinds/project-configuration/README.md) | `zebflow.yaml` | `zebflow.com/v1` `ProjectConfiguration` | `src/contracts/kinds/project_configuration.rs`, `src/platform/services/project_config.rs` | Frozen | Reference contract complete; preserve its schema and use its freeze evidence for every later kind |
| 4 | P0 | [Pipeline source](./kinds/pipeline/README.md) | Pipeline graph and `.zf.json` source | `zebflow.com/v1` `Pipeline` | `src/contracts/kinds/pipeline.rs`, `src/pipeline/model.rs`, `src/platform/services/project.rs` | Frozen | Preserve v1 and use its contract/runtime separation and recoverable activation sequence for later executable kinds |
| 5 | P0 | [Dependency lock](./kinds/dependency-lock/README.md) | `zeb.lock` | `zebflow.com/v1` `DependencyLock` | `src/contracts/kinds/dependency_lock.rs`, dependency service, RWE library service, node registry | Frozen | Strict namespaces, discovery, repair, atomic Hub installation, migration, and two-instance transfer implemented and tested; source vocabulary restructured 2026-08-27 pre-release (`hub.local/public/static`, `direct.npm/file`, `project`) — code caught up 2026-08-28: the validator enum accepts only the new words (dead words refuse and the repair/refresh path quarantines and regenerates from requested state), lock writers record the real channel and package coordinate, entries resolve at `data/hub/rwe-libraries/{package_id}/…`, bundle keys are the dot-form coordinate, and the seeded official source order is flipped (static first) |
| 6 | P0 | [Node definition](./kinds/node-definition/README.md) | Shared native, composite, and WASM node interface | `zebflow.com/v1` `NodeDefinition` | `src/contracts/kinds/node.rs`, `src/pipeline/model.rs`, node registry | Frozen | Preserve the implementation-neutral v1 interface; package release, artifacts, installation, and rollback remain owned by `NodeBundle` |
| 7 | P0 | [Node bundle](./kinds/node-bundle/README.md) | Installable one-node and multi-node source | `zebflow.com/v1` `NodeBundle` | `src/contracts/kinds/node.rs`, `src/platform/model.rs`, `src/platform/services/node_registry.rs` | Frozen | Kind ownership by consumption scope, run bindings, per-node WASM exports, scoped credentials, artifact existence, digest enforcement, local and Hub install through one review, and carried node interfaces are implemented and verified live; install remains recoverable rather than atomic by design |
| 8 | P0 | [File reference](./kinds/file-ref/README.md) | FileRef passed between native, composite, and WASM nodes | `__zf_type=file_ref` | `src/pipeline/nodes/basic/file_ref.rs` | Review | Spec sealed 2026-08-29: eleven required fields, `ref` opaque to all but its backend, eight `kind` values, digest `sha256:` + 64 hex. Code catch-up owed: stop emitting `path`, `name`, `content_type`, and `url`; the two help docs disagree with each other and with the contract (`authoring.md` says `content_type`, `dsl.md` says `mime`). Still open: `trust` has one value, temporary cleanup has no writer, remote streaming undefined |
| 9 | P0 | [Project file storage](./kinds/zebfs-acl/README.md) | ZebFS paths, object identity, lifecycle, and the ACL document | `zebflow.com/v1` `ZebFsAcl` | `src/zebfs/`, `src/platform/services/zebfs_acl.rs` | Review | Spec sealed 2026-08-29: `private`/`public_read`, `object`/`prefix`, longest match wins, missing or unreadable means private. Code catch-up owed: the document gains the standard envelope, with the bare `{rules}` shape accepted on read and rewritten on the next change because an ACL records a human decision and cannot be regenerated. Still open: who may change a rule, whether deleting an object removes its rule, whether a public prefix implies a public listing; and from before, multi-object transactions and cleanup ownership |
| 10 | P0 | [Project data stores](./kinds/database-schema/README.md) | Platform SQLite catalog; the project's own store and how its structure and starting rows live in `repo/` | SQLite migration `15`; `zebflow.com/v1` `DatabaseSchema` | `src/platform/adapters/data/sqlite.rs`, `src/platform/sekejap.rs`, `src/platform/policy/package.rs` | Review | Spec sealed 2026-08-29: own store only (sekejap, sqlite; Postgres/MySQL capture deferred past stabilization), `000_structure.sql` platform-owned beside authored row files, explicit save, publish refuses a stale structure, replay at creation only; code catch-up owed: the save action and the publish staleness check (load side already ships). Still open for the store itself: backup/recovery, poisoned-lock recovery |
| 11 | P0 | [Project transfer](./kinds/project-bundle/README.md) | One envelope for every whole-project movement: declared classes, staged replace-per-class import, two-import identity rule | `zebflow.com/v1` `ProjectBundle` | `src/platform/services/project_transfer.rs`, `src/contracts/kinds/project_bundle.rs` | Review | Spec sealed 2026-08-28; code caught up 2026-08-28: strict envelope kind with golden fixture and negative tests; the transfer archive is the envelope and speaks classes (`store` is `data/store/` only — cache, hub, logs, and recovery stay home; `direct.*` lock bytes travel in `carried-dependencies/`); import verifies at staging (entry path safety, per-class tree digests, carried bytes against the archived lock) before one recovery swap per class, with reverse-swap rollback exposed; identity is provenance (repo-owned `zebflow.yaml`/`zeb.lock` retarget to the destination); superadmin platform import creates projects, auto-initiating the store for `repo`-only archives; unresolved `n.function.call` targets are the dependency report's fifth family. Remaining: the hub `project_bundle` asset channel still ships the HubPackage layout rather than this envelope, and remote-office import destination and remote rollback stay unbuilt (row 14) |
| 12 | P0 | Runtime synchronization | Retired 2026-08-29. A project moves between any two Zebflow instances as a [`ProjectBundle`](./kinds/project-bundle/README.md), in any direction. The master-to-worker copier in `src/infra/execution/sync/` (added 2026-04-10 in `fa3cb6d`) is a one-directional duplicate of that, with no verification and a destination wipe; it has no contract and nothing depends on it | none | `src/infra/execution/sync/`, `src/contracts/kinds/runtime_bundle.rs` | Retired | Code removal owed: the `RuntimeBundle` contract kind, the sync module, and the materialize routes |
| 13 | P1 | Reactive web protocol | RWE compile, render, event, and error messages | `rwe.v1` | `src/rwe/protocol.rs` | Partial | Separate source, compiled artifact, and wire protocol contracts and test version rejection |
| 14 | P1 | [Hub package](./kinds/hub-package/README.md) | One published release: what installing it requires. Presentation, publisher identity, and provenance live in mutable storage beside it | `zebflow.com/v1` `HubPackage`; release version is independent | `src/contracts/kinds/hub_package.rs`, `src/platform/services/hub.rs`, platform models | Candidate | `spec.layout` aged past the week it was written, with cross-layout installs proven and one reader behind publish and both install gates; before Frozen, decide or record as deliberate each "Still to review" edge (unmapped install refusal codes, per-release retraction, uninstall destinations, layout-vs-carried-configuration disagreement); contract-code gap (2026-08-27) closed 2026-08-28: `services/hub-local/` (seed-only blessed shelf) and `services/hub-public/` (the one publish target, publisher/token/grant rows, created behind service enablement) are two roots of one store machinery, and no publish surface holds the local store handle; remaining: multi-office placement provisioning of `hub-public` on a remote state-owning office is not built |
| 14b | P0 | [Distribution](./distribution.md) | How a resource leaves one instance and arrives at another | Policy only | `docs/contracts/distribution.md`, `src/platform/services/hub.rs`, `src/platform/policy/package.rs` | Review | Decide git-source installation, promotion into the curated namespace, and whether official content is locked; apply each decision to every channel at once |
| 14c | P1 | [Hub repository index](./kinds/hub-repository-index/README.md) | `zebflow-repository.json`: what a static serving offers, where each release document sits, and which bytes are the right ones | `zebflow.com/v1` `HubRepositoryIndex` | `src/contracts/kinds/hub_repository_index.rs`, `src/platform/services/hub_repository.rs` | Candidate | Live static pass proven — ordered resolution across seeded sources, digest refusal by hash, review refusal with no project created; before Frozen, let the days-old kind read one real repository not produced by this codebase |
| 14d | P1 | [Offices](./offices.md) | Whether two instances are related at all: what an office keeps and accepts, the controller's three verbs, joining, break-glass, and exit | Not a kind; a top-level contract | `src/platform/services/cluster/`, `src/platform/web/mod.rs` registration routes | Review | Spec sealed 2026-08-29 (institutions/projects/data/execution/version kept; login and placement accepted; controller never on the data path; owner mapping at the door; host break-glass; defined exit). Code catch-up owed: workers currently skip their own blessed seed, the join token is a shared environment secret rather than a per-office revocable one, local-account disable/re-enable and owner mapping do not exist, and `apply_bundle` wipes a project repo on every sync |
| 15 | P1 | Public and cluster protocols | HTTP API, webhook, MCP, controller-worker, and service messages | Mostly route-based or unversioned | Platform routes, MCP, cluster services | Critical | Group protocol families, define compatibility windows, and version durable wire envelopes |
| 16 | P2 | Invocation and temporary state | Invocation records, traces, node payloads, caches, and temporary files | No common version | Pipeline runtime and invocation services | Critical | Make it bounded and disposable; version only records that may survive a runtime upgrade |
| 17 | P1 | [Library definition](./kinds/rwe-library-manifest/README.md) | Installed library manifests, versions, integrity, and project enablement | `zebflow.com/v1` `RweLibraryManifest` | `src/contracts/kinds/rwe_library_manifest.rs`, `src/platform/services/library.rs`, library catalog files | Review | The validator enforces the settled spec — `offline`/`hub` sources only, required sha256 integrity, non-zero sizes, descending entries — the blessed manifests carry real digests kept honest by recompute tests, and the golden fixture round-trips; remaining before a freeze judgement: the manifest-less `zebflow.preact` package recorded in the kind's Open section |
| 18 | P1 | [Map publish manifest](./kinds/map-publish-manifest/README.md) | Published layer identity, source, generated artifacts, styles, and caches | `zebflow.com/v1` `MapPublishManifest`; generated artifacts are private formats | project MapServer service, `src/mapserver/publish/` | Review | Spec sealed 2026-08-29: `allowed_properties` closed by default — empty means geometry only, no wildcard, the UI's select-all writes real column names. Code catch-up owed, security-bearing: `prune_feature_properties` (`src/mapserver/resolve/artifact.rs:106`) returns every property when the list is empty, the opposite of the contract; `path` is stored with a leading slash by `web/mod.rs:14867` and without by the node, which is why web-published layers fail to serve; `LayerRecord` in `mapserver_crud.rs:41` hand-mirrors `MapserverLayerRecord` and has already drifted (`column_stats_path`). Still open: artifact rebuild is untested, deleted-source behaviour undefined, `style` has no schema |

## Verified Audit, 2026-08-14

The source audit and an independent read-only review reached the same release
assessment:

- Zebflow is ready for demonstrations and controlled single-node development.
- A pinned single-node production deployment is possible with external backups,
  trusted inputs, and conservative payload sizes.
- Zebflow is not ready to declare a stable v1 storage and transfer contract.

The full library suite passed: 420 default tests and three tests ignored by
default, run separately. This proves the currently tested behavior. Focused
failure tests now cover atomic-write interruption and forward-version rejection
for the contracts listed in the implementation record below. It does not yet
prove multi-file rollback, full package rollback, or every tamper boundary.

The strongest verified mechanisms are the central strict contract decoder,
transactional platform SQLite migrations, path-confined atomic ZebFS object
writes, FileRef size and digest checks, graph and node-definition validation,
remote Hub artifact hash checks, and staged map artifact publishing.

The highest remaining risks are multi-file operations without rollback,
destructive project import and runtime materialization, non-atomic node and Hub
installation, temporary FileRef cleanup, poisoned Sekejap locks, and repeated
cloning of large JSON node payloads. Other authoritative document formats still
need to adopt the shared strict reader and durable writer as their rows are
reviewed.

## Implementation Record, 2026-08-14

The contract kernel is implemented in `src/contracts/`. Generic atomic
byte replacement remains in `src/infra/io/durable.rs`:

- Same-directory temporary writes, file sync, atomic replacement, and parent
  directory sync on Unix.
- One closed kind registry and one `zebflow.com/v1` envelope.
- Strict root validation before typed deserialization or writing.
- Missing files are distinct from malformed, unreadable, and unsupported files.
- Injected partial-write tests prove that a failed replacement preserves the
  previous valid file and removes the temporary file.

The foundation currently protects these paths:

- `zebflow.yaml` project configuration.
- The existing RWE-only `zeb.lock`; its general RWE and node contract remains
  under review and must not be called frozen yet.
- ZebFS local objects and the ACL manifest.
- Pipeline source and active runtime snapshots.
- Installed node files and Hub artifact files at the individual-file boundary.

Canonical strict readers now cover project configuration, dependency locks,
ACL manifests, pipeline graphs, node definitions and bundles, database schemas,
project and project placements, Hub packages, library manifests, and map publish
manifests. Hub artifacts and FileRefs verify stored digests before use.

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
