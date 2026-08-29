# Registered Contract Kinds

This folder defines the concrete contracts governed by the shared rules in
`docs/contracts/`.

Every registered kind has one folder. Its `README.md` defines the permanent
structure, ownership, validation, lifecycle, and freeze status. A separate
migration document is used only when an existing format must be converted
explicitly.

## Review Order

| No. | Kind | Representation | Status |
| ---: | --- | --- | --- |
| 1 | [`ProjectConfiguration`](./project-configuration/README.md) | Envelope, persisted file | Frozen |
| 2 | [`Pipeline`](./pipeline/README.md) | Envelope, persisted file | Frozen |
| 3 | [`DependencyLock`](./dependency-lock/README.md) | Envelope, persisted file | Frozen |
| 4 | [`NodeDefinition`](./node-definition/README.md) | Envelope, normalized node interface | Frozen |
| 5 | [`NodeBundle`](./node-bundle/README.md) | Envelope, package source and transfer | Frozen |
| 6 | [`FileRef`](./file-ref/README.md) | Inline payload | Pending |
| 7 | [`ZebFsAcl`](./zebfs-acl/README.md) | Envelope, persisted file | Pending |
| 8 | [`DatabaseSchema`](./database-schema/README.md) | Directory convention, plain statement files (no envelope) | Review |
| 9 | [`ProjectBundle`](./project-bundle/README.md) | Envelope, transfer archive | Review |
| 10 | [`RuntimeBundle`](./runtime-bundle/README.md) | Envelope, runtime transfer | Pending |
| 11 | [`HubPackage`](./hub-package/README.md) | Envelope, stored and transferred package | Candidate |
| 12 | [`RweLibraryManifest`](./rwe-library-manifest/README.md) | Envelope, persisted library source | Review |
| 13 | [`MapPublishManifest`](./map-publish-manifest/README.md) | Envelope, persisted file | Pending |
| 14 | [`InvocationRecord`](./invocation-record/README.md) | Database record | Pending |
| 15 | [`ProjectManifest`](./project-manifest/README.md) | Reserved | Pending decision |
| 16 | [`RweSource`](./rwe-source/README.md) | Source file, persisted | Pending |
| 17 | [`Credential`](./credential/README.md) | Database record | Pending |
| 18 | [`OfficeTopology`](./office-topology/README.md) | Envelope, persisted | Pending |
| 19 | [`HubRepositoryIndex`](./hub-repository-index/README.md) | Envelope, transferred index | Candidate |

## Required Contents

Before a kind can be marked frozen, its folder must state:

1. purpose and owner
2. exact representation and example
3. disk, database, transfer, and runtime boundaries
4. required fields and intentional extension points
5. semantic validation and size limits
6. concurrency, atomicity, and recovery behavior
7. security and secret-handling rules
8. compatibility and migration rules
9. authoritative implementation files
10. tests proving every boundary
