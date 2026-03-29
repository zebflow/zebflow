# Zebflow Agent Core

Zebflow is a pipeline-based platform. Pipelines connect triggers to actions — REST APIs, **web pages (TSX)**, cron jobs, and webhooks, without a separate frontend build step in the project. TSX files under the project are rendered to HTML on the server and can hydrate in the browser.

---

## Phase 1: Orient (Call First)

**Always call `start_here` at the start of every session.**

```
start_here     ← overview, project name, docs list, connections, template tree
```

Then based on what you need:

```
docs_agent_read  name=AGENTS.md    ← project-specific rules (required reading)
docs_agent_read  name=MEMORY.md    ← what happened in previous sessions
pipeline_list                      ← understand existing logic
template_list                      ← understand existing UI
connection_list                    ← understand data sources
```

After reviewing, update MEMORY.md with your session goals before starting work.
If AGENTS.md contradicts any skill doc, follow AGENTS.md.

---

## MCP Tools

### Orientation

| Tool | What it does |
|------|-------------|
| `start_here` | First call — returns overview, project context, doc list, connections, template tree |
| `help` (no topic) | Full help index — all available topics |
| `help(topic="pipeline")` | Pipeline DSL guide — syntax, pipe mode, web patterns, examples + live node appendix |
| `help(topic="web")` | Web pages — TSX templates, server render, hooks, pipeline `input` |
| `help(topic="pipeline/examples")` | Project archetypes — blog, forum, game, scheduling, scraping, auth (with full DSL) |
| `help(topic="pipeline/nodes")` | Node catalog — all nodes with flags and schemas (live from Rust) |
| `help(topic="pipeline/nodes/{kind}")` | One node — e.g. `topic="pipeline/nodes/n.trigger.webhook"` |
| `help_search` | Search all help docs for a concept, node name, or DSL syntax |

### Pipelines

| Tool | What it does |
|------|-------------|
| `pipeline_list` | List all pipelines with status (draft / active) |
| `pipeline_get` | Get pipeline graph JSON |
| `pipeline_describe` | Describe nodes, edges, trigger config in detail |
| `pipeline_register` | Save a new pipeline from DSL body (stored as draft) |
| `pipeline_patch` | Update a node's config inside an existing pipeline |
| `pipeline_activate` | Promote draft to active — goes live immediately |
| `pipeline_deactivate` | Remove from active registry — stops serving traffic |
| `pipeline_execute` | Run the active version of a saved pipeline |
| `pipeline_run` | Run a pipeline body once — not saved, not logged |
| `git_command` | Run git: status, log, diff, add, commit |

### Templates

| Tool | What it does |
|------|-------------|
| `template_list` | List all template files in the project |
| `template_get` | Read a template file's full content |
| `template_create` | Scaffold a new template file with boilerplate |
| `template_write` | Write (overwrite) a template file's content |

### Docs

| Tool | What it does |
|------|-------------|
| `docs_project_list` | List markdown docs in repo/docs/ |
| `docs_project_read` | Read a doc file |
| `docs_project_write` | Write a doc (spec, ERD, README, CHANGELOG, ADR) |

### Agent Docs

| Tool | What it does |
|------|-------------|
| `docs_agent_list` | List AGENTS.md, SOUL.md, MEMORY.md |
| `docs_agent_read` | Read one agent doc by name |
| `docs_agent_write` | Write an agent doc |

### Knowledge

| Topic | What it covers |
|-------|---------------|
| `help(topic="pipeline")` | Pipeline DSL, nodes, examples |
| `help(topic="web")` | TSX templates, hooks, UI kit, Tailwind |
| `help(topic="db")` | Database connections, SekejapQL |
| `help(topic="tool")` | Tool.* globals (time, arr, stat, geo) |
| `help(topic="platform")` | Platform API, operations, agent workflow |

### Connections & Credentials

| Tool | What it does |
|------|-------------|
| `connection_list` | List DB connections (slug, label, kind) |
| `connection_describe` | Describe DB schema — tables, columns, types |
| `credential_list` | List credentials (id, title, kind — values never exposed) |

---

## The 3 Domains

Master these before building anything:

| Domain | Tool | Covers |
|--------|------|--------|
| **Pipeline DSL** | `help(topic="pipeline")` | All commands, pipe mode, graph mode, branching, git, connections |
| **Web templates** | `help(topic="web")` | TSX layout, hooks, UI kit install, import rules, hydration |
| **Project Operations** | `help(topic="platform/operations")` | File layout, agent docs, build loop, channels, git workflow |

Node details (live from Rust): `help(topic="pipeline/nodes")` for full catalog, `help(topic="pipeline/nodes/{kind}")` for one node.

---

## Sekejap — Embedded Database

Zebflow's built-in multi-model database. Capabilities:
- **Graph** traversal, **vector** similarity, **spatial** queries
- **Full-text** search (if `fulltext_fields` defined on table)
- **Vague temporal** queries

Suitable for: blog posts, user tables, AI memory, vector embeddings, event graphs, RAG indexes.

**Workflow:**
1. Create a table in the UI (Tables page) — give it a slug and field definitions
2. Use `n.sekejap.query` in pipelines to query or upsert rows

**Pipeline node (DSL):**
```
| n.sekejap.query --table posts --op query
| n.sekejap.query --table posts --op upsert
```

**Direct query (run_db_query / connection_describe):**
- Connection kind: `sekejap` (already available in every project, no config needed)
- Query language: SekejapQL text DSL — `collection "sjtable__posts" | take 50`
- Collections use internal prefix `sjtable__`: table `posts` → collection `sjtable__posts`

See `help(topic="db/sekejap")` for the full query language reference.

---

## Quick Example: Full Stack Feature

### 1. Define the pipeline (DSL body)

```
| trigger.webhook --path /blog --method GET
| pg.query --credential main-db -- "SELECT id, title, created_at FROM posts ORDER BY created_at DESC LIMIT 20"
| n.web.response --template pages/blog-home
```

Pass this as `body` to `pipeline_register` with a canonical `file_rel_path` (e.g. `pipelines/pages/blog-home.zf.json`).

### 2. Create the template

```
template_create  kind=page  name=blog-home
```

Then `template_write rel_path=pages/blog-home.tsx` with TSX content.
See `help(topic="web")` for TSX conventions.

### 3. Activate and commit

```
pipeline_activate  file_rel_path=pipelines/pages/blog-home.zf.json
git_command  subcommand=add  args="."
git_command  subcommand=commit  message="feat: blog home page"
docs_agent_write  name=MEMORY.md  content="..."
```
