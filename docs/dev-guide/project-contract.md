# Project Contract

> Status: draft, intended to become normative for all `0.2.x` work.
>
> This document defines the durable contract for one Zebflow project. New
> features may extend this contract additively. They must not silently
> reinterpret it.

## 1. Purpose

The Project Contract exists to make one Zebflow project:

- durable
- portable
- inspectable
- versionable
- recoverable

The project must remain intelligible even when:

- a controller or managing office is unavailable
- Kubernetes communication fails
- a future release extends the project model
- a project is exported, imported, cloned, or migrated

This document is the reference for deciding whether a proposed change is:

- additive and safe
- a migration-requiring change
- a contract violation

## 2. Normative Language

The words `Must`, `Must Not`, `Should`, and `May` are used normatively.

- `Must`
  required for compliance with the contract
- `Must Not`
  prohibited by the contract
- `Should`
  strongly preferred; deviation requires explicit justification
- `May`
  allowed but optional

## 3. Core Principle

A project **Must** be reconstructible from its own project artifact set.

Controller-only hidden state **Must Not** be required for project survival.

The project contract therefore privileges:

- project-local durable data
- project-local repo state
- explicit export/import artifacts
- explicit manifests and version markers

over:

- implicit PVC assumptions
- opaque controller-only metadata
- undocumented filesystem conventions

## 4. Project Identity

A project is identified by:

- `owner`
- `project`

Within one office data root, the stable project root is:

`{data_root}/users/{owner}/{project}`

This location is part of the contract and **Must Not** be silently redefined in
`0.2.x`.

## 5. Stable Project Artifact Set

At minimum, a project is composed of three top-level durable areas:

- `repo/`
- `data/`
- `files/`

Each has a distinct meaning.

### 5.1 `repo/`

`repo/` is the git-synced authoring workspace.

It contains:

- source files
- project configuration
- documentation
- git history if present

### 5.2 `data/`

`data/` is the durable project-owned operational data area.

It contains:

- project-local databases or snapshots owned by the project
- runtime materializations owned by the platform runtime
- future portable execution artifacts if explicitly defined

### 5.3 `files/`

`files/` is the project-owned public/private file artifact area.

It contains:

- generated public outputs
- private file outputs
- static-generation results
- future object-store mirror sources if later enabled

## 6. Stable Repo Structure

The following `repo/` paths are normative in `0.2.x`:

- `repo/.git`
- `repo/pipelines/`
- `repo/docs/`
- `repo/zebflow.json`
- `repo/zeb.lock`

### 6.1 `repo/pipelines/`

`repo/pipelines/` is the unified source root for:

- pipeline definitions (`*.zf.json`)
- page templates
- components
- scripts
- related TSX or TypeScript source artifacts that are part of the project

The platform **Must** treat this as the canonical live source tree for current
project authoring.

### 6.2 `repo/docs/`

`repo/docs/` is the project documentation tree shown in Studio as `/docs`.

The platform **Must** treat it as a real tree, not a pseudo-surface.

That means documentation entries **Must** support:

- nested folders
- file create
- file save
- move
- delete

### 6.3 `repo/zebflow.json`

`repo/zebflow.json` is the git-tracked, non-sensitive project configuration
file.

It **May** be extended additively.

Existing fields **Must Not** be reinterpreted incompatibly inside `0.2.x`.

### 6.4 `repo/zeb.lock`

`repo/zeb.lock` is the git-tracked lock file for project-managed package or
library state.

It **May** be extended additively.

Its existing fields and semantics **Must** remain stable across `0.2.x`.

## 7. Stable Data Structure

The following `data/` structure is normative in `0.2.x`:

- `data/`
- `data/runtime/`
- `data/runtime/pipelines/`

### 7.1 `data/`

This is the broad project-owned durable data namespace.

Project-specific application data **May** live here when Zebflow manages it
locally.

### 7.2 `data/runtime/`

This is the platform-owned runtime materialization namespace inside the project.

It is reserved for:

- activated runtime snapshots
- runtime indexes
- future execution journals or caches explicitly defined by the platform

The platform **Must Not** use undocumented ad hoc paths outside its reserved
runtime namespace when a stable runtime-owned location already exists.

### 7.3 `data/runtime/pipelines/`

This is the current runtime materialization area for activated pipelines.

Future versions **May** add sibling runtime-owned subdirectories under
`data/runtime/`, but they **Must Not** redefine the meaning of
`data/runtime/pipelines/`.

## 8. Stable Files Structure

The following `files/` structure is normative:

- `files/public/`
- `files/private/`

### 8.1 `files/public/`

Publicly servable generated or user-managed artifacts belong here.

Examples:

- generated static pages
- public downloads
- published assets

### 8.2 `files/private/`

Non-public project-owned artifacts belong here.

Examples:

- internal exports
- private generation outputs
- later private object-store staging

### 8.3 Static Generation Rule

Any future static generation feature **Must** target either:

- `files/public/...`
- `files/private/...`

or an explicitly documented external publish backend that mirrors this logical
split.

## 9. Reserved vs Extensible Namespaces

The contract distinguishes between:

- reserved Zebflow-owned namespaces
- project-owned extensible namespaces

### 9.1 Reserved Namespaces

The following are reserved by Zebflow and **Must Not** be reinterpreted by
project business logic:

- `repo/pipelines/`
- `repo/docs/`
- `repo/zebflow.json`
- `repo/zeb.lock`
- `data/runtime/`
- `files/public/`
- `files/private/`

### 9.2 Extensible Namespaces

The project **May** add additional files and directories under:

- `repo/`
- `data/`
- `files/`

provided they do not collide with documented reserved namespaces.

### 9.3 Future System-Owned Data

If future versions add system-owned relational tables, collections, or internal
file areas, they **Must** use an explicit Zebflow-reserved namespace or prefix.

They **Must Not** appropriate ambiguous project business names such as
`content`, `comments`, `expression`, or `users` as if they were universally
platform-owned.

This rule exists so projects can model rich domain structures such as
hierarchical content, expression engines, and comment systems without later
platform upgrades colliding with those names.

## 10. Execution And Observation Envelope

Zebflow’s future execution surfaces may expand beyond classic pipeline nodes.
Examples include:

- agent tasks
- research jobs
- engineering experiments
- Python execution
- C++ execution
- static generation batches

To remain observable, every such execution surface **Should** project into the
same logical execution envelope.

Minimum envelope fields:

- `run_id`
- `owner`
- `project`
- `actor_id` or `agent_id`
- `execution_kind`
- `capability`
- `objective_ref` or intent reference
- `status`
- `started_at`
- `finished_at`
- `artifact_refs`
- `trace_ref` or trace payload
- `verification_ref` or verification payload

This does not force every execution surface to become a pipeline graph.
It does require them to remain observable in a common way.

## 11. Git Contract

The project git workspace belongs to the project.

### Rules

- If a git repository exists, export/import **Should** preserve it.
- Git history is part of project durability, not controller-only state.
- Future history views, Overleaf-style activity, or commit inspection **Must**
  build on top of the project workspace rather than inventing a parallel source
  of truth.

## 12. Export / Import Contract

Project export/import **Must** be explicit and versioned.

### Minimum artifact shape

- `project.bundle`
  - `repo/`
  - `data/`
  - `manifest`
- `project.files`
  - `files/public/`
  - `files/private/`

### Manifest minimum fields

- contract schema version
- owner/project identity
- source office id
- source managing office id if present
- export timestamp
- current runtime profile snapshot
- placement snapshot
- checksums or file counts

### Rules

- Export **Must** be project-scoped, not office-wide.
- Import **Must** be deterministic.
- Import **Must Not** require the original controller to still exist.
- Export/import **Must** distinguish non-sensitive config from secrets or
  rebinding material.

## 13. Sensitive Material Boundary

The project contract **Must** keep a clean boundary between:

- project-owned durable artifacts
- environment-owned secrets and credentials

By default:

- `repo/`, `data/`, and `files/` are project-owned artifacts
- secrets and credential values are environment-owned unless explicitly exported
  through a documented mechanism

This rule exists so portability does not silently become credential sprawl.

## 14. Additive Extension Rule

Within `0.2.x`, new releases **May**:

- add new files
- add new directories
- add new manifest fields
- add new `zebflow.json` fields
- add new execution envelope fields

Within `0.2.x`, new releases **Must Not**:

- silently rename normative paths
- silently change the meaning of normative paths
- silently require a controller-only metadata source to reconstruct the project
- silently reinterpret an existing manifest field incompatibly

If a change cannot satisfy those rules, it is a migration event and **Must** be
treated as such.

## 15. Compatibility Examples

### Additive And Safe

- add `repo/pipelines/shared/`
- add `data/runtime/jobs/`
- add `manifest.artifact_checksums`
- add `zebflow.json.runtime.ingress_mode`

### Breaking Unless Migrated Explicitly

- moving all templates out of `repo/pipelines/` to another root
- reusing `files/public/` for private outputs
- changing `data/runtime/` to mean arbitrary project data
- requiring the controller database to rebuild `repo/`

## 16. Failure And Recovery Semantics

If office-to-office communication fails:

- the project artifact set **Must** remain intact on disk
- runtime execution may degrade
- management operations may fail
- the project contract remains valid

If Kubernetes orchestration fails but project storage remains intact:

- the project **Must** remain recoverable from its artifact set
- export/import **Should** still be possible from the durable project root

## 17. Non-Goals

This contract does not define:

- app-level business authentication
- business database schema chosen by the project
- scheduling policy between offices
- office governance semantics

Those belong to other contracts.

## 18. Example Tree

```text
{data_root}/users/superadmin/example/
├── repo/
│   ├── .git/
│   ├── pipelines/
│   ├── docs/
│   ├── zebflow.json
│   └── zeb.lock
├── data/
│   └── runtime/
│       └── pipelines/
└── files/
    ├── public/
    └── private/
```

## 19. Compliance Test

A project is compliant with this contract if:

1. its normative artifact set is present and coherent
2. reserved namespaces keep their documented meaning
3. export/import can describe the project explicitly
4. future additive changes do not reinterpret prior contract surfaces

That is the baseline the rest of Zebflow must preserve.
