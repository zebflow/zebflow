# Databases

Every project has two databases ready without any setup, and can connect to
external ones by credential.

| Connection slug | Kind | What it is |
|---|---|---|
| `default-multimodel` | `sekejap` | Zebflow's embedded multi-model store: tables, graph, vector, spatial, full-text. The default choice for a project's own data. |
| `default` | `sqlite` | an embedded SQLite file (`data/store/local.db`) |
| yours | `postgresql`, … | external servers, added in Studio → DB Connections with a credential |

`connection_list` shows them; `connection_describe slug=… [scope=tables|schemas|functions] [schema=] [table=]`
shows tables and columns. Read the schema before writing SQL.

## Nodes

| Node | Database | Binds |
|---|---|---|
| `sekejap.query.run` | Sekejap | `$1, $2 …` |
| `sekejap.record.create` | Sekejap — bulk records and graph edges (`--table`, `--record`, `--edge`) | |
| `sqlite.query.run` | the project's `default` SQLite — reads and writes (`--write`) | `?1, ?2 …` |
| `postgres.query.run` | PostgreSQL — `--credential <id from credential_list>` | `$1, $2 …` |
| `table.query.run` | files (CSV, JSON, NDJSON, Parquet) with SQL | `$1, $2 …` |

SQL goes in the body; values go in `--param`:

```
| sekejap.query.run --param "1={{ $trigger.params.id }}" -- "SELECT id, title FROM posts WHERE id = $1"
| sekejap.query.run --param "1={{ $trigger.body.title }}" --param "2={{ $trigger.body.slug }}" --write -- "INSERT INTO posts (title, slug) VALUES ($1, $2)"
| sqlite.query.run --param "1={{ $trigger.body.email }}" -- "SELECT * FROM users WHERE email = ?1"
| postgres.query.run --credential pg_main --param "1={{ $trigger.auth.sub }}" -- "SELECT * FROM accounts WHERE id = $1"
| table.query.run --from "datasets/orders.parquet as o" --param "1={{ $trigger.params.region }}" -- "SELECT * FROM o WHERE region = $1"
```

Query nodes answer one key, `query: { rows, columns, row_count, truncated }`,
plus `rows_affected` when run with `--write` — the rows are `input.query.rows`,
never `input.rows`. `table.query.run` only reads; with `--folder`, `--filename`
or `--path` it writes the whole result as `--format csv|json|ndjson|parquet`
and `query` is that file's FileRef with `row_count`, `columns` and `format`
(`--rows` adds the first `--limit` rows). There is no MySQL node; a MySQL connection can be stored
but nothing queries it from a pipeline.

## Trying a query

`pipeline_run` runs a body once without saving it:

```
pipeline_run body="| trigger.function | sekejap.query.run --limit 5 -- \"SELECT * FROM posts\""
```

The Studio's DB pages (`/projects/{o}/{p}/db/{kind}/{slug}/query`) run the
same thing interactively, and `POST /api/projects/{o}/{p}/db/connections/{id}/query`
is the HTTP form.

## Creating tables

- **Sekejap:** plain SQL through the node — `CREATE TABLE posts (id TEXT, title TEXT, body_json JSON, created_at TEXT)` — or a *managed table* with declared attributes and indexes (hash, range, full-text, vector, spatial) in Studio → the connection's Tables tab, or `POST /api/projects/{o}/{p}/tables`. Declared schemas live in `schemas/sekejap/`; seed rows in `initial-data/`.
- **SQLite:** `sqlite.query.run --write -- "CREATE TABLE …"`; schema in `schemas/sqlite/schema.sql`.
- **PostgreSQL:** `postgres.query.run --credential … --write -- "CREATE TABLE …"` against a credential that is allowed to.

`help("db/sekejap")` for SekejapQL: graph reads with `FROM MATCH`, full-text,
vectors, spatial, the managed-table API.
