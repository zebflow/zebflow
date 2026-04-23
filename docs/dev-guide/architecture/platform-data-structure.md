# Platform Data Structure

## Formal Definition

Zebflow data storage is split into two layers:

1. **Platform metadata storage**
   - one global SQLite catalog
   - path: `{data_root}/platform/catalog.db`
   - purpose: users, projects, credentials, DB connections, policies, runtime placement, marketplace metadata, Git-adjacent project metadata, operation history

2. **Project runtime storage**
   - isolated per project under:
     - `{data_root}/users/{owner}/{project}/`
   - purpose: source repo, runtime state, files, project-local DB engines

This means:
- `platform/catalog.db` is the authoritative platform metadata database
- project data does **not** live inside `catalog.db`
- each project has its own filesystem root, and may also have its own local runtime SQLite DB

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

### Purpose

This DB stores **platform metadata**, not project business data.

It is the authoritative storage for:
- users
- projects
- credentials
- DB connections
- pipeline catalog metadata
- project access/policy state
- worker registry
- project runtime placement
- transfer/import/export operations
- MCP sessions
- marketplace metadata
- pipeline invocation logs

## SQLite Tables

### `users`

Platform users and their Git identity defaults.

| Column | Meaning |
| --- | --- |
| `owner` | user id / owner id, primary key |
| `role` | platform role |
| `git_name` | Git author name for this platform user |
| `git_email` | Git author email for this platform user |
| `password` | stored password/auth payload |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

### `projects`

Platform-known projects.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project)`

### `project_credentials`

Per-project credentials such as OpenAI, Git, DB, and other secrets.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `credential_id` | stable credential id |
| `title` | display title |
| `kind` | credential kind |
| `secret_json` | secret payload JSON |
| `notes` | optional notes |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, credential_id)`

### `project_db_connections`

Per-project database connections exposed in Studio.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `connection_id` | stable internal id |
| `connection_slug` | stable route slug |
| `connection_label` | display label |
| `database_kind` | `sqlite`, `postgresql`, `mysql`, `sekejap`, etc. |
| `credential_id` | optional linked credential |
| `config_json` | kind-specific config payload |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, connection_id)`

### `project_marketplace_repositories`

Per-project configured external marketplace repositories.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `repository_id` | stable repository id |
| `title` | display title |
| `base_url` | exact marketplace API base |
| `remote_owner` | remote owner fallback |
| `remote_project` | remote project fallback |
| `read_token` | read token for remote access |
| `enabled` | whether repo is active |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, repository_id)`

### `marketplace_asset_packages`

Marketplace package-level metadata.

| Column | Meaning |
| --- | --- |
| `package_id` | package id, primary key |
| `publisher_owner` | publisher owner |
| `asset_kind` | package kind |
| `title` | display title |
| `description` | package description |
| `visibility` | public/private visibility |
| `tags_json` | tag list JSON |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

### `marketplace_asset_versions`

Immutable published versions for one package.

| Column | Meaning |
| --- | --- |
| `package_id` | package id |
| `version` | version string |
| `publisher_owner` | publisher owner |
| `source_owner` | source project owner |
| `source_project` | source project slug |
| `source_kind` | publish source kind |
| `source_ref` | source reference |
| `artifact_rel_path` | artifact file path relative to storage root |
| `artifact_sha256` | content hash |
| `manifest_json` | published manifest JSON |
| `created_at` | created timestamp |

Primary key:
- `(package_id, version)`

### `marketplace_tokens`

Marketplace API tokens.

| Column | Meaning |
| --- | --- |
| `token_id` | token id, primary key |
| `owner` | owner |
| `title` | display title |
| `secret_hash` | hashed token secret |
| `scopes_json` | scopes list JSON |
| `expires_at` | expiry timestamp |
| `last_used_at` | last used timestamp |
| `revoked_at` | revoked timestamp |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

### `pipeline_meta`

Pipeline catalog rows for one project.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `file_rel_path` | pipeline file path in repo |
| `name` | stable pipeline name |
| `title` | display title |
| `virtual_path` | registry/path presentation value |
| `description` | pipeline description |
| `trigger_kind` | trigger type |
| `hash` | current working tree hash |
| `active_hash` | active runtime hash |
| `activated_at` | activation timestamp |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, file_rel_path)`

### `project_policies`

Named project policy definitions.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `policy_id` | stable policy id |
| `title` | display title |
| `capabilities_json` | allowed capabilities JSON |
| `managed` | whether system-managed |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, policy_id)`

### `project_policy_bindings`

Policy bindings from subjects to policies.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `subject_kind` | bound subject kind |
| `subject_id` | bound subject id |
| `policy_id` | target policy id |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, subject_id, policy_id)`

### `project_members`

Project membership rows.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `user_id` | member user id |
| `role_preset` | access preset |
| `custom_policy_ids_json` | explicit policy ids |
| `mcp_capabilities_json` | MCP capability list |
| `created_by` | inviter/creator |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, user_id)`

### `project_invites`

Pending or historical project invites.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `invite_id` | stable invite id |
| `target_user` | invited user |
| `role_preset` | invited role preset |
| `custom_policy_ids_json` | invited custom policies |
| `mcp_capabilities_json` | invited MCP capabilities |
| `note` | invite note |
| `invited_by` | inviter |
| `status` | invite status |
| `expires_at` | expiry timestamp |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project, invite_id)`

### `worker_registry`

Known office/worker nodes.

| Column | Meaning |
| --- | --- |
| `node_id` | worker id, primary key |
| `label` | display label |
| `base_url` | worker base URL |
| `status` | worker status |
| `capabilities_json` | worker capability payload |
| `registered_at` | first registration timestamp |
| `last_heartbeat_at` | latest heartbeat timestamp |

### `project_runtime_placements`

Where and how a project should run.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `mode` | placement mode |
| `target` | placement target |
| `worker_id` | optional worker id |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |

Primary key:
- `(owner, project)`

### `project_operations`

Import/export/transfer operation log and state machine.

| Column | Meaning |
| --- | --- |
| `owner` | project owner |
| `project` | project slug |
| `operation_id` | stable operation id |
| `kind` | operation kind |
| `status` | operation status |
| `current_step` | current step label |
| `source_office_id` | source office |
| `target_office_id` | target office |
| `artifact_rel_path` | artifact relative path |
| `artifact_sha256` | artifact hash |
| `artifact_bytes` | artifact size |
| `error_message` | failure message |
| `retry_count` | retries |
| `created_at` | created timestamp |
| `updated_at` | updated timestamp |
| `completed_at` | completion timestamp |

Primary key:
- `(owner, project, operation_id)`

Index:
- `idx_project_operations_project(owner, project, updated_at DESC, operation_id ASC)`

### `mcp_sessions`

Per-project MCP session tokens.

| Column | Meaning |
| --- | --- |
| `token` | MCP bearer token, primary key |
| `owner` | project owner |
| `project` | project slug |
| `capabilities_json` | allowed MCP capabilities |
| `created_at` | created timestamp |
| `auto_reset_seconds` | optional TTL/reset window |
| `enabled` | whether active |

### `pipeline_invocations`

Execution log for pipeline runs.

| Column | Meaning |
| --- | --- |
| `id` | autoincrement primary key |
| `owner` | project owner |
| `project` | project slug |
| `file_rel_path` | pipeline path |
| `run_id` | run id |
| `at` | invocation timestamp |
| `duration_ms` | execution duration |
| `status` | run status |
| `trigger` | trigger type |
| `error` | error text |
| `trace_json` | execution trace JSON |

Index:
- `idx_pipeline_invocations_pipeline(owner, project, file_rel_path, at DESC)`

## Project SQLite

### File

- `{data_root}/users/{owner}/{project}/data/local.db`

### Purpose

This is **not** the platform catalog.

This is the project-local runtime SQLite database created by the project data engine.
Right now Zebflow ensures it exists and enables:
- WAL journaling
- `PRAGMA synchronous=NORMAL`

It is intended for project-level runtime data, not platform metadata.

## Project Sekejap

Project-local Sekejap storage is separate from SQLite:

- `{data_root}/users/{owner}/{project}/data/sekejap/`

So the actual persistent structure is:

- platform metadata -> `platform/catalog.db`
- project SQLite runtime -> `users/{owner}/{project}/data/local.db`
- project Sekejap runtime -> `users/{owner}/{project}/data/sekejap/`

## First-Principles Summary

Zebflow does **not** use one giant DB for everything.

It uses:

- one global SQLite catalog for platform metadata
- one isolated filesystem root per project
- optional per-project runtime DB engines inside that project root

That separation is the core storage model.
