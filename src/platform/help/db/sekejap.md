# SekejapQL — Sekejap Query & Write Guide

Sekejap is Zebflow's embedded multimodel database: records, graph edges,
spatial shapes, vectors and full text in one SQL, in-process, no server.
Every project has one (connection `default-multimodel`), and it is on
sekejap 0.18. Outside graph queries its SQL is PostgreSQL's; relations between
rows are graph walks.

Engine reference: <https://github.com/sekejapdb/sekejap> — `docs/lang/QL_CONTRACT.md`
is the whole dialect; this page is the short form for pipelines.

## Three things to know first

1. **Every row has a `_key`.** It is the row's address. Either the `INSERT`
   names it, or the table mints it: `_key TEXT PRIMARY KEY DEFAULT ulid()`,
   and `INSERT ... RETURNING _key` hands the new key back. An `INSERT` that
   gives no key to a table that mints none is refused (`23502`). ULID keys
   sort by time, so `ORDER BY _key DESC` is newest first.
2. **A `WHERE` needs an index on its column, and it never scans instead.**
   Every `TEXT`, `INT`, `REAL`, `BOOLEAN` and geometry column gets one
   automatically when the table is created. Full text, spatial and vector
   indexes are declared in the `WITH (...)` clause. A predicate on a
   `JSONB` column is refused by name.
3. **What is not built is refused by name, not emulated.** `JOIN` (walk the
   graph instead — below), `OFFSET` (continue after the last key),
   `array_agg` / `json_agg`, `WITH`, window functions, `CHECK`, two
   `ORDER BY` keys — each answers with the tier it sits in and why. Read the
   message; it says what to write instead.

## Running a query

From a pipeline the node is `sekejap.query` (SQL in the body, values in
`--params` as `$1, $2, …`); to try one without saving anything,
`pipeline_run body="| trigger.function | sekejap.query -- \"SELECT …\""`;
over HTTP, `POST /api/projects/{o}/{p}/db/connections/default-multimodel/query`.

```
| sekejap.query -- "SELECT _key, title FROM posts LIMIT 20"
| sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT _key, title FROM posts WHERE slug = $1"
| sekejap.query --params "{{ [input.body.title] }}" --read-only false -- "INSERT INTO posts (title) VALUES ($1) RETURNING _key"
```

Flags: `--params` (bind values; a whole `{{ }}` keeps its type, so
`"{{ [a, b] }}"` is a real array and a single value is wrapped), `--limit <n>`
(default 200 rows for reads), `--read-only true|false` (set `false` for
INSERT/UPDATE/DELETE/CREATE), `--query` (the SQL as a flag instead of the
body). Output: `{ columns, rows, row_count, truncated, affected_rows,
duration_ms }` — each row an object keyed by column name
(`input.rows[0].name`), `columns` the positional list beside it. A write
answers `affected_rows`; a write with `RETURNING` also answers its rows.

## Creating a table

```sql
CREATE TABLE contacts (
  _key TEXT PRIMARY KEY DEFAULT ulid(),
  name TEXT NOT NULL,
  email TEXT UNIQUE,
  status TEXT DEFAULT 'active',
  score REAL,
  bio TEXT,
  location GEOMETRY(Point,4326),
  embedding VECTOR(384),
  created_at TIMESTAMPTZ DEFAULT now()
) WITH (fulltext: [bio], spatial: [location], vector: [embedding])
```

Types: `TEXT` · `INT` / `BIGINT` · `REAL` / `DOUBLE PRECISION` · `BOOLEAN` ·
`JSONB` · `TIMESTAMPTZ` / `DATE` · `GEOMETRY(Point,4326)` /
`GEOMETRY(Polygon,4326)` · `VECTOR(n)` — the dimension is required.

Column clauses: `PRIMARY KEY` (on `_key`, or on one named column such as
`id TEXT PRIMARY KEY`, which then stands in for `_key`), `NOT NULL`,
`DEFAULT` (a constant — `'active'`, `0`, `false` — or `now()`, `ulid()`,
`uuid4()`), `UNIQUE` (a second equal value is `23505`; NULLs never collide),
`REFERENCES t` (for edge tables, below). No `CHECK`.

`WITH (...)` keys: `fulltext` / `bm25` (a text index, BM25-scored),
`spatial` (a point or geometry index), `vector` (exact nearest), `quantized`
(approximate, ~10× smaller), and `index: none` to switch the automatic
scalar indexes off. `hash` and `range` are accepted and mean the same
btree every scalar column already gets.

Other DDL:

```sql
CREATE INDEX ON contacts USING gin (to_tsvector('simple', bio))
CREATE INDEX ON contacts USING quantized (embedding vector_cosine_ops)
DROP INDEX [IF EXISTS] contacts_bio_gin          -- indexes are named <table>_<column>_<family>
ALTER TABLE contacts ADD COLUMN phone TEXT       -- indexed automatically, over the rows already there
ALTER TABLE contacts DROP COLUMN phone
DROP TABLE [IF EXISTS] contacts [CASCADE]        -- RESTRICT by default: refuses while edges reference its rows
CREATE SCHEMA geo  /  CREATE TABLE geo.places (…)  -- a bare name resolves in public
SHOW TABLES  |  SHOW contacts  |  SHOW CREATE TABLE contacts  |  SHOW INDEXES ON contacts  |  SHOW EDGES
EXPLAIN SELECT ...                               -- the plan the engine would build
```

A *managed table* is the same thing created from the Studio's Tables tab or
`POST /api/projects/{owner}/{project}/tables`; attribute kinds there are
`string` · `number` · `boolean` · `json` · `geo` · `vector(n)`, and index
types `hash` · `range` · `fulltext` · `vector` · `spatial`.

## Reading

```sql
SELECT _key, name, status FROM contacts WHERE status = 'active' ORDER BY name LIMIT 50
SELECT _key, score FROM contacts WHERE score BETWEEN 80 AND 100 ORDER BY score DESC LIMIT 20
SELECT _key, name FROM contacts WHERE email IN ('a@x.io', 'b@x.io')
SELECT status, COUNT(*) AS n FROM contacts GROUP BY status ORDER BY n DESC
```

One `ORDER BY` key per query. Paging continues after the last key shown,
never `OFFSET` (which would read and throw away every skipped row):

```sql
SELECT _key, name FROM contacts ORDER BY _key DESC LIMIT 20                   -- first page, newest first
SELECT _key, name FROM contacts WHERE _key < $1 ORDER BY _key DESC LIMIT 20   -- next page: $1 = last _key shown
```

**Text matching** — `LIKE` and `ILIKE` with any pattern (`%`, `_`, `ESCAPE`,
`NOT`), PostgreSQL's rules, no index needed. `name ILIKE $1` with
`$1 = '%doe%'` checks each row; `LIKE 'abc%'` uses the column's index.

**Dates** are stored as UTC microseconds behind a `TIMESTAMPTZ` column; write
an ISO-8601 string or the integer, read back ISO text, and filter with
`created_at BETWEEN '2026-09-01' AND '2026-10-01'` or
`EXTRACT(YEAR FROM created_at) = 2026`.

**Full text** — a `gin` index over the column, then:

```sql
SELECT _key, title FROM articles
WHERE to_tsvector('simple', body) @@ to_tsquery('simple', 'quarterly & report')
ORDER BY bm25(body, 'quarterly report') DESC LIMIT 10
```

`search(body, 'quartely reprot')` is the typo-tolerant form, ordered by
`search_score()`. `to_tsquery('simple', 'quar:*')` matches a prefix.

**Spatial** — PostGIS spellings, WGS84:

```sql
SELECT _key, name FROM contacts
WHERE ST_DWithin(location, ST_MakePoint(144.96, -37.81)::geography, 5000)
ORDER BY location <-> ST_MakePoint(144.96, -37.81)::geography LIMIT 10
```

**Vectors** — pgvector spellings; the parameter is a JSON array of numbers:

```sql
SELECT _key, name FROM contacts ORDER BY embedding <=> $1 LIMIT 5
```

`<=>` is cosine, `<->` L2, `<#>` negative inner product.

**Graph** — where PostgreSQL would `JOIN`, sekejap walks. A relation between
rows is an edge, and `GRAPH_TABLE` walks it in ISO GQL:

```sql
SELECT _key, name FROM GRAPH_TABLE (base MATCH
  (u:users WHERE u._key = $1)-[:follows]->(f:users)
  RETURN f._key AS _key, f.name AS name)
```

- `<-[:follows]-` walks the other way; `-[:follows]->{1,3}` walks up to three
  hops, and `{0,n}` includes the starting node.
- An edge's own properties are read by naming the edge: `-[e:follows]->`
  then `RETURN e.since AS since`.
- Each matching path is one row: a node reached by two paths comes back
  twice. `RETURN DISTINCT` returns it once.
- `OPTIONAL MATCH`, `EXISTS`, `UNION` and `CALL` work inside the body; the
  outer `SELECT` filters, groups, orders and pages the result like a table:
  `SELECT g.name, COUNT(*) FROM GRAPH_TABLE (...) AS g GROUP BY g.name`.
- `base` walks every edge; a property graph's name (below) walks that graph.

**Edge tables** — edges with typed properties and a key, written in plain
SQL. A table with two `REFERENCES` columns, declared as an edge table by a
property graph:

```sql
CREATE TABLE affiliated_with (
  member_id TEXT REFERENCES members,
  institution_id TEXT REFERENCES institutions,
  position TEXT,
  PRIMARY KEY (member_id, institution_id)        -- one edge per pair
)
CREATE PROPERTY GRAPH network
  VERTEX TABLES (members, institutions)
  EDGE TABLES (affiliated_with
    SOURCE KEY (member_id) REFERENCES members (_key)
    DESTINATION KEY (institution_id) REFERENCES institutions (_key))

INSERT INTO affiliated_with (member_id, institution_id, position) VALUES ($1, $2, $3)
UPDATE affiliated_with SET position = $3 WHERE member_id = $1 AND institution_id = $2
DELETE FROM affiliated_with WHERE member_id = $1 AND institution_id = $2
SELECT institution_id, position FROM affiliated_with WHERE member_id = $1
```

An edge to a row that does not exist is refused (`23503`); a taken key is
`23505`. A `WHERE` on an edge table names an end. Deleting a row removes
its edges. `GRAPH_TABLE (network MATCH ...)` walks these edges.

## Writing

```sql
INSERT INTO contacts (name, email) VALUES ($1, $2) RETURNING _key
INSERT INTO contacts (_key, name) VALUES ('a', 'Ann'), ('b', 'Bob')
INSERT INTO contacts (_key, name) VALUES ($1, $2) ON CONFLICT (_key) DO UPDATE SET name = EXCLUDED.name
INSERT INTO contacts (_key, name) VALUES ($1, $2) ON CONFLICT (_key) DO NOTHING
UPDATE contacts SET status = 'inactive' WHERE _key = $1
UPDATE contacts SET status = 'inactive' WHERE score < 10
DELETE FROM contacts WHERE _key = $1
```

A plain `INSERT` never overwrites: a key that exists is `23505`, and
`ON CONFLICT` is the upsert. `UNIQUE` keeps any other column unique. Each
statement is its own transaction; `BEGIN` / `COMMIT` are refused. A batch
that must land together goes through `sekejap.insert`.

## The `sekejap.insert` node

Bulk records and graph edges, typed against the table's columns, as one
commit:

```
| sekejap.insert --target contacts --records-path items --edges-path links
```

with a payload shaped

```json
{
  "items": [{ "key": "a", "fields": { "name": "Ann", "embedding": [0.1, 0.2, "…"] } }],
  "links": [{ "from": { "target": "contacts", "key": "a" }, "type": "knows",
              "to": { "target": "contacts", "key": "b" }, "fields": { "since": 2026 } }]
}
```

The table must already exist; a `VECTOR(n)` field takes exactly n numbers;
both endpoints of an edge must exist, in this batch or before it. A field
the table does not declare is stored as an extra and is not indexed.

An edge whose `type` is an edge table's label is written into that table,
its `fields` as the table's columns; the target itself must be a table of
rows. Records and edges land in one commit, or not at all.

## Platform collections

The platform catalog is separate from the project store. Use the admin DB
API for platform metadata, not the project Sekejap connection.
