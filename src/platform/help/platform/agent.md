# Zebflow — working in a project over MCP

A Zebflow project is pipelines plus the files they use. A pipeline connects a
trigger (HTTP route, schedule, WebSocket, MCP, function) to nodes (query,
script, HTTP, files, auth, AI) and answers with JSON or a server-rendered TSX
page. There is no build step: `file_write` a page, `pipeline_activate` the
route, fetch it.

---

## 1. Orient — every session

```
start_here                          ← project, pipelines and their status, files, connections, docs
docs_agent_read  name=AGENTS.md     ← the project's own rules; they win over anything below
docs_agent_read  name=MEMORY.md     ← what earlier sessions did and left open
```

Then, as the task needs: `pipeline_list`, `file_list`, `connection_list`,
`credential_list`. Write your goal into `MEMORY.md` before you start and what
you verified before you stop.

`start_here` ends with the **skills** — one line each. A skill is the
procedure for one kind of task: when to do what, in what order, and what
proves it worked. When a task matches one, `skill_read name="…"` before
acting (a reference file beside it: `skill_read name="…" path="references/x.md"`).
The blessed set ships with the platform and is published unchanged as
`github.com/zebflow/skills`; a client that already loaded it can skip the
`zebflow-*` bodies and read only the project's own `skills/<name>/`, which
shadow blessed ones by name. Project-specific guidance reaches you in three
places, all in `start_here`: **AGENTS.md** (the owner's rules, embedded in
full), **Project Docs** (every `.md` in the repository, `file_read` on demand)
and the project's **skills**.

| Task | Skill |
|---|---|
| any session, any completion claim | `zebflow-basic` |
| the first file of a project, a new domain or entity, "where does this go" | `zebflow-engineering` |
| a route, an API, a form's POST, a job | `zebflow-pipeline` |
| a page, a component, a script | `zebflow-rwe` |
| a screen built from components | `zebflow-ui` |
| tables, SQL, migrations | `zebflow-data` |
| login, roles, protected routes | `zebflow-auth` |
| uploads, images, rich text | `zebflow-files-editor` |
| proving it works | `zebflow-verify` |
| adding or publishing a package | `zebflow-hub` |
| drawing or modelling an asset — SVG, a Three.js mesh, a scene | `procedural-assets` (optional — `hub_add package_id=zebflow.skill-procedural-assets` first) |

Read the help topic before writing in a domain you have not used this session:

| Domain | Topic |
|---|---|
| pipelines and the DSL | `help(topic="pipeline")` → `pipeline/dsl`, `pipeline/authoring`, `pipeline/web` |
| nodes and their flags | `help(topic="pipeline/nodes")`, one node: `help(topic="pipeline/nodes/n.fs.save")` |
| pages | `help(topic="web")` → `web/hooks`, `web/ui`, `web/tailwind`, `web/libraries` |
| databases | `help(topic="db")`, `help(topic="db/sekejap")` |
| script helpers | `help(topic="tool")` |
| end-to-end recipes | `help(topic="pipeline/examples")` |
| the platform, API, operations | `help(topic="platform")` |

`help_search query="…"` searches every help page and every node definition.

---

## 2. Tools

**Pipelines**

| Tool | What it does |
|---|---|
| `pipeline_list` | index rows `file_rel_path | trigger | status | description`; filters `query`, `glob`, `status` (`active`, `stale`, `draft`, `all`), `trigger_kind`, `limit`; `format="json"` or `"tree"` |
| `pipeline_get` | the pipeline document (JSON) |
| `pipeline_describe` | the DSL with node ids (`n0`, `n1`, …); `compact=true` for one line per node |
| `pipeline_register` | save a DSL body as a draft at `file_rel_path` (`title`, `description` optional). Re-registering a live pipeline makes it `stale` |
| `pipeline_patch` | change one node's flags or body by `node_id`; the pipeline becomes `stale` |
| `pipeline_activate` / `pipeline_deactivate` | promote to live / stop serving; `glob="api/**"` activates many |
| `pipeline_execute` | run the live version with `input` |
| `pipeline_run` | run a DSL body once, unsaved — the way to test a query or a script |
| `pipeline_get_invocations` | recent runs of a live pipeline: status, duration, error, per-node trace |
| `pipeline_search` | grep across `.zf.json` files |

Status: **`active`** live and current · **`stale`** live but changed since
activation — run `pipeline_activate` · **`draft`** never activated.

**Files** — every file in the project's source root (pages, components,
scripts, CSS, docs, pipelines); paths are relative to that root.

| Tool | What it does |
|---|---|
| `file_list` | index rows `rel_path | kind | title | description`; `query`, `glob`, `kind`, `limit`; `format="tree"` |
| `file_read` | a file, or a line range with `offset`/`limit` |
| `file_outline` | imports, exports, functions of a `.tsx`/`.ts` — cheaper than reading it |
| `file_deps` | what a file imports and what imports it |
| `file_create` | scaffold: `kind` = page · component · script · style · doc · folder; the file lands at `parent_rel_path/name.<ext>` — pass `parent_rel_path="pages"` for `pages/<name>.tsx` |
| `file_write` | write the whole file (`rel_path`, `content`) |
| `file_edit` / `file_batch_edit` | exact `old_string` → `new_string` replacement, one file or many |
| `file_search` | grep across files |
| `move_resource` | rename or move a pipeline or file; a live pipeline is deactivated, moved and re-activated |

Project docs are files under `docs/` (`file_write rel_path="docs/schema.md"`).
`AGENTS.md`, `SOUL.md` and `MEMORY.md` are separate: `docs_agent_list`,
`docs_agent_read`, `docs_agent_write`.

**Data and credentials**

| Tool | What it does |
|---|---|
| `connection_list` | database connections: slug, label, kind. Every project has `default` (SQLite) and `default-multimodel` (Sekejap) |
| `connection_describe` | tables and columns of a connection; `scope`, `schema`, `table` narrow it |
| `credential_list` | credential ids, titles and kinds — values are never returned. `--credential`, `--auth-credential` and `mail.send --credential` take an **id from here**, not a connection slug |
| `list_ui_catalog` / `install_ui_components` | the clone-to-own component catalog (`shared/ui/`); pages import `zeb/ui/*` without installing anything |
| `route_fetch` | fetch one of the project's routes through the real ingress — status, `location`, `set_cookie`, `rwe_component_errors`, body; `method`, `form`, `body`, `cookie`, `headers`; the verification step |
| `hub_search` / `hub_review` / `hub_add` | the Hub shelf: what the project can add (optional skills, libraries, bundles), what an add would write, and the add itself — review before add, always |
| `git_command` | `subcommand` = status · log · diff · add · commit (`args`, `message`); the commit author is the user's profile |
| `skill_list` / `skill_read` | the skills: the list, one body, one reference file |
| `help`, `help_search`, `version` | knowledge and the platform version |

Any active pipeline whose entry is `n.trigger.mcp` also appears here as a
tool of its own, named by the pipeline.

---

## 3. Rules that save a session

- **Read exact names; never guess them.** `--template` is a `rel_path` from `file_list` ending in `.tsx`; `--credential` is an id from `credential_list`; a table name comes from `connection_describe`. A guessed template is a 500 at request time; a guessed credential id is an auth failure.
- **A node accepts only the flags it declares.** `help(topic="pipeline/nodes/<kind>")` before using an unfamiliar node.
- **Webhook data is under `input.body`.** A form field is `input.body.email`; path params `input.params`, query `input.query`. In `{{ }}` use `$trigger.params`, `$trigger.query`, `$trigger.auth` (no `body`).
- **Quote any flag value with `{{ }}` or a space** as one argument.
- **Draft is not live.** After `pipeline_register` or `pipeline_patch`, `pipeline_activate`. Then fetch the route (`/wh/{owner}/{project}{path}`) and look at what came back; `pipeline_get_invocations` shows the trace.
- **A 200 is not a rendered page.** A component that throws is replaced by `<!-- RWE component error: … -->` and the response is still 200. Search the body for it. A page whose hydration failed serves correct HTML and logs a browser console error — open it.
- **Every file imports what it uses** from `"zeb/react"`, `"zeb/ui/<name>"` or `"@/…"`; nothing is inherited from the page.
- **Locked resources** — an owner can lock a pipeline, a file or a folder. The lock holds at the service layer, so every write channel refuses (`PLATFORM_PIPELINE_LOCKED`, `PLATFORM_TEMPLATE_LOCKED`); MCP also refuses reads of locked items. `pipeline_list` and `file_list` still show they exist. You cannot unlock; tell the user.
- **Capabilities** — the MCP session may be narrowed (read-only, no git…). A refused tool names the missing capability; do not work around it.

---

## 4. A feature, end to end

```
connection_describe  slug=default-multimodel                       ← what tables exist
pipeline_run  body="| trigger.function | sekejap.query --read-only false -- \"CREATE TABLE posts (id TEXT, title TEXT, slug TEXT, body_json JSON, created_at TEXT)\""
file_create   kind=page  name=blog-home  parent_rel_path=pages
file_write    rel_path=pages/blog-home.tsx  content="…"           ← help(topic="web")
pipeline_register  file_rel_path="pages/blog-home"  title="Blog home"
                   body="| trigger.webhook --path /blog --method GET | sekejap.query -- \"SELECT id, title, slug, created_at FROM posts ORDER BY created_at DESC LIMIT 20\" | web.response --template pages/blog-home.tsx"
pipeline_activate  file_rel_path="pages/blog-home"
```

Fetch `/wh/{owner}/{project}/blog`, check for `RWE component error`, open it.
Then:

```
git_command  subcommand=add  args="."
git_command  subcommand=commit  message="feat: blog home"
docs_agent_write  name=MEMORY.md  content="… what was built, what was verified, what is open"
```
