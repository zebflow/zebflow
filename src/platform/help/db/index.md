# Databases

Zebflow supports two categories of databases: the built-in Sekejap embedded store and external connections.

---

## Sekejap — Built-in, Always Available

Zebflow's built-in multi-model database — graph, vector, spatial, full-text, vague temporal.

**No connection setup needed.** Scoped to your project automatically.

Suitable for: blog posts, user tables, AI memory, vector embeddings, event graphs, RAG indexes.

### Workflow

1. Create a table in the Studio UI (Tables page)
2. Use `n.sekejap.query` in pipelines with raw SQL
3. For graph reads, use `SELECT ... FROM MATCH ...`

### Node

```zf
| n.sekejap.query -- "SELECT _key, title FROM posts LIMIT 20"
| n.sekejap.query --params-path params.id -- "SELECT friend._key AS friend_key FROM MATCH (u:users)-[:follows]->(friend:users) WHERE u._key = $1"
| n.sekejap.query --params-expr "[$trigger.body.slug, $trigger.body.title]" -- "INSERT INTO posts (_key, title) VALUES ($1, $2)"
```

### Reference

Call `help("db/sekejap")` for the full SekejapQL query language reference.

---

## External DB Connections

PostgreSQL, MySQL, and other databases via named connections.

### Setup

Configure connections in Studio → Connections → DB Connections.

### Workflow

1. `connection_list` — see your configured connections and their slugs
2. `connection_describe slug=<slug>` — inspect the schema
3. Use slug in pipeline nodes: `n.pg.query --credential <slug>`

### Available pipeline nodes

- `n.pg.query` — PostgreSQL queries (SELECT, INSERT, UPDATE, DELETE)
- `n.mysql.query` — MySQL/MariaDB queries
- `n.sekejap.query` — Sekejap (built-in)

### Schema discovery

```
connection_describe slug=main-db scope=tables
connection_describe slug=main-db table=public.users
```

Always run `connection_describe` before writing SQL queries.

---

## Further Reading

- `help("db/sekejap")` — SekejapQL query language: INSERT, SELECT, UPDATE, DELETE syntax
