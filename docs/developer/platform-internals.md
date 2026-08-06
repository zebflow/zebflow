# Platform Internals

Authority:

- `src/platform/web/`
- `src/platform/services/`
- `src/platform/model.rs`
- `src/platform/sqlite_schema.rs`
- `src/platform/operations.rs`

Major areas:

- project CRUD and source workspace
- Project Studio pages and APIs
- credentials and credential types
- database connections
- files and assets
- Hub service and package add/publish
- MCP sessions and tools
- git remote/commit/sync
- profile and access control
- settings
- transfer/import/export
- invocation history

Rule:

If behavior is shared by UI, API, MCP, or pipeline nodes, place it in a service or shared helper rather than duplicating it in a page handler.
