# Platform Internals

Authoritative modules:

- `src/platform/services/`
- `src/platform/web/`
- `src/platform/model.rs`
- `src/platform/sqlite_schema.rs`
- `src/platform/operations.rs`
- `src/platform/policy/`
- `src/infra/`
- `src/provision/`
- `src/bin/zebflow.rs`

Contract boundary:

- `docs/developer/repository-structure.md`
- `docs/developer/architecture.md`
- `docs/developer/platform.md`
- `docs/contracts/platform.md`
- `docs/contracts/versioning.md`

Responsibilities:

| Module | Responsibility |
|---|---|
| `platform/web` | HTTP routes, UI handlers, project pages, API endpoints |
| `platform/services` | project, credential, hub, library, runtime, auth, git, transfer services |
| `platform/mcp` | project-scoped MCP transport and tools |
| `platform/policy` | package/source review and warnings |
| `platform/db` | database driver abstraction |
| `platform/help` | embedded help served through UI/MCP |
| `infra` | execution, storage, cache, cluster, health, scheduler, websocket primitives |
| `provision` | deployment manifest helpers |
| `bin/zebflow.rs` | CLI/server entrypoint |

Rules:

- Keep service logic out of page handlers when it is reusable.
- Keep policy checks centralized under `platform/policy`.
- Do not hide dangerous package effects from review.
- Keep project-level concerns project-scoped; keep platform service management platform-scoped.
- Health/liveness logic must remain reachable when the main runtime is under load.
