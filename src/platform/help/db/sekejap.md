# SekejapQL — Sekejap Query & Write Guide

Sekejap is Zebflow's embedded multi-model database — graph, vector, spatial, full-text, and vague
temporal. In Zebflow, project queries should be written as SQL-like SekejapQL.

Project + engine reference:

- GitHub: <https://crates.io/crates/sekejap>

## Query Shape

Read queries start with `SELECT`.

Basic table query:

```sql
SELECT _key, name, status
FROM contacts
WHERE status = 'active'
ORDER BY name ASC
LIMIT 50
```

Graph query:

```sql
SELECT friend._key AS friend_key, friend.name AS friend_name
FROM MATCH (u:users)-[:follows]->(friend:users)
WHERE u._key = 'alice'
LIMIT 50
```

The `SELECT` list now acts like the return clause from the older graph form.
If you are used to:

```sql
MATCH (u:users)-[:follows]->(friend:users) WHERE u._key = 'alice' RETURN friend
```

write this instead:

```sql
SELECT friend._key AS friend_key, friend.name AS friend_name
FROM MATCH (u:users)-[:follows]->(friend:users)
WHERE u._key = 'alice'
```

## Running a query

From a pipeline the node is `sekejap.query` (SQL in the body, values in
`--params`); to try one without saving anything,
`pipeline_run body="| trigger.function | sekejap.query -- \"SELECT …\""`;
over HTTP, `POST /api/projects/{o}/{p}/db/connections/default-multimodel/query`.

List rows:

```sql
SELECT _key, title, created_at
FROM posts
ORDER BY created_at DESC
LIMIT 100
```

Filter by field:

```sql
SELECT _key, email, status
FROM contacts
WHERE status = 'active'
LIMIT 50
```

Range + sort:

```sql
SELECT _key, score
FROM contacts
WHERE score >= 80
ORDER BY score DESC
LIMIT 20
```

Full-text search:

```sql
SELECT _key, title
FROM articles
WHERE title ILIKE '%quarterly report%'
LIMIT 10
```

Graph traversal:

```sql
SELECT cause._key AS cause_key
FROM MATCH (event:events)-[:caused_by*1..5]->(cause:events)
WHERE event._key = 'maribyrnong-flood'
LIMIT 20
```

## Creating a table

Plain SQL through the node creates a table:

```
| sekejap.query --read-only false -- "CREATE TABLE contacts (name TEXT, email TEXT, status TEXT, score REAL, created_at TEXT)"
```

`SHOW TABLES` lists them. A *managed table* additionally declares attribute
kinds and indexes (hash, range, full-text, vector, spatial) and appears in
the Studio's Tables tab — create it there or over HTTP:

`POST /api/projects/{owner}/{project}/tables`:

```json
{
  "table": "contacts",
  "attributes": [
    {"name": "name", "kind": "string"},
    {"name": "email", "kind": "string", "index_types": ["hash"]},
    {"name": "status", "kind": "string", "index_types": ["hash"]},
    {"name": "score", "kind": "number", "index_types": ["range"]},
    {"name": "bio", "kind": "text", "index_types": ["fulltext"]},
    {"name": "embedding", "kind": "vector", "index_types": ["vector"]}
  ],
  "hash_indexed_fields": ["email", "status"],
  "range_indexed_fields": ["score"]
}
```

Attribute kinds: `string` · `number` · `boolean` · `text` · `json` · `vector` · `geo`.
Index types: `hash` · `range` · `fulltext` · `vector` · `spatial`.

## Writing Rows

Use SQL directly:

```sql
INSERT INTO contacts (_key, name, email, status, score, created_at)
VALUES ('alice-001', 'Alice', 'alice@example.com', 'active', 95, 1741300000000)
```

## Full Example: Create + Insert + Query

```sql
-- 1. Create table
CREATE TABLE products (_key TEXT PRIMARY KEY, name TEXT, price REAL, category TEXT, in_stock JSON)

-- 2. Insert rows
INSERT INTO products (_key, name, price, category, in_stock) VALUES ('prod-001', 'Widget A', 29.99, 'widgets', true)
INSERT INTO products (_key, name, price, category, in_stock) VALUES ('prod-002', 'Widget B', 49.99, 'widgets', false)

-- 3. Query rows
SELECT _key, name, price
FROM products
WHERE category = 'widgets' AND price >= 30
ORDER BY price ASC
LIMIT 20
```

## The `sekejap.query` node

```
| sekejap.query -- "SELECT _key, title FROM posts LIMIT 20"
| sekejap.query --params "{{ [$trigger.params.id] }}" -- "SELECT friend._key AS friend_key FROM MATCH (u:users)-[:follows]->(friend:users) WHERE u._key = $1"
| sekejap.query --params "{{ [input.body.slug, input.body.title] }}" --read-only false -- "INSERT INTO posts (_key, title) VALUES ($1, $2)"
```

Flags: `--params` (bind values for `$1, $2 …`; a whole `{{ }}` keeps its type,
so `"{{ [a, b] }}"` is a real array and a single value is wrapped),
`--limit <n>` (default 200 rows for reads), `--read-only true|false`
(refuse writes; set `false` for INSERT/UPDATE/DELETE/CREATE), `--query`
(the SQL as a flag instead of the body). Output:
`{ columns, rows, row_count, truncated, affected_rows, duration_ms }` — the
rows are `input.rows` in the next node.

## Platform Collections

The platform catalog is separate from the project Sekejap store. Use the admin DB API for platform
metadata, not the project Sekejap connection.
