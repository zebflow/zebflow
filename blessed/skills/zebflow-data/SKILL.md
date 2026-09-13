---
name: zebflow-data
description: Storing and querying data in a Zebflow project — Sekejap (the built-in database), SQLite, PostgreSQL by credential, tables, schema, migrations, seeds. Use before writing any SQL, creating a table, or adding a field; covers schema-first discovery, naming conventions, binds, and verifying with pipeline_run.
license: MIT
metadata:
  version: "1"
---

# Data: schema first, then SQL

Every project has `default-multimodel` (Sekejap — tables, graph, vector,
full-text, spatial) and `default` (SQLite) with no setup; PostgreSQL and
others attach by credential. Facts: `help(topic="db")`, `db/sekejap`, the
query nodes in `pipeline/nodes`.

## Discover before you write

1. `connection_list` — slugs and kinds. Slugs are for `connection_describe`;
   `--credential` on `pg.query` takes a **credential id** from `credential_list`.
2. `connection_describe slug=default-multimodel` (`scope=tables`, then
   `table=<name>` for columns). Never invent a table or a column: an unknown
   name in Sekejap fails the node at request time, not at register time.
3. If `docs/schema.md` exists, `file_read` it first; if it does not, write it
   after step 2 so the next session starts here.
4. Look at three real rows before querying in earnest —
   `pipeline_run body="| trigger.function | sekejap.query --limit 3 -- \"SELECT * FROM posts\""`.
   Value formats, enums and nulls are only visible in data.

## Write the schema down, then apply it

- Tables and migrations are files: `db/001_posts.sql` (or under
  `schemas/sekejap/` for a declared schema, `schemas/sqlite/schema.sql` for
  SQLite), applied through the node — `sekejap.query --read-only false -- "CREATE TABLE …"`
  — from a one-off `pipeline_run` or a `jobs/migrate` function pipeline. Keep
  the file; the database is not the record.
- Conventions that pay for themselves: an `id` (or Sekejap's `_key`), `created_at`
  and `updated_at` as ISO strings, a `status` column over booleans when a
  third state will come, `locale` and `translation_of` on anything a person
  reads, `slug` unique per locale. Store rich text as the editor's JSON
  (`body_json`) with derived `body_html`.
- Sekejap column kinds: `TEXT`, `REAL`/`INTEGER`, `BOOLEAN`, `JSON`; managed
  tables add indexes (hash, range, fulltext, vector, spatial) — `db/sekejap`.
  Sekejap has no `UPSERT`/`ON CONFLICT`: select by key, then `logic.if`
  between `UPDATE` and `INSERT`.
- Seeds go in `initial-data/` so a fresh install of the project (or of a
  bundle made from it) gets them.

## Querying

```
| sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT id, title, body_json FROM posts WHERE slug = $1"
| sekejap.query --params "{{ [input.body.title, input.body.slug] }}" --read-only false -- "INSERT INTO posts (title, slug) VALUES ($1, $2)"
| sqlite.query --params "{{ [input.body.email] }}" -- "SELECT * FROM users WHERE email = ?1"
| pg.query --credential pg_main --params "{{ [$trigger.auth.sub] }}" -- "SELECT * FROM accounts WHERE id = $1"
```

- SQL in the body, values in `--params`; a whole `{{ }}` keeps its type, so
  `"{{ [a, b] }}"` is a real array. Never build SQL text from input.
- The result is `{ columns, rows, row_count, … }` for reads and
  `{ affected_rows }` for writes. The next node reads `input.rows`; a page
  reads `input.rows` too. `--read-only false` is required for any write.
- Reads default to 200 rows (`--limit`); paginate with `LIMIT $1 OFFSET $2`
  bound from `$trigger.query.page`.
- In PostgreSQL text, use `format()`/`concat()` rather than `||` inside a DSL
  body — the parser reads `|` as a node separator.
- Never branch on the database kind inside a page; the pipeline decides
  where data comes from, the page receives rows.

## Prove it

1. `pipeline_run` the exact query with a real `input` and read the rows.
2. After a migration, `connection_describe table=<name>` shows the columns
   you declared.
3. After a write path, read it back: the POST pipeline's redirect target
   renders the new row.
4. Write what you learned about the schema to `docs/schema.md` and the
   decisions (why a column exists) to `MEMORY.md`.

## Large data

Rows do not travel through MCP tool windows well. Aggregate
(`string_agg(...)`), paginate, or write a file with `table.convert` /
`table.query` and pass its FileRef; never paste thousands of rows into a
script node or a page payload.
