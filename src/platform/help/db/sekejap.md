# SekejapQL — Sekejap Query & Write Guide

Sekejap is Zebflow's embedded multi-model database — graph, vector, spatial, full-text, and vague
temporal. In Zebflow, project queries should be written as SQL-like SekejapQL.

Project + engine reference:

- GitHub: <https://github.com/insanalamin/sekejap>

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

## Querying via `run_db_query`

Pass SekejapQL text directly as the `sql` param.

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

## Creating a Table — DSL

```text
create table contacts --title "Contacts" --fields "name:Text,email:Text,status:Text,score:Number,created_at:Number" --hash "email,status" --range "score,created_at"
```

- `--fields "f1:Kind,f2:Kind"` — comma-separated `name:Kind` pairs. Kinds: `Text`, `Number`, `Boolean`, `Json`
- `--hash "f1,f2"` — exact-match index fields
- `--range "f1,f2"` — range index fields

After creation you can immediately query:

```sql
SELECT _key, name
FROM contacts
LIMIT 50
```

List all tables:

```sql
SHOW TABLES
```

## Creating a Table — HTTP API

`POST /api/projects/{owner}/{project}/tables`:

```json
{
  "table": "contacts",
  "title": "Contacts",
  "attributes": [
    {"name": "name", "kind": "Text"},
    {"name": "email", "kind": "Text"},
    {"name": "status", "kind": "Text"},
    {"name": "score", "kind": "Number"},
    {"name": "created_at", "kind": "Number"}
  ],
  "hash_indexed_fields": ["email", "status"],
  "range_indexed_fields": ["score", "created_at"]
}
```

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

## `n.sekejap.query` Pipeline Node Config

```zf
| n.sekejap.query -- "SELECT _key, title FROM posts LIMIT 20"
| n.sekejap.query -- "SELECT friend._key AS friend_key FROM MATCH (u:users)-[:follows]->(friend:users) WHERE u._key = '{{ $trigger.params.id }}'"
```

Optional flags:

- `--limit <n>` — maximum rows returned for read queries
- `--read-only true|false` — reject writes when enabled

## Platform Collections

The platform catalog is separate from the project Sekejap store. Use the admin DB API for platform
metadata, not the project Sekejap connection.
