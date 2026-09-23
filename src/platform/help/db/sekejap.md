# SekejapQL — Sekejap Query & Write Guide

Sekejap is Zebflow's embedded multimodel database: records, graph edges,
spatial shapes, vectors and full text in one SQL, in-process, no server.
Every project has one (connection `default-multimodel`), and it is on
sekejap 0.17.

Engine reference: <https://github.com/sekejapdb/sekejap> — `docs/lang/QL_CONTRACT.md`
is the whole dialect; this page is the short form for pipelines.

## Three things to know first

1. **Every row has a `_key` you supply.** It is the row's address. An
   `INSERT` names it, always — Zebflow refuses one that does not, because
   sekejap would otherwise take the first column as the key. For a key
   nobody typed, put a `crypto --op random_hex` node before the query and
   bind `$nodes.<id>.hex`.
2. **A `WHERE` needs an index on its column, and it never scans instead.**
   Every `TEXT`, `INT`, `REAL`, `BOOLEAN` and geometry column gets one
   automatically when the table is created. Full text, spatial and vector
   indexes are declared in the `WITH (...)` clause. A predicate on a
   `JSONB` column is refused by name.
3. **What is not built is refused by name, not emulated.** `JOIN`, `OFFSET`,
   `UNION`, window functions, `ILIKE`, two `ORDER BY` keys — each answers
   with the tier it sits in and why. Read the message; it says what to write
   instead.

## Running a query

From a pipeline the node is `sekejap.query` (SQL in the body, values in
`--params` as `$1, $2, …`); to try one without saving anything,
`pipeline_run body="| trigger.function | sekejap.query -- \"SELECT …\""`;
over HTTP, `POST /api/projects/{o}/{p}/db/connections/default-multimodel/query`.

```
| sekejap.query -- "SELECT _key, title FROM posts LIMIT 20"
| sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT _key, title FROM posts WHERE slug = $1"
| sekejap.query --params "{{ [input.body.slug, input.body.title] }}" --read-only false -- "INSERT INTO posts (_key, title) VALUES ($1, $2)"
```

Flags: `--params` (bind values; a whole `{{ }}` keeps its type, so
`"{{ [a, b] }}"` is a real array and a single value is wrapped), `--limit <n>`
(default 200 rows for reads), `--read-only true|false` (set `false` for
INSERT/UPDATE/DELETE/CREATE), `--query` (the SQL as a flag instead of the
body). Output: `{ columns, rows, row_count, truncated, affected_rows,
duration_ms }` — each row an object keyed by column name
(`input.rows[0].name`), `columns` the positional list beside it.

## Creating a table

```sql
CREATE TABLE contacts (
  _key TEXT PRIMARY KEY,
  name TEXT, email TEXT, status TEXT,
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

Column clauses: `PRIMARY KEY` (on `_key` only), `NOT NULL`, `DEFAULT now()`,
`DEFAULT uuid4()`, `DEFAULT ulid()`. No `UNIQUE`, `REFERENCES` or `CHECK`;
uniqueness beyond `_key` is enforced in the pipeline (a `SELECT` before the
`INSERT`).

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
SHOW TABLES  |  SHOW contacts  |  SHOW INDEXES ON contacts  |  SHOW EDGES
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

One `ORDER BY` key per query. Paging is by key, not `OFFSET`:
`WHERE _key > $1 ORDER BY _key LIMIT 50`, binding the last key seen.

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
`search_score()`.

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

**Graph** — edges are written with `sekejap.insert` (below) and walked with
`GRAPH_TABLE`. The first node in the pattern is anchored by its `_key`:

```sql
SELECT _key, name FROM GRAPH_TABLE (base MATCH
  (u:users WHERE u._key = $1)-[:follows]->(f:users)
  COLUMNS (f._key AS _key, f.name AS name))
```

`<-[:follows]-` walks the other way; `-[:follows]->{1,3}` walks up to
three hops; an edge's own properties project as `@edge.<name>`.

## Writing

```sql
INSERT INTO contacts (_key, name, email) VALUES ($1, $2, $3)
INSERT INTO contacts (_key, name) VALUES ('a', 'Ann'), ('b', 'Bob')
UPDATE contacts SET status = 'inactive' WHERE _key = $1
UPDATE contacts SET status = 'inactive' WHERE score < 10
DELETE FROM contacts WHERE _key = $1
```

A `SELECT` before an `INSERT` is how uniqueness on any column other than
`_key` is kept.

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

## Platform collections

The platform catalog is separate from the project store. Use the admin DB
API for platform metadata, not the project Sekejap connection.
