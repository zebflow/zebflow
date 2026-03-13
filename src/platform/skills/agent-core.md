# Zebflow Agent Core

Zebflow is a pipeline-based reactive web platform. Pipelines connect trigger nodes to action nodes — they produce REST APIs, web pages, scheduled jobs, and webhooks with zero build step and zero deploy. The RWE engine compiles TSX templates server-side, hydrates them client-side, and serves them through activated pipelines.

---

## Phase 1: Connect Protocol

**Always run these in order at the start of every project session:**

```
list_agent_docs
read_agent_doc  name=AGENTS.md    ← project-specific rules (required reading)
read_agent_doc  name=MEMORY.md    ← what happened in previous sessions
list_pipelines                    ← understand existing logic
list_templates                    ← understand existing UI
list_tables                       ← understand data model
```

After reviewing, update MEMORY.md with your session goals before starting work.
If AGENTS.md contradicts any skill doc, follow AGENTS.md.

---

## MCP Tools

### Pipelines

| Tool | What it does |
|------|-------------|
| `list_pipelines` | List all pipelines with status (draft / active) |
| `get_pipeline` | Get pipeline graph JSON |
| `describe_pipeline` | Describe nodes, edges, trigger config in detail |
| `register_pipeline` | Save a new pipeline from DSL body (stored as draft) |
| `patch_pipeline` | Update a node's config inside an existing pipeline |
| `activate_pipeline` | Promote draft to active — goes live immediately |
| `deactivate_pipeline` | Remove from active registry — stops serving traffic |
| `execute_pipeline` | Run the active version of a saved pipeline |
| `run_ephemeral` | Run a pipeline body once — not saved, not logged |
| `git_command` | Run git: status, log, diff, add, commit |

### Templates

| Tool | What it does |
|------|-------------|
| `list_templates` | List all template files in the project |
| `get_template` | Read a template file's full content |
| `create_template` | Scaffold a new template file with boilerplate |
| `write_template` | Write (overwrite) a template file's content |

### Docs

| Tool | What it does |
|------|-------------|
| `list_project_docs` | List markdown docs in repo/docs/ |
| `read_project_doc` | Read a doc file |
| `write_doc` | Write a doc (spec, ERD, README, CHANGELOG, ADR) |

### Agent Docs

| Tool | What it does |
|------|-------------|
| `list_agent_docs` | List AGENTS.md, SOUL.md, MEMORY.md |
| `read_agent_doc` | Read one agent doc by name |
| `write_agent_doc` | Write an agent doc |

### Knowledge

| Tool | What it does |
|------|-------------|
| `list_skills` | List all available skill docs |
| `read_skill` | Read a skill doc in full |

### Data

| Tool | What it does |
|------|-------------|
| `list_tables` | List Simple Tables in this project |

---

## The 3 Domains

Master these before building anything:

| Domain | Command | Covers |
|--------|---------|--------|
| **Pipeline DSL** | `read_skill pipeline-dsl` | All commands, pipe mode, graph mode, branching, git, connections |
| **RWE Templates** | `read_skill rwe-templates` | TSX structure, hooks, component library, import rules, hydration |
| **Project Operations** | `read_skill project-operations` | File layout, agent docs, build loop, channels, git workflow |

Supporting skills: `pipeline-nodes`, `pipeline-authoring`, `pipeline-dsl-rwe`, `sekejapql`, `api-reference`

---

## Quick Example: Full Stack Feature

### 1. Define the pipeline (DSL body)

```
| trigger.webhook --path /blog --method GET
| pg.query --credential main-db -- "SELECT id, title, created_at FROM posts ORDER BY created_at DESC LIMIT 20"
| web.render --template pages/blog-home --route /blog
```

Pass this as `body` to `register_pipeline name=blog-home`.

### 2. Create the template

```
create_template  kind=page  name=blog-home
```

Then `write_template rel_path=pages/blog-home.tsx` with TSX content.
See `read_skill rwe-templates` for TSX conventions.

### 3. Activate and commit

```
activate_pipeline  name=blog-home
git_command  subcommand=add  args="."
git_command  subcommand=commit  message="feat: blog home page"
write_agent_doc  name=MEMORY.md  content="..."
```
