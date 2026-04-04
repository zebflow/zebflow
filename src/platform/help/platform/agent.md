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
| `version` | Returns the running platform version string |
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
| `pipeline_get` | Get pipeline graph JSON. Accepts partial path — resolves to unique match automatically. |
| `pipeline_describe` | Describe nodes, edges, trigger config in detail. Set `compact=true` for one-line-per-node summary without body content — use when pipelines have long SQL or scripts. |
| `pipeline_register` | Save a new pipeline from DSL body (stored as draft) |
| `pipeline_patch` | Update a node's config inside an existing pipeline. `node_id` accepts opaque ID, kind (`trigger.webhook`), or kind+index (`pg.query[1]`) — no describe needed |
| `pipeline_search` | Grep across all `.zf.json` pipeline files with optional glob filter and context lines |
| `pipeline_activate` | Promote draft to active — goes live immediately. Set `glob="pipelines/modules/**"` to bulk-activate all matching pipelines in one call. |
| `pipeline_deactivate` | Remove from active registry — stops serving traffic |
| `pipeline_execute` | Run the active version of a saved pipeline. Always pass `input` when testing function pipelines (`n.trigger.function`) — without it the pipeline receives `{}`. Accepts `input` as a JSON object or string. |
| `pipeline_run` | Run a pipeline body once — not saved, not logged. Pass `input` to provide an initial payload. |
| `pipeline_get_invocations` | Get recent execution history for a pipeline. Returns stored invocations with timestamp, duration, status, trigger, error, and per-node trace. Use this to inspect past runs or debug failing scheduled pipelines. |
| `git_command` | Run git: status, log, diff, add, commit. Commit author uses the user's configured `git_name` / `git_email` from their profile |

### Templates

| Tool | What it does |
|------|-------------|
| `template_list` | List all template files in the project |
| `template_get` | Read a template file's full content. Accepts partial path — resolves to unique match automatically (e.g. `"game/page"` → `pages/game/page.tsx`). |
| `template_create` | Scaffold a new template file with boilerplate |
| `template_write` | Write (overwrite) a template file's content |
| `template_search` | Grep across all template files with optional glob filter and context lines (e.g. `context=3` for 3 lines before/after each match) |
| `template_edit` | Exact string replacement inside a template file — `old_string` → `new_string`. Fails if `old_string` is not unique in the file |
| `move_resource` | Rename or reorganize a pipeline or template file. Domain auto-detected from extension (`.zf.json` = pipeline, else template). Pipeline lifecycle (deactivate → move → re-activate) handled automatically. Parent folders created. No cross-domain moves |

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
| `list_ui_catalog` | List all available shadcn-compatible UI components and whether each is installed |
| `install_ui_components` | Install one or more UI components into `shared/ui/` (e.g. `names=["button","card"]`) |

---

## Locked Resources

Project owners can lock individual pipelines or templates (and entire template folders) to prevent agent access. This is enforced **at the MCP layer only** — human web UI always works normally.

### What happens when a resource is locked

| Tool | Behavior |
|------|----------|
| `pipeline_list` | Still shows the locked pipeline — you can see it exists |
| `template_list` | Still shows the locked template/folder |
| `pipeline_get` | ❌ Error — locked |
| `pipeline_describe` | ❌ Error — locked |
| `pipeline_register` (update) | ❌ Error — locked |
| `pipeline_patch` | ❌ Error — locked |
| `pipeline_activate` | ❌ Error — locked |
| `pipeline_deactivate` | ❌ Error — locked |
| `template_get` | ❌ Error — locked |
| `template_write` | ❌ Error — locked |
| `template_create` (inside locked folder) | ❌ Error — locked |

Error message returned: `"This pipeline/template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it."`

### Template folder locking

Locking a folder path (e.g. `components/auth`) blocks access to all files under that prefix. You do not need to lock each file individually.

### You cannot unlock resources

Only the project owner can lock/unlock via the UI lock toggle button in the pipeline or template editor. If you encounter a locked resource that you need to modify, stop and inform the user.

---

## Know Exact Names Before You Use Them

Two values in pipelines are often hallucinated wrong — always use the actual value from the project:

| What you're writing | Source of truth | How to get it |
|---------------------|----------------|---------------|
| `web.response --template <path>` | exact `rel_path` from the project (always ends in `.tsx`, e.g. `pages/home.tsx`) | `template_list` |
| `--credential <slug>` on any node | exact `slug` from the project | `connection_list` |

**Rule:** If you already have the exact value in your current context (e.g. from a recent `template_list` or `connection_list` call), use it directly. If you're not certain, call the tool first. Never guess, never use memory from a different project.

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
| n.web.response --template pages/blog-home.tsx
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
