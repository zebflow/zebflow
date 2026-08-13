# Platform Services

The platform module turns runtime parts into a product with users, projects,
permissions, settings, UI, APIs, MCP, Git, Hub, and database management.

## Layers

- `model.rs` contains shared platform data types.
- `services/` owns reusable product behavior.
- `web/` owns routes, request handling, and embedded UI.
- `mcp/` exposes project tools with scoped access.
- `db/` provides database driver interfaces.
- `policy/` scans package effects and risk.
- `adapters/` owns physical data and file implementations.
- `sqlite_schema.rs` owns the platform catalog schema.

Web handlers should call services. They should not grow a second copy of service
logic. A service used by UI and MCP must give both paths the same validation and
authorization.

## Controller and Office

The controller manages platform level coordination. An office owns project
execution and project local services. Routes for a project must reach its owning
office. Credentials and connection IDs must be resolved in that project scope.

## Related Source

- `src/platform/mod.rs`
- `src/platform/model.rs`
- `src/platform/services/`
- `src/platform/web/mod.rs`
- `src/platform/mcp/`
- `src/platform/db/`
- `src/platform/policy/`
- `src/platform/sqlite_schema.rs`
