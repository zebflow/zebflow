# Project Operations

Everything an agent needs to understand how a Zebflow project is structured and how to operate it.

---

## Project File Layout

```
{project-root}/
├── repo/
│   ├── zebflow.json              ← project config (title, assistant LLM settings)
│   ├── pipelines/                ← pipeline definitions (.zf.json)
│   ├── templates/
│   │   ├── pages/                ← full-page TSX templates
│   │   ├── components/
│   │   │   ├── ui/               ← design system (always use, never bypass)
│   │   │   ├── layout/           ← page shell / layout wrappers
│   │   │   └── behavior/         ← client-side behavior modules (.ts)
│   │   ├── scripts/              ← shared TS utility modules
│   │   └── styles/               ← CSS files (main.css, etc.)
│   └── docs/                     ← project docs (markdown)
│
├── data/
│   └── sekejap/                  ← project data
│       └── runtime/
│           └── agent_docs/       ← AGENTS.md, SOUL.md, MEMORY.md
│
└── files/                        ← public static assets (images, SVG, JSON)
```

---

## Agent Docs

Three special files every agent reads and writes:

| File | Purpose | Who writes |
|------|---------|------------|
| `AGENTS.md` | Project rules, tech decisions, conventions for all agents | Project owner |
| `SOUL.md` | Agent personality, tone, and communication style | Project owner |
| `MEMORY.md` | Persistent notes across sessions — what was done, what's next | Agent |

**Rules:**
- Read `AGENTS.md` at the start of every session — it overrides all skill docs.
- Read `MEMORY.md` to understand prior context before doing any work.
- Write to `MEMORY.md` after completing any significant work.
- Never overwrite `AGENTS.md` unless explicitly asked — it's the owner's configuration.

---

## Data Understanding Phase

**Always do this first when a project has DB connections.** One session of understanding saves many sessions of guessing.

```
1. list_connections                              → identify available DBs (slug, kind)
2. describe_connection slug=<slug>               → full schema: tables, columns, types, PKs, FKs
3. write_doc path=docs/schema.md                 → persist the schema for all future sessions
4. write_agent_doc MEMORY.md                     → note key tables, relations, auth pattern
```

`describe_connection` returns per-table `meta.columns`:
```json
{
  "name": "unit_id",
  "type": "uuid",
  "nullable": false,
  "fk": { "schema": "academic", "table": "academic_unit", "column": "unit_id" }
}
```

Fields present: `name`, `type`, `nullable`, optionally `pk: true`, `fk: {schema, table, column}`, `default`.

After writing `schema.md`, start every future session with `read_project_doc path=docs/schema.md` — instant full context, zero re-discovery queries.

### SQL Exploration Techniques

When `schema.md` doesn't exist yet, or you need to verify actual data patterns, use these techniques in order:

**1. Sample rows first — always before writing queries**
```sql
SELECT * FROM myschema.orders LIMIT 3
```
Reveals actual value formats, JSONB shapes, enum values, and NULLs before you guess.

**2. Inspect JSONB field keys**
```sql
SELECT DISTINCT jsonb_object_keys(metadata) FROM myschema.products LIMIT 5
-- or see a live example:
SELECT metadata FROM myschema.products LIMIT 1
```

**3. Discover enum/categorical values**
```sql
SELECT DISTINCT status FROM myschema.orders
SELECT DISTINCT role FROM myschema.users
```

**4. Check row counts before joining (avoid accidental cross joins)**
```sql
SELECT COUNT(*) FROM myschema.orders     -- 30k rows? 300?
SELECT COUNT(*) FROM myschema.order_items
```

**5. Avoid `||` pipe in SQL inside DSL** — the DSL parser treats `||` as a node separator.
Use `format()` or `concat()` instead:
```sql
-- ✗ WRONG in DSL:  first_name || ' ' || last_name
-- ✓ CORRECT:       format('%s %s', first_name, last_name)
-- ✓ CORRECT:       concat(first_name, ' ', last_name)
```

**6. Aggregate to a single string to bypass MCP row-count limits**
```sql
SELECT string_agg(format('%s: %s', id, name), E'\n' ORDER BY name) AS result
FROM myschema.categories
```
Returns one row → always fits in the MCP response window.

**7. PostgreSQL JSONB access patterns**
```sql
col->>'key'           -- text value (use for WHERE, ORDER BY, display)
col->'key'            -- jsonb value (use for nested access)
col->>'key' = 'val'   -- filter on JSONB text field
```

---

## Build Loop

A complete feature delivery follows this sequence:

```
1. read_agent_doc AGENTS.md                      → understand project rules
2. read_agent_doc MEMORY.md                      → understand prior state
3. read_project_doc path=docs/schema.md          → instant schema context (if exists)
   └─ if missing: describe_connection → write_doc docs/schema.md
4. write_doc path=docs/feature.md                → write spec / ERD before building
5. list_pipelines                                → understand existing logic
6. register_pipeline                             → build pipeline DSL (status: draft)
7. create_template → write_template              → build TSX UI
8. activate_pipeline                             → goes live
9. git_command add + commit                      → commit all changes
10. write_agent_doc MEMORY.md                    → record what was done and what's next
```

Always write a spec doc before building. Always commit after a logical chunk. Always update MEMORY.md before ending the session.

**RULE: Before writing DSL for any node you haven't used in this session → `read_skill pipeline-nodes` first.** Never guess flag names. The skill is the ground truth — not intuition, not naming conventions.

---

## Operational Channels

Three ways to interact with a project — same capability model enforces the same rules across all three:

| Channel | Entry point | Best for |
|---------|-------------|----------|
| **MCP Tools** | 25 structured tool calls | LLM agents (Cursor, Claude, etc.) |
| **Project Assistant** | `execute_pipeline_dsl` + DSL string | Interactive chat, exploratory/diagnostic work |
| **REST API** | `/api/projects/{owner}/{project}/...` | Programmatic access, CI/CD, integrations |

---

## Capability System

Every MCP session has scoped permissions. Your session token determines what tools you can call:

| Capability | Controls |
|-----------|---------|
| `PipelinesRead` | list, get, describe pipelines |
| `PipelinesWrite` | register, patch, activate, deactivate, git |
| `PipelinesExecute` | execute, run_ephemeral |
| `TemplatesRead` | list, get templates |
| `TemplatesWrite` | write_template |
| `TemplatesCreate` | create_template |
| `SettingsRead` | list/read agent docs |
| `SettingsWrite` | write agent docs |
| `TablesRead` | list_connections, describe_connection |
| `CredentialsRead` | list_credentials |

---

## Git Workflow

All project files under `repo/` are git-tracked. Commit after every logical chunk:

```
git_command  subcommand=add      args="."
git_command  subcommand=commit   message="feat: add blog pipeline and home page"
```

**Allowed:** `status`, `log`, `diff`, `add`, `commit`
**Blocked (safety):** `reset`, `rebase`, `force-push`, `checkout .`

Use descriptive commit messages. Convention: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`.

---

## Webhook Ingress

Activated pipelines with `trigger.webhook` are reachable at:

```
{method} /wh/{owner}/{project}/{webhook-path}
```

Example: `GET /wh/acme/my-app/blog` → triggers the blog-home pipeline → returns HTML.

---

## DSL Reference

Run `read_skill pipeline-dsl` for the full command reference.
Quick cheat:

```
register <name> --path <folder>  [DSL body]   ← save pipeline (draft)
activate pipeline <name>                       ← go live
deactivate pipeline <name>                     ← stop serving
execute pipeline <name>                        ← run saved active version
run [DSL body]                                 ← ephemeral, not saved
describe pipeline <name>                       ← inspect nodes + config
patch <name> --node <id> [flags]               ← update node config
git status / log / diff / add / commit         ← version control
get pipelines / templates / docs / tables      ← list resources
```
