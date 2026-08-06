# Data, Files, and Maps

Authoritative code and docs:

- `src/platform/sekejap.rs`
- `src/platform/services/db_connection.rs`
- `src/platform/services/db_runtime.rs`
- `src/platform/db/`
- `src/zebfs/`
- `src/mapserver/`
- `src/pipeline/nodes/basic/sekejap_*.rs`
- `src/pipeline/nodes/basic/sqlite_*.rs`
- `src/pipeline/nodes/basic/pg_query.rs`
- `src/pipeline/nodes/basic/fs_*.rs`
- `src/pipeline/nodes/basic/table_*.rs`
- `src/pipeline/nodes/basic/geo_*.rs`
- `src/pipeline/nodes/basic/mapserver_crud.rs`
- `src/platform/help/db/`
- `src/platform/help/guide/mapserver.md`

User-facing docs:

- `docs/usage/sekejap-db.md`
- `docs/usage/files-storage.md`
- `docs/usage/mapserver-gis.md`

Database surfaces:

- Sekejap embedded project database
- SQLite project/database connections
- PostgreSQL connections
- connection registry and database browser
- query nodes and high-throughput structured mutation nodes

File surfaces:

- project files UI
- `/fs/{owner}/{project}/{path}` serving path
- file ACL through `src/zebfs/acl.rs`
- upload, paste, drag, mkdir, delete, access changes
- FileRef payloads in pipelines
- runtime artifacts

Map surfaces:

- `/ms/{owner}/{project}/{path}`
- map layer registry
- GeoJSON and GeoParquet sources
- bbox/query/stats/tile paths
- style DSL
- MVT support in mapserver resolve code

Large data rule:

- Store large bytes as files.
- Pass FileRef-like values through JSON.
- Use table/geo/map nodes to inspect, convert, query, and publish.
- Avoid sending full datasets through script payloads unless deliberately bounded.
