# Pipeline DSL

Zebflow Pipeline DSL is a text-based command language for creating, managing, and executing pipelines and project resources.
It runs server-side and is accessible via the REST API, MCP tools, the in-app assistant, and the Pipeline CLI panel.
Its primary audience is LLM agents, but it works identically for humans.

---

## Security & Policy

**Zebflow is a whitelist, policy-based framework. All DSL commands are subject to capability checks.**

- Every command requires the caller to hold the appropriate project capability.
- Credential values are **never readable or writable via DSL** — credential management is UI-only. Any write attempt returns an error, not a suggestion.
- Git commands are a curated safe subset — destructive operations (`reset`, `rebase`, `force`, `checkout .`) are not exposed and cannot be called.
- Future: per-agent policy bindings will control which commands each agent session is allowed to run.

---

## Two Pipeline Modes

A pipeline is written in one of two modes. The parser auto-detects which one based on syntax.
Mixing is not allowed — pick one per pipeline.

### Pipe mode (linear chain)

Use `|` to chain nodes sequentially. No node labels needed.
Covers ~80% of pipelines: webhooks, scheduled jobs, simple data flows.

```zf
register blog-home --path /pages \
  | trigger.webhook --path /blog --method GET \
  | pg.query --credential main-db -- "SELECT * FROM posts ORDER BY created_at DESC" \
  | web.response --template pages/blog-home.tsx
```

### Graph mode (branching, fan-out, fan-in, loops)

Declare all nodes with `[id]` labels, then declare all edges separately.
Use when you need conditional routing, parallel fan-out, fan-in, or back-edges (loops).
Branching logic lives in `logic.*` nodes — edges are pure structural wiring, no conditions on edges.

```zf
register classify-ingest --path /webhooks \
  [a] trigger.webhook --path /ingest --method POST \
  [b] logic.switch --expr "input.type" --cases normal,urgent --default unknown \
  [c] sekejap.query --table normal_queue --op upsert \
  [d] http.request --url https://alerts.api/send --method POST \
  [e] sekejap.query --table unknown_queue --op upsert \
  [a] -> [b] \
  [b]:normal  -> [c] \
  [b]:urgent  -> [d] \
  [b]:unknown -> [e]
```

Both modes compile to the same JSON graph model.

---

## Syntax Basics

```
<command> [<resource>] [<name>] [--flag value] [-- <body>]
```

- **Multiline**: end a line with `\` to continue on the next line.
- **Chaining**: `&&` runs the next command only if the previous succeeded (unix-style).
- **Body**: `-- <content>` passes inline content (script source, SQL, JSON input, etc.).
- **Help**: append `--help` to any command or node name to show its usage.

### Flag value kinds

| Kind | Example | Produces |
|------|---------|---------|
| Scalar | `--template pages/foo.tsx` | `"pages/foo.tsx"` string |
| Comma list | `--auth-required-role admin,lecturer` | `["admin","lecturer"]` array |
| Bool | `--http-only` | `true` (no value consumed) |
| Key-value pairs | `--claim name=$.name --claim roles=$.roles` | `{"name":"$.name","roles":"$.roles"}` object |

Key-value pairs (`--claim`, `--header`, `--set-cookie`) repeat the **same flag with a different key** each time — each occurrence adds one entry to a map.
Comma lists repeat **values for the same key** — pass them all in one flag, comma-separated.
There is no "repeat the same flag for a list" — `--role admin --role lecturer` is not valid; use `--role admin,lecturer`.

---

## Dynamic Config Expressions — `{{ expr }}`

Any string field in a node's config can contain `{{ js_expr }}` placeholders.
The engine resolves them **before** the node runs, in a hermetically sandboxed Deno context.

### Scope variables

| Variable          | Contents                                                        |
|-------------------|-----------------------------------------------------------------|
| `$input`          | The current payload flowing into this node                      |
| `$input.field`    | Specific field from the upstream node's output                  |
| `$trigger.auth`   | Verified JWT claims from the original request (full claims, not filtered) |
| `$trigger.params` | URL path params (`:id`, `:slug`, etc.)                          |
| `$trigger.query`  | Query string params (`?page=2` etc.)                            |
| `$trigger.headers`| Safe subset of request headers (content-type, user-agent, etc.) |
| `$nodes.id`       | Output payload of a completed upstream node by its graph ID     |
| `$nodes.id.field` | Specific field from that node's output                          |
| `$ctx.pipeline`   | Current pipeline identifier                                     |
| `$ctx.request_id` | Unique execution request id                                     |

> **`$trigger.auth` vs `ctx.auth` in templates**: In script node `{{ expr }}` expressions, `$trigger.auth` holds the full decoded JWT claims (all claims, including private). In templates rendered by `n.web.response`, `ctx.auth` holds only the `:public`-marked claims after filtering. Use `$trigger.auth` in expressions inside script/query configs; use `ctx.auth` in template TSX files.

### Type preservation

Whole-field expressions (`{{ expr }}` with no surrounding text) → **native JS type** (object, array, number, boolean).
Interpolated expressions (`"Hello {{ name }}!"`) → **string** (result is stringified and concatenated).

### Sandbox security

Expressions run with `capabilities: []` — the `n.*` bridge is completely disabled.
No database access, no HTTP, no side effects. Pre-existing locks on `eval`, `Function`, `fetch`, and timers remain in effect.
A tight op budget (`maxOps: 500`) prevents runaway computation.

### Examples

```zf
# Use path param in a Postgres query
| trigger.webhook --path /users/:id --method GET
| pg.query --credential main-db \
    --params-expr "{{ [$trigger.params.id] }}" \
    -- "SELECT * FROM users WHERE id = $1"

# Build a URL from upstream node output
| http.request --url "https://api.example.com/{{ $nodes.lookup.rows[0].slug }}"

# Pass upstream data as JSON body
| http.request --url https://notify.svc/send --method POST \
    --body-expr "{{ { userId: $trigger.auth.sub, data: $input } }}"

# Conditional auth redirect
| script -- "return { target: $trigger.query.next || '/dashboard' }"
```

---

## Pipeline Commands

Pipelines are the primary resource. They have a **lifecycle status**:

| Status | Meaning |
|---|---|
| `draft` | Registered but never activated |
| `active` | Live and serving traffic; active snapshot matches current source |
| `stale` | Live but source has changed since last activation (needs re-activate) |
| `inactive` | Explicitly deactivated; source retained |

### get pipelines

```zf
get pipelines
get pipelines --path /webhooks
get pipelines --status active
get pipelines --path /jobs --status stale
```

### describe pipeline

Returns full graph with **node IDs**, edge wiring, status, and hit stats.
Node IDs from `describe` are used with `patch pipeline <name> node <id>`.

```zf
describe pipeline blog-home
```

Example output:
```
pipeline: blog-home
path:     /pages
status:   active (hash: a3f2b1)
trigger:  webhook → /blog GET

nodes:
  [a] n.trigger.webhook   path=/blog method=GET
  [b] n.pg.query          credential_id=main-db
  [c] n.web.response        template=pages/blog-home.tsx route=/blog

edges:
  [a]:out → [b]:in
  [b]:out → [c]:in

hits: 142 ok / 0 failed
```

### register — create or update full pipeline DSL

```zf
# Pipe mode
register <name> [--path <virtual-path>] [--title <title>]
  | <node-kind> [--flag val] [-- <body>]
  | <node-kind> ...

# Graph mode
register <name> [--path <virtual-path>] [--title <title>]
  [id] <node-kind> [--flag val] [-- <body>]
  [id] <node-kind> ...
  [id] -> [id]
  [id]:pin -> [id]:pin
  ...

# Return JSON without persisting
register <name> --json | ...
```

### patch pipeline — targeted update

Patch metadata or one node without rewriting the full graph:

```zf
# Metadata only
patch pipeline blog-home --path /new-path --title "Blog v2"

# One node config flag — by opaque ID
patch pipeline blog-home node b --credential new-db

# One node config flag — by node kind (no describe needed)
patch pipeline blog-home node trigger.webhook --auth-type jwt --auth-credential my-jwt

# By kind + index when multiple nodes of the same kind exist
patch pipeline blog-home node pg.query[1] -- "SELECT id, title FROM posts WHERE published = true"

# One node body (e.g. SQL)
patch pipeline blog-home node b -- "SELECT id, title, slug FROM posts WHERE published = true"
```

`node_id` accepts: opaque ID (`b`, `n0`), kind (`trigger.webhook`, `pg.query`), or kind+index (`pg.query[0]`, `pg.query[1]`).
When using kind, the first matching node is used. If multiple nodes match with no index, an error lists the options.
After patching, the pipeline is `stale` until re-activated.

### activate / deactivate

```zf
activate pipeline blog-home
deactivate pipeline blog-home
```

### execute — trigger a registered pipeline

```zf
execute pipeline blog-home
execute pipeline process-order -- {"order_id": 42, "action": "resend"}
```

Triggers with the declared trigger kind. Use `-- {json}` to pass input payload (manual trigger pipelines).

### delete pipeline

```zf
delete pipeline blog-home
```

### run — ephemeral inline pipeline (implicit trigger.manual)

`run` is always a one-shot manual trigger. Not persisted.

```zf
# Pipe mode
run | pg.query --credential main-db -- "SELECT count(*) FROM users"

run \
  | http.request --url https://example.com --method GET \
  | sekejap.query --table results --op upsert

# Graph mode
run \
  [a] http.request --url https://example.com --method GET \
  [b] logic.if --expr "input.status >= 400" \
  [c] sekejap.query --table errors --op upsert \
  [a] -> [b] \
  [b]:true -> [c]

# Return JSON without executing
run --dry-run | pg.query --credential main-db -- "SELECT 1"
```

---

## Connection Commands

DB connections are first-class. Explore them before writing SQL nodes.

### get connections

```zf
get connections
```

Returns: slug, label, kind (postgres, mysql, sekejap) for each connection.
The **slug** is what `--credential` references in `pg.query` and `sekejap.query` nodes.

### describe connection — traverse schema

```zf
describe connection main-db                              # full tree
describe connection main-db --scope schemas              # schemas only
describe connection main-db --scope tables               # all tables
describe connection main-db --scope tables --schema public  # one schema
describe connection main-db --scope functions            # stored functions
```

Use this before writing SQL — confirms table names, column types, constraints.

### delete connection

```zf
delete connection old-db
```

Connection create/update is UI-only. The DSL cannot write connection config.

---

## Credential Commands

```zf
get credentials    # names + kinds only — values are never exposed
```

**Credential create/update/delete is blocked in DSL — use the UI.**
Any write attempt returns: `credentials can only be managed from the UI`.

### jwt_signing_key credential fields

| Field | Description |
|---|---|
| `algorithm` | Signing algorithm: `HS256`, `HS384`, `HS512`, `RS256`, `RS384`, `RS512`, `ES256`, `ES384` |
| `secret` | Shared secret for HS* algorithms |
| `private_key` | PEM private key for RS*/ES* algorithms |
| `auth_redirect` | Path to redirect to when a protected webhook receives a **missing or invalid token** (e.g. `/login`). Leave blank to return 401 JSON. |
| `auth_forbidden_redirect` | Path to redirect to when the token is valid but the **roles are insufficient** (e.g. `/403`). Leave blank to return 403 JSON. |

---

## Node Catalog

```zf
get nodes
get nodes --filter logic
describe node n.pg.query        # config shape, input/output pins, description
n.logic.switch --help           # same
```

### Data & compute nodes

| Short name | Full kind | Config flags |
|---|---|---|
| `trigger.webhook` | `n.trigger.webhook` | `--path <path> --method <GET\|POST\|...> [--auth-type jwt\|hmac\|api_key] [--auth-credential <id>] [--auth-required-role <roles>]` |
| `trigger.schedule` | `n.trigger.schedule` | `--cron <expr> --timezone <tz>` |
| `trigger.manual` | `n.trigger.manual` | _(none)_ |
| `script` | `n.script` | `--lang <js\|ts>` or `-- <code>` |
| `web.response` | `n.web.response` | `--template <pages/name>` (no `.tsx`), `--status`, `--location`, `--message`, `--body <$.path>`, `--set-cookie`, `--header <key=value>`, `--load-scripts <urls>` |
| `http.request` | `n.http.request` | `--url <url> --method <GET\|POST> [--timeout-ms <ms>] [--header <key=value> ...] [--merge-input]` |
| `sekejap.query` | `n.sekejap.query` | `--table <name> --op <query\|upsert>` |
| `pg.query` | `n.pg.query` | `--credential <credential-slug>` (**credential slug** from `get credentials`, kind=postgres) `[--params-path <dot.path>] [--params-expr <js-expr>] [--credential-expr <js-expr>] [--query-expr <js-expr>]` + `-- <sql>` |
| `auth.token.create` | `n.auth.token.create` | `--credential <jwt_key_id> [--expires-in <secs>] [--claim key=$.field ...] [--issuer <iss>] [--audience <aud>]` — append `:public` to a claim value to expose it in the browser via `ctx.auth` (e.g. `--claim name=$.fullname:public`). Use `--claim roles=$.roles:public` where `roles` is an array — role-based access control always uses the `roles` array claim. Claims without `:public` are signed but never reach the browser DOM. Secure by default — `ctx.auth` is `null` unless at least one claim is marked public. |
| `ai.zebtune` | `n.ai.zebtune` | `--budget <n> --output <mode>` |
| `trigger.ws` | `n.trigger.ws` | `--event <name> --room <id>` |
| `ws.emit` | `n.ws.emit` | `--event <name> --to <all\|session\|others> --payload-path <ptr> --room <id>` |
| `ws.sync_state` | `n.ws.sync_state` | `--op <set\|merge\|delete> --path <ptr> --value-path <ptr> --room <id>` |

### `n.trigger.webhook` — request payload shape

The webhook trigger normalises all request bodies to a flat JSON object:

| Source | Where it appears in payload |
|---|---|
| JSON body (`application/json`) | Fields merged to root |
| Form body (`application/x-www-form-urlencoded`) | Fields merged to root (percent-decoded) |
| Multipart form text fields | Fields merged to root |
| Multipart form files | Under `input.files.{field}` as `{ filename, content_type, size, data }` (data = base64) |
| Query string params | Merged to root **and** available at `input.query` |
| URL path params | Available at `input.params` |
| Verified JWT claims | Available at `input.auth` (downstream from trigger only — see `ctx.trigger.auth` for all nodes) |

### `n.trigger.webhook` — authentication flags

| Flag | Description |
|---|---|
| `--auth-type jwt` | Verify JWT. Checks `Authorization: Bearer <token>` first, then falls back to `Cookie: zebflow_session`. |
| `--auth-type hmac` | Verify HMAC-SHA256 signature in `X-Hub-Signature-256` header (GitHub webhook style). |
| `--auth-type api_key` | Verify static API key in `X-API-Key` header. |
| `--auth-credential <id>` | Credential ID holding the signing key/secret. Required when `--auth-type` is not `none`. |
| `--auth-required-role <roles>` | Comma-separated list of required roles. The JWT `roles` array claim must contain at least one. Empty = any authenticated user. |

**On auth failure:**
- Missing or invalid token → `auth_redirect` path from the credential (if set), or 401 JSON.
- Valid token, wrong role → `auth_forbidden_redirect` path from the credential (if set), or 403 JSON.

`auth_redirect` and `auth_forbidden_redirect` are properties of the **JWT signing key credential**, not node flags — configure them in the UI credentials panel.

### Logic / control-flow nodes (graph mode only)

| Short name | Full kind | Output pins | Config |
|---|---|---|---|
| `logic.if` | `n.logic.if` | `true`, `false` | `--expr <js-expression>` |
| `logic.switch` | `n.logic.switch` | named cases + default | `--expr <js-expression> --cases a,b,c --default <name>` |
| `logic.branch` | `n.logic.branch` | named branches | `--fanout a,b,c` or `--expr <js-expression>` |
| `logic.merge` | `n.logic.merge` | `out` | `--strategy wait_all\|first_completed\|pass_through` |

External nodes use `x.` prefix (e.g. `x.firebase.notify`).

---

## Template Commands

```zf
get templates
get templates --path /components
describe template components/ui/button.tsx   # file contents
delete template components/ui/old-button.tsx
```

---

## File Commands

```zf
get files [--scope public|private]
read file public/logo.png
write file public/data.json -- '{"key":"value"}'
delete file private/old-export.csv
```

---

## Doc Commands

Project docs live in `repo/docs/` (git-synced).

```zf
get docs
read doc README.md
read doc erd.svg
write doc AGENTS.md -- "# Agents\n..."
```

---

## Git Commands

The `repo/` folder is git-synced. Use git commands to track and commit changes made via DSL.
**Only safe operations are exposed. Destructive git operations are not available.**

```zf
git status                     # modified, staged, untracked files under repo/
git log [--limit 10]           # recent commits (message, hash, author, date)
git diff [<path>]              # unstaged changes
git add <path>                 # stage a file
git commit -- "message"        # commit all staged changes
```

Blocked (not exposed): `reset`, `rebase`, `force`, `checkout .`, `clean`, `branch -D`.

---

## Graph Mode: Routing & Control Flow

All branching logic lives in `logic.*` nodes — not on edges. Edges are pure pin-to-pin wiring.

### Edge syntax

```zf
[from] -> [to]           # default out pin → default in pin
[from]:pin -> [to]       # named output pin → default in pin
[from]:pin -> [to]:pin   # named output pin → named input pin
```

Omitting `:pin` defaults to `out` for the source, `in` for the target.

### logic.if — binary branch

```zf
register check-status --path /webhooks \
  [a] trigger.webhook --path /status --method GET \
  [b] http.request --url https://example.com --method GET \
  [c] logic.if --expr "input.status >= 400" \
  [d] http.request --url https://hooks.slack.com/xxx --method POST \
  [a] -> [b] \
  [b] -> [c] \
  [c]:true -> [d]
```

`[c]:false` has no edge → execution stops silently (no error). That's your "do nothing" branch.

### logic.switch — multi-case routing

```zf
register event-router --path /webhooks \
  [a] trigger.webhook --path /events --method POST \
  [b] logic.switch --expr "input.type" --cases create,update,delete --default unknown \
  [c] script --lang js -- "return handleCreate(input);" \
  [d] script --lang js -- "return handleUpdate(input);" \
  [e] script --lang js -- "return handleDelete(input);" \
  [f] sekejap.query --table unknown_events --op upsert \
  [a] -> [b] \
  [b]:create  -> [c] \
  [b]:update  -> [d] \
  [b]:delete  -> [e] \
  [b]:unknown -> [f]
```

### logic.branch — parallel fan-out

```zf
register notify-all --path /jobs \
  [a] trigger.schedule --cron "0 9 * * *" --timezone UTC \
  [b] pg.query --credential main-db -- "SELECT * FROM alerts WHERE active = true" \
  [c] logic.branch --fanout email,sms,slack \
  [d] http.request --url https://email.api/send --method POST \
  [e] http.request --url https://sms.api/send --method POST \
  [f] http.request --url https://hooks.slack.com/xxx --method POST \
  [a] -> [b] \
  [b] -> [c] \
  [c]:email -> [d] \
  [c]:sms   -> [e] \
  [c]:slack -> [f]
```

### logic.merge — fan-in

Strategies: `wait_all` (all pins before firing), `first_completed` (first arrival wins), `pass_through` (fires on each, default).

```zf
register parallel-fetch --path /jobs \
  [a] trigger.manual \
  [b] http.request --url https://source-a.com --method GET \
  [c] http.request --url https://source-b.com --method GET \
  [d] logic.merge --strategy wait_all \
  [e] script --lang js -- "return combine(input.in_b, input.in_c);" \
  [a] -> [b] \
  [a] -> [c] \
  [b]:out -> [d]:in_b \
  [c]:out -> [d]:in_c \
  [d] -> [e]
```

### Loops (back-edges)

```zf
register retry-job --path /jobs \
  [a] trigger.manual \
  [b] script --lang js \
       -- "const n=(input.attempts||0)+1; return {...doWork(input), attempts:n};" \
  [c] logic.switch --expr "input.status" --cases done,failed --default retry \
  [d] script --lang js -- "return { result: input };" \
  [e] sekejap.query --table failures --op upsert \
  [f] logic.if --expr "input.attempts < 5" \
  [a] -> [b] \
  [b] -> [c] \
  [c]:done   -> [d] \
  [c]:failed -> [e] \
  [c]:retry  -> [f] \
  [f]:true   -> [b] \
  [f]:false  -> [e]
```

`[f]:true -> [b]` is the back-edge (loop). `[f]:false` routes to failures when max attempts exceeded.

---

## Typical Agent Workflow

```zf
# 1. Understand the project
get connections
get credentials
get pipelines
git log --limit 5

# 2. Explore the DB before writing SQL
describe connection main-db --scope tables --schema public

# 3. Test a query
run | pg.query --credential main-db -- "SELECT id, title FROM posts LIMIT 5"

# 4. Register the pipeline
register get-posts --path /api \
  | trigger.webhook --path /posts --method GET \
  | pg.query --credential main-db -- "SELECT id, title, created_at FROM posts ORDER BY created_at DESC"

# 5. Activate and verify
activate pipeline get-posts
describe pipeline get-posts

# 6. Commit changes
git status
git add pipelines/api/get-posts.json
git commit -- "add get-posts pipeline"
```

---

## JSON Equivalent

Both pipeline modes compile to the same `PipelineGraph` JSON. Edges are pin-to-pin — no conditions on edges:

```json
{
  "kind": "zebflow.pipeline",
  "version": "0.1",
  "id": "event-router",
  "entry_nodes": ["a"],
  "nodes": [
    { "id": "a", "kind": "n.trigger.webhook",  "input_pins": [],     "output_pins": ["out"],                        "config": { "path": "/events", "method": "POST" } },
    { "id": "b", "kind": "n.logic.switch",     "input_pins": ["in"], "output_pins": ["create","update","unknown"],  "config": { "expression": "input.type", "cases": ["create","update"], "default": "unknown" } },
    { "id": "c", "kind": "n.script",           "input_pins": ["in"], "output_pins": ["out"],                        "config": { "language": "js", "source": "return handleCreate(input);" } }
  ],
  "edges": [
    { "from_node": "a", "from_pin": "out",    "to_node": "b", "to_pin": "in" },
    { "from_node": "b", "from_pin": "create", "to_node": "c", "to_pin": "in" }
  ]
}
```

No `condition` on edges. Routing is entirely expressed via named output pins on logic nodes.

---

## Command Chaining

```zf
get pipelines && get connections

describe pipeline blog-home && activate pipeline blog-home
```

---

## CLI vs API vs MCP

| Interface | How |
|---|---|
| Pipeline CLI panel | Type commands directly in the panel |
| REST API | `POST /api/projects/{owner}/{project}/pipelines/dsl` with `{ "dsl": "..." }` |
| MCP tool | `execute_pipeline_dsl` tool with `dsl` param |
| Assistant | Natural language → assistant generates and submits DSL |

See also: **pipeline-dsl-web** (HTML pages + `n.web.response`), **pipeline-dsl-web-auto** (web.auto language).
