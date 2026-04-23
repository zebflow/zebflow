# Platform Data Structure

## Purpose

This document defines the **long-term Zebflow platform storage contract**.

It is optimized for:

- multi-user management
- multi-project management
- multi-office control
- runtime management
- marketplace management

It is intentionally **not** the contract for:

- project business data
- project content payloads
- pipeline execution payloads
- arbitrary app-local storage

Those belong in project storage, not in the platform catalog.

## Core Principle

Zebflow storage is split into two layers:

1. **Platform control-plane storage**
   - one global SQLite catalog
   - path: `{data_root}/platform/catalog.db`
   - purpose: identities, ownership, access, office topology, runtime placement, marketplace authority, operations

2. **Project storage**
   - isolated per project under:
     - `{data_root}/users/{owner}/{project}/`
   - purpose: source repo, runtime state, project files, project-local DB engines

This means:

- `catalog.db` stores platform truth
- project business/runtime payload does **not** belong in `catalog.db`
- `repo/zebflow.json` is the durable project contract
- project-local mutable data lives under `data/` and `files/`

## Security Principles

This schema is designed with **hypersecure by default** rules:

1. **Stable internal ids first**
   - all core entities use internal stable ids
   - slugs are external-facing names, not long-term relational identity

2. **Foreign keys on**
   - relational ownership is enforced in SQLite
   - no orphan rows by design

3. **Auth separated from identity**
   - user profile and password/auth material are not mixed casually

4. **Secrets are minimized**
   - the catalog stores only what must be stored
   - secrets should be encrypted or externally managed later, but schema must keep them isolated already

5. **Soft disable before delete**
   - important entities are disabled/deactivated first
   - destructive deletion is exceptional, not normal flow

6. **JSON only at the edges**
   - JSON is allowed for extensible payloads
   - core control-plane facts must stay relational

7. **Forward-only migrations**
   - schema evolves through explicit ordered migrations
   - no silent drift

## Project File Layout

Each project is rooted at:

- `{data_root}/users/{owner}/{project}/`

Main directories:

| Path | Meaning |
| --- | --- |
| `repo/` | project source of truth, Git workspace |
| `data/` | project runtime state |
| `files/` | project file storage |

Important derived paths:

| Path | Meaning |
| --- | --- |
| `repo/.git` | project Git metadata |
| `repo/pipelines/` | pipelines, templates, scripts |
| `repo/docs/` | project docs |
| `repo/zebflow.json` | durable project config contract |
| `data/runtime/` | runtime state root |
| `data/runtime/pipelines/` | active pipeline runtime snapshots |
| `data/local.db` | project-local SQLite runtime DB |
| `data/sekejap/` | project-local Sekejap store |
| `files/public/` | public project file payloads |
| `files/private/` | private project file payloads |

## Platform SQLite

### File

- `{data_root}/platform/catalog.db`

### Runtime mode

- SQLite
- WAL journaling
- `PRAGMA synchronous=NORMAL`
- explicit schema migrations

### Purpose

This DB stores **platform control-plane metadata**, not project business data.

It is the authoritative storage for:

- users
- auth credentials for local platform login
- projects
- project membership
- project credentials
- project DB connection registry
- pipeline catalog metadata
- offices and nodes
- runtime placement
- project operations
- marketplace authorities
- publishers
- marketplace tokens
- packages and published versions
- platform marketplace browsing sources

## Migration Contract

The schema must evolve through a migration table:

### `schema_migrations`

| Column | Meaning |
| --- | --- |
| `version` | ordered migration version, primary key |
| `name` | migration label |
| `applied_at` | applied timestamp |

Rules:

- startup must ensure `schema_migrations` exists
- pending migrations run in order
- migrations are forward-only
- no schema change is allowed outside the migration path

## Locked Platform Schema

## Identity and Access

### `users`

Stable internal user identity.

| Column | Meaning |
| --- | --- |
| `user_id` | primary key |
| `user_slug` | stable public/user-facing identifier, unique |
| `role` | platform role |
| `display_name` | human-facing name |
| `git_name` | default Git author name |
| `git_email` | default Git author email |
| `status` | active / disabled / archived |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `user_id`
- unique: `user_slug`

### `user_local_auth`

Local platform authentication material.

| Column | Meaning |
| --- | --- |
| `user_id` | primary key, foreign key to `users` |
| `password_hash` | hashed password |
| `password_alg` | hash algorithm identifier |
| `password_updated_at` | last password update timestamp |

Rules:

- auth must remain separate from profile data
- future auth methods can expand beside this table without changing `users`

## Project Management

### `projects`

Stable internal project identity.

| Column | Meaning |
| --- | --- |
| `project_id` | primary key |
| `owner_user_id` | foreign key to `users` |
| `project_slug` | stable project slug |
| `title` | display title |
| `description` | display description |
| `status` | active / disabled / archived |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `project_id`
- unique: `(owner_user_id, project_slug)`

### `project_members`

Project membership and coarse access control.

| Column | Meaning |
| --- | --- |
| `project_id` | foreign key to `projects` |
| `user_id` | foreign key to `users` |
| `role` | owner / admin / editor / viewer style role |
| `joined_at` | membership timestamp |
| `created_by_user_id` | foreign key to `users` |

Constraints:

- primary key: `(project_id, user_id)`

Notes:

- this is the stable core membership layer
- richer policy systems can exist later, but should not replace this basic contract
- richer policy tables are secondary overlays, not the primary long-term access contract

### `project_credentials`

Per-project external credentials and secret references.

| Column | Meaning |
| --- | --- |
| `credential_id` | primary key |
| `project_id` | foreign key to `projects` |
| `kind` | git / openai / db / other |
| `title` | display title |
| `secret_json` | secret payload |
| `notes` | optional notes |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `credential_id`

### `project_db_connections`

Stable DB connection registry per project.

| Column | Meaning |
| --- | --- |
| `connection_id` | primary key |
| `project_id` | foreign key to `projects` |
| `connection_slug` | route-safe slug |
| `connection_label` | display label |
| `database_kind` | sqlite / postgresql / mysql / sekejap / etc |
| `credential_id` | nullable foreign key to `project_credentials` |
| `config_json` | kind-specific config payload |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `connection_id`
- unique: `(project_id, connection_slug)`

### `pipeline_meta`

Stable pipeline catalog metadata per project.

| Column | Meaning |
| --- | --- |
| `pipeline_id` | primary key |
| `project_id` | foreign key to `projects` |
| `file_rel_path` | pipeline file path in repo |
| `name` | stable pipeline name |
| `title` | display title |
| `description` | description |
| `trigger_kind` | trigger type |
| `current_hash` | current working tree hash |
| `active_hash` | active runtime hash |
| `activated_at` | activation timestamp |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `pipeline_id`
- unique: `(project_id, file_rel_path)`

## Multi-Office and Runtime Management

### `offices`

Stable office identity for controller/office topology.

| Column | Meaning |
| --- | --- |
| `office_id` | primary key |
| `office_slug` | unique office slug |
| `label` | display label |
| `office_kind` | controller / office |
| `base_url` | advertised office URL |
| `status` | active / degraded / disabled |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `office_id`
- unique: `office_slug`

### `office_nodes`

Registered runtime nodes inside offices.

| Column | Meaning |
| --- | --- |
| `node_id` | primary key |
| `office_id` | foreign key to `offices` |
| `label` | display label |
| `base_url` | node base URL |
| `status` | health/availability state |
| `capabilities_json` | node capabilities payload |
| `registered_at` | registration timestamp |
| `last_heartbeat_at` | last heartbeat timestamp |

Constraints:

- primary key: `node_id`

### `project_runtime_placements`

Authoritative runtime placement for each project.

| Column | Meaning |
| --- | --- |
| `project_id` | primary key, foreign key to `projects` |
| `runtime_mode` | shared / dedicated / other stable runtime mode |
| `target_office_id` | nullable foreign key to `offices` |
| `target_node_id` | nullable foreign key to `office_nodes` |
| `resource_profile` | stable resource class |
| `desired_replicas` | target replica count |
| `effective_state` | current runtime state |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Rules:

- one authoritative placement row per project
- target node is optional
- target office is the normal placement anchor

### `project_operations`

Long-running control-plane operations for projects.

| Column | Meaning |
| --- | --- |
| `operation_id` | primary key |
| `project_id` | foreign key to `projects` |
| `kind` | transfer / import / export / deploy / other |
| `status` | pending / running / failed / completed |
| `source_office_id` | nullable foreign key to `offices` |
| `target_office_id` | nullable foreign key to `offices` |
| `artifact_rel_path` | optional artifact path |
| `artifact_sha256` | optional artifact hash |
| `artifact_bytes` | optional artifact size |
| `error_message` | failure text |
| `retry_count` | retry count |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Rules:

- this is control-plane audit, not business logging
- retention policy should be defined separately

## Marketplace Management

Marketplace authority remains **project-hosted**, but explicitly modeled.

### `marketplace_authorities`

Project-hosted marketplace producer authorities.

| Column | Meaning |
| --- | --- |
| `authority_id` | primary key |
| `host_project_id` | unique foreign key to `projects` |
| `enabled` | whether producer mode is enabled |
| `public_base_url` | public producer API base |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `authority_id`
- unique: `(host_project_id)`

### `marketplace_publishers`

Stable publisher identities inside one marketplace authority.

| Column | Meaning |
| --- | --- |
| `publisher_pk` | primary key |
| `authority_id` | foreign key to `marketplace_authorities` |
| `publisher_id` | stable public publisher id |
| `display_name` | publisher display name |
| `publisher_url` | stable public publisher URL |
| `email` | publisher contact email |
| `description` | description |
| `icon_url` | icon URL |
| `website_url` | website URL |
| `enabled` | active / disabled |
| `created_by_user_id` | foreign key to `users` |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `publisher_pk`
- unique: `(authority_id, publisher_id)`

### `marketplace_tokens`

Revocable credentials bound to one publisher identity.

| Column | Meaning |
| --- | --- |
| `token_id` | primary key |
| `authority_id` | foreign key to `marketplace_authorities` |
| `publisher_pk` | foreign key to `marketplace_publishers` |
| `title` | display title |
| `secret_hash` | token secret hash |
| `scope_read` | read permission flag |
| `scope_publish` | publish permission flag |
| `scope_manage` | manage permission flag |
| `expires_at` | optional expiry |
| `last_used_at` | last used timestamp |
| `revoked_at` | revoked timestamp |
| `created_by_user_id` | foreign key to `users` |
| `revoked_by_user_id` | nullable foreign key to `users` |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Rules:

- token is not the identity
- publisher is the identity
- token is only a revocable credential

### `marketplace_packages`

Published package-level metadata.

| Column | Meaning |
| --- | --- |
| `package_pk` | primary key |
| `authority_id` | foreign key to `marketplace_authorities` |
| `publisher_pk` | foreign key to `marketplace_publishers` |
| `package_id` | stable package id within one authority |
| `asset_kind` | project / pipeline / template / script / folder |
| `title` | display title |
| `description` | description |
| `visibility` | public / private / unlisted |
| `tags_json` | extensible tags payload |
| `status` | active / disabled / archived |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Constraints:

- primary key: `package_pk`
- unique: `(authority_id, package_id)`

### `marketplace_package_versions`

Immutable published versions of packages.

| Column | Meaning |
| --- | --- |
| `package_pk` | foreign key to `marketplace_packages` |
| `version` | package version |
| `source_project_id` | nullable foreign key to `projects` |
| `source_kind` | source kind |
| `source_ref` | source reference |
| `artifact_rel_path` | artifact path under platform storage |
| `artifact_sha256` | artifact content hash |
| `manifest_json` | version manifest payload |
| `created_by_user_id` | foreign key to `users` |
| `created_at` | created timestamp |

Constraints:

- primary key: `(package_pk, version)`

### `platform_marketplace_sources`

Platform-home browsing sources for installing apps/packages.

| Column | Meaning |
| --- | --- |
| `source_id` | primary key |
| `owner_user_id` | foreign key to `users` |
| `title` | display title |
| `base_url` | remote marketplace API URL |
| `read_token_secret` | optional read credential or credential reference |
| `enabled` | active / disabled |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Rules:

- this is for browsing/installing from Home
- this is not the producer authority itself

## What Does Not Belong In The Platform Catalog

The following should not be treated as long-term core catalog contract:

- project source files
- project runtime business data
- project file payloads
- general pipeline execution outputs
- arbitrary assistant/session history
- large blob content
- project-local DB internals

Those belong in project storage, not in platform SQLite.

## Stability Rules

This schema is considered long-term only if these rules are kept:

1. top-level platform responsibilities stay narrow
   - users
   - projects
   - offices
   - runtime management
   - marketplace management

2. project contract remains in `repo/zebflow.json`

3. no new core platform domain is added casually

4. every schema change goes through explicit migration

5. slugs may change, ids do not

6. tokens rotate, publisher identities do not

7. project-hosted marketplace authority may evolve operationally, but publisher/package identity remains stable

## Lifecycle Rules

The control-plane catalog is **soft-disable first**.

Destructive delete is not the normal path for long-lived entities.

### Users

- default operation: `status = disabled`
- hard delete is exceptional and only safe when no owned projects, memberships, or audit-critical references remain

### Projects

- default operation: `status = archived` or `status = disabled`
- hard delete is exceptional and only safe after runtime placement, marketplace authority, credentials, and project storage cleanup are complete

### Offices

- default operation: `status = disabled`
- hard delete is only safe after placements and node registrations have been drained

### Marketplace Authorities

- default operation: `enabled = false`
- disabling the authority must preserve publisher, package, and version history

### Marketplace Publishers

- default operation: `enabled = false`
- disabling a publisher must preserve package attribution and published history

### Marketplace Tokens

- default operation: `revoked_at = <timestamp>`
- tokens are revocable credentials, not durable identity

### Marketplace Packages

- default operation: `visibility = private` or `status = archived`
- historical versions and attribution should remain intact

## Retention Rules

The platform catalog is not an unbounded event store.

Retention rules:

- `project_operations`
  - keep recent operational history in the catalog
  - archive or prune old completed rows by policy
- `office_nodes`
  - keep the latest registration and heartbeat state
  - do not accumulate unbounded heartbeat history in the catalog
- future audit/event tables
  - must define explicit retention at creation time
- large execution history, pipeline outputs, and app business logs
  - do not belong in the platform catalog

Operational principle:

- keep the catalog small, relational, and durable
- move large or high-churn histories outside the control-plane DB

## Cutover Rules

The schema contract from this line onward is migration-backed.

### Fresh Data Line

- major contract cuts such as `0.4.x` should use a fresh PVC / fresh data root by default
- do not silently reuse pre-contract platform state from older experimental lines

### Pre-Contract State

- pre-contract catalogs may be inspected or migrated deliberately
- they are not assumed safe for automatic in-place production reuse

### Deployment Rule

- generated deployment manifests must point new workloads at the new data root / PVC line
- production rollout should validate:
  - catalog boot on a clean volume
  - migration boot on a known current local-dev catalog
  - marketplace default base URL wiring
  - foreign key integrity after boot

## Summary

The intended long-term Zebflow storage model is:

- **hypersecure**
  - stable ids
  - separated auth
  - revocable tokens
  - FK-enforced ownership

- **hyperstable**
  - explicit migration table
  - narrow platform scope
  - durable project contract

- **hyperlightweight**
  - one SQLite catalog
  - project payload outside the catalog
  - JSON only where extensibility is actually needed

This is the schema direction to treat as the main Zebflow platform contract.
