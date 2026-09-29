---
name: zebflow-sekejap
description: Writing SekejapQL for a Zebflow project's built-in database (`default-multimodel`) — tables, edge tables, GRAPH_TABLE walks, JSONB, full text, paging. Use before the first sekejap.query in a task; covers what this engine version refuses, where its grammar lives for exactly this version, and how to prove a query before saving it.
license: MIT
metadata:
  version: "1"
---

# Sekejap: the version you run, not the one on main

This build runs **sekejap <!-- sekejap-version -->**. Its grammar:

<!-- sekejap-grammar -->

Read the pinned copy. Main moves ahead of releases; a feature it lists that
the pinned copy lacks is refused here. `help(topic="db/sekejap")` is the
short form with examples; this skill is the order of work.

## 1. Find the real schema

- `connection_describe slug=default-multimodel scope=tables`, then
  `table=<name>` for columns. **An empty table may show no columns** —
  that means "no rows sampled yet", not "no columns".
- Then read the declared source: the project's DDL file (often
  `data/model/*.sql`, or whatever `docs/structure.md` names) — it is the
  only place an empty table's columns and its edge tables are written down.
- `pipeline_run body="| trigger.function | sekejap.query -- \"SHOW CREATE TABLE posts\""`
  answers from the engine itself; `SHOW EDGES` lists every edge table.
- Look at three real rows before writing a filter: values, enums and nulls
  only show in data.

## 2. Write the query sekejap answers

- **No JOIN.** A relation is an edge; walk it:
  `SELECT a.title, m.name FROM GRAPH_TABLE (base MATCH (a:articles WHERE a._key = $1)<-[w:wrote]-(m:members) RETURN a.title AS title, m.name AS name, w.ord AS ord) AS g ORDER BY g.ord`.
  One row per path — `RETURN DISTINCT` when two paths reach the same node.
- **`_key` is the row's address**; a table may use it as the slug.
- **Every `WHERE` column needs an index** (scalars get one automatically;
  a long free-text column may have none — `WHERE about = …` is refused).
- **Schema changes are cheap**: `ADD COLUMN … DEFAULT`, `RENAME COLUMN`,
  `DROP COLUMN` and `SET NOT NULL` work on tables that hold rows, edge
  tables included (their end columns stay).
- **An edge table is read by one of its ends**: `WHERE member = $1` works;
  `SELECT *`, `COUNT(*)` or a filter on neither end is refused. To count
  edges, walk them in `GRAPH_TABLE` and `COUNT(*)` the outer select.
- **Values go in `--params`, never in the SQL text.** A `JSONB` column takes
  a bound object or array: `--params "{{ [input.body.meta] }}"`. A quoted
  `'{"a":1}'` is stored as a string, and nothing warns you.
- **Refused here, and the replacement:**
  - `now()` in `VALUES` → bind `new Date().toISOString()`, or `DEFAULT now()` on the column
  - `LOWER(col) = $1` → store a lower-cased copy and match that
  - `LIKE … OR LIKE …` (even with a trigram index) → one `LIKE` per query, or a `fulltext` index with `search()`
  - `col->>'k'` in the select list → return `col`, read the key in a script
  - `OFFSET` → keyset: `WHERE (name, _key) > ($1, $2) ORDER BY name, _key`
  - an indexed value over 1024 bytes → keep long text unindexed
- The refusal message names its `QL_CONTRACT §` — open that section in the
  pinned copy, not on main.

## 3. Wire it into a pipeline

- Several queries feeding one script: chain them in a line
  (`| q_a | q_b | merge`), reading each by literal id in the script
  (`ctx.nodes.q_a.rows`). Plain edges from parallel nodes into one node do
  **not** join — it runs once per edge, seeing one predecessor each time.
  Use `logic.collect` if you truly need a fan-in.
- A read answers `{ columns, rows, row_count }`; each row is an object
  (`input.rows[0].title`). A write needs `--read-only false`.

## 4. Prove it before you save it

1. `pipeline_run` the exact query with realistic params; read the rows.
2. An empty table proves only the syntax. Test the shaping script with
   fixture `ctx.nodes` in `pipeline_run` — never by inserting test rows
   into a live database.
3. After registering, `route_fetch` the page: status, `rwe_component_errors`
   empty, and the expected text in the (possibly `truncated`) body.
