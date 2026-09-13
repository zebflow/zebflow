# Project operations

How a project is laid out and operated: files, agent docs, capabilities,
locks, git, probes. The MCP tools are described in `help("platform/agent")`;
the HTTP surface in `help("platform/api")`.

---

## Layout

```
users/{owner}/{project}/
├── repo/                    the git repository = the source root (unless spec.layout.source moves it)
│   ├── zebflow.yaml         ProjectConfiguration: layout, rwe.libraries, locks, policy
│   ├── zeb.lock             installed node bundles and zeb/* libraries
│   ├── globals.css          theme tokens
│   ├── api/  pages/  components/  scripts/  jobs/     pipelines (*.zf.json) and code — the folders are convention
│   ├── docs/                project documents
│   ├── static/              served at /static/{owner}/{project}/…
│   ├── schemas/sekejap/  schemas/sqlite/  initial-data/  nodes/  shared/ui/
├── data/
│   ├── store/               sekejap/ · local.db · kv.db · assistant/{user}/memory.md (MEMORY.md, per user)
│   ├── cache/               pipelines/ (live snapshots) · agent_docs/ (AGENTS.md, SOUL.md) · mapserver-artifacts/
│   ├── hub/                 nodes/ · rwe-libraries/
│   ├── logs/  recovery/
└── files/                   ZebFS objects (public/… is anonymous, the rest private)
```

`spec.layout` keys and defaults: `source` = `""` (the repo root), `static` =
`static`, `docs` = `docs`, `schema` = `schemas/sekejap`, `sqlite_schema` =
`schemas/sqlite`, `node_interfaces` = `nodes`. `spec.layout.allowed_extensions`
narrows which file types a Hub package may write; a package carrying anything
else is refused, not warned about.

---

## Agent docs

| File | Purpose | Written by |
|---|---|---|
| `AGENTS.md` | the project's rules and decisions — read first, they override the help | owner |
| `SOUL.md` | voice and tone for the in-Studio assistant | owner |
| `MEMORY.md` | what previous sessions did and what is open; one file per user | the agent |

`docs_agent_list` · `docs_agent_read name=` · `docs_agent_write name= content=`.
Never rewrite `AGENTS.md` unless asked. Everything else the project documents
is an ordinary file under `docs/` — `file_write rel_path="docs/schema.md"`.

---

## Understand the data first

When a project has database connections, one session of understanding saves
many of guessing:

```
connection_list                                       → slugs and kinds
connection_describe  slug=default-multimodel          → tables and columns (scope, schema, table narrow it)
file_write  rel_path=docs/schema.md  content=…        → the schema, written down once
docs_agent_write  name=MEMORY.md  content=…           → key tables, relations, auth pattern
```

`connection_describe` returns per-table `columns` with `name`, `type`,
`nullable`, and `pk` / `fk: { schema, table, column }` / `default` when known.
Then start every later session with `file_read rel_path=docs/schema.md`.

To look at real values, run a query — `pipeline_run` with the right node:

```
pipeline_run  body="| trigger.function | sekejap.query --limit 3 -- \"SELECT * FROM orders\""
pipeline_run  body="| trigger.function | pg.query --credential pg_main -- \"SELECT DISTINCT status FROM orders\""
```

Sample rows before writing queries; check counts before joining; in PostgreSQL
use `format()`/`concat()` rather than `||` inside a DSL body (the parser reads
`|` as a node separator); aggregate to one string with `string_agg(...)` when a
result would be too long for the tool window.

---

## The build loop

```
1. docs_agent_read AGENTS.md, MEMORY.md
2. file_read docs/schema.md            (or connection_describe → write it)
3. file_write docs/<feature>.md        the spec: routes, tables, pages — before code
4. pipeline_list, file_list            what already exists
5. pipeline_register                   draft
6. file_create → file_write            the page and its components
7. pipeline_activate                   live
8. fetch the route, search the body for "RWE component error", open it in a browser
9. git_command add + commit
10. docs_agent_write MEMORY.md         what was built, what was verified, what is open
```

Before using a node for the first time in a session: `help(topic="pipeline/nodes/<kind>")`.

---

## Channels

| Channel | Entry | For |
|---|---|---|
| MCP tools | `POST /api/projects/{o}/{p}/mcp` with the session's bearer token | agents (Claude Code, Cursor, Codex, …) |
| Studio | the web UI, including the project console (`register …`, `activate pipeline …`) | people |
| REST API | `/api/projects/{o}/{p}/…` with a session cookie | scripts, CI, integrations |

All three go through the same services, so locks and validation apply
everywhere.

---

## Capabilities

An MCP session token carries a set of capabilities; a tool outside the set
is refused with the missing capability named.

| Capability key | Tools |
|---|---|
| `project.read` | `start_here`, `help`, `help_search`, `skill_list`, `skill_read` |
| `pipelines.read` | `pipeline_list`, `pipeline_get`, `pipeline_describe`, `pipeline_search`, `pipeline_get_invocations`, `list_ui_catalog`, `hub_search`, `hub_review` |
| `pipelines.write` | `pipeline_register`, `pipeline_patch`, `pipeline_activate`, `pipeline_deactivate`, `git_command`, `install_ui_components`, `hub_add`, `move_resource` |
| `pipelines.execute` | `pipeline_execute`, `pipeline_run`, `route_fetch` |
| `templates.read` | `file_list`, `file_read`, `file_search`, `file_outline`, `file_deps` |
| `templates.create` / `templates.write` | `file_create` / `file_write`, `file_edit`, `file_batch_edit` |
| `tables.read` | `connection_list`, `connection_describe` |
| `credentials.read` | `credential_list` |
| `settings.read` / `settings.write` | `docs_agent_list`, `docs_agent_read` / `docs_agent_write` |

These keys are what `POST /mcp/session { "capabilities": [...] }` takes.

`version` needs no capability. The project owner sets the session's
capabilities in the Studio's session panel (the MCP button in the project
shell), which also shows the endpoint, the token and a client setup guide.

---

## Locks

An owner can lock a pipeline, a file, or a folder of files. The lock is
enforced in the service layer, so it holds for every channel — MCP, Studio
and the REST API alike — with `PLATFORM_PIPELINE_LOCKED` /
`PLATFORM_TEMPLATE_LOCKED`. Reading stays open in Studio; MCP refuses reads
of locked items too, while `pipeline_list` and `file_list` still show that
they exist. Agents cannot unlock anything.

- A pipeline lock lives in the pipeline file: `"metadata": { "locked": true }`; toggling it commits.
- File locks live in `zebflow.yaml` under `spec.locks.templates` as a list of paths; a folder path locks everything under it.

```yaml
spec:
  locks:
    templates:
      - components/auth
      - pages/admin.tsx
```

---

## Git

`repo/` is a git repository. Commit after each logical chunk:

```
git_command  subcommand=add     args="."
git_command  subcommand=commit  message="feat: blog pipeline and home page"
```

Allowed subcommands: `status`, `log`, `diff`, `add`, `commit`. Nothing that
rewrites history. The commit author is the user's profile (`git_name`,
`git_email`); a remote and pushes are configured in Studio → Settings → Git.

---

## Health and readiness probes

Zebflow answers the usual probes on the main port, and can run a dedicated
liveness listener when `ZEBFLOW_HEALTH_PORT` is set (use it for Kubernetes
liveness on office pods — it stays reachable when the main router is wedged
and answers 503 when the main runtime stops ticking).

| Endpoint | Purpose | Returns |
|---|---|---|
| `GET :10611/health/live` | the dedicated health thread is alive | `200 {"status":"ok","kind":"live","version":"…"}` |
| `GET :10611/health/runtime` | the main runtime heartbeat is fresh | `200 {"status":"ok"}` or `503 {"status":"stale"}` |
| `GET :10610/health` | compatibility liveness on the main router | `200 {"status":"ok","version":"…"}` |
| `GET :10610/ready` | the main app can serve — at least one SSR worker is alive | `200 {"status":"ready"}` or `503 {"status":"not_ready"}` |

```yaml
livenessProbe:
  httpGet: { path: /health/runtime, port: health }
  initialDelaySeconds: 30
  periodSeconds: 10
readinessProbe:
  httpGet: { path: /ready, port: http }
  initialDelaySeconds: 10
  periodSeconds: 5
  failureThreshold: 6
```

`SIGTERM` is handled gracefully: new connections stop, in-flight requests
finish, exit code 0.

---

## Webhook ingress

Every active `trigger.webhook` answers at `{method} /wh/{owner}/{project}{path}`;
`GET /wh/acme/shop/blog` runs the pipeline whose trigger declares
`--path /blog --method GET`. The DSL that registers it: `help("pipeline/dsl")`.
