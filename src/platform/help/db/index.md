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
| `sekejap.query` | Sekejap | `$1, $2 …` |
| `sekejap.insert` | Sekejap — bulk records and graph edges from a payload (`--target`, `--records-path`, `--edges-path`) | |
| `sqlite.query` / `sqlite.mutate` | the project's `default` SQLite | `?1, ?2 …` |
| `pg.query` | PostgreSQL — `--credential <id from credential_list>` | `$1, $2 …` |
| `table.query` | files (CSV, JSON, NDJSON, Parquet) with SQL | `$1, $2 …` |

SQL goes in the body; values go in `--params`:

```
| sekejap.query --params "{{ [$trigger.params.id] }}" -- "SELECT id, title FROM posts WHERE id = $1"
| sekejap.query --params "{{ [input.body.title, input.body.slug] }}" --read-only false -- "INSERT INTO posts (title, slug) VALUES ($1, $2)"
| sqlite.query --params "{{ [input.body.email] }}" -- "SELECT * FROM users WHERE email = ?1"
| pg.query --credential pg_main --params "{{ [$trigger.auth.sub] }}" -- "SELECT * FROM accounts WHERE id = $1"
```

Query nodes answer `{ columns, rows, row_count, … }` for reads and
`{ affected_rows }` for writes — the rows are `input.rows`, never `input`
itself. There is no MySQL node; a MySQL connection can be stored but nothing
queries it from a pipeline.

## Trying a query

`pipeline_run` runs a body once without saving it:

```
pipeline_run body="| trigger.function | sekejap.query --limit 5 -- \"SELECT * FROM posts\""
```

The Studio's DB pages (`/projects/{o}/{p}/db/{kind}/{slug}/query`) run the
same thing interactively, and `POST /api/projects/{o}/{p}/db/connections/{id}/query`
is the HTTP form.

## Creating tables

- **Sekejap:** plain SQL through the node — `CREATE TABLE posts (id TEXT, title TEXT, body_json JSON, created_at TEXT)` — or a *managed table* with declared attributes and indexes (hash, range, full-text, vector, spatial) in Studio → the connection's Tables tab, or `POST /api/projects/{o}/{p}/tables`. Declared schemas live in `schemas/sekejap/`; seed rows in `initial-data/`.
- **SQLite:** `sqlite.mutate -- "CREATE TABLE …"`; schema in `schemas/sqlite/schema.sql`.
- **PostgreSQL:** `pg.query --credential … -- "CREATE TABLE …"` against a credential that is allowed to.

`help("db/sekejap")` for SekejapQL: graph reads with `FROM MATCH`, full-text,
vectors, spatial, the managed-table API.
