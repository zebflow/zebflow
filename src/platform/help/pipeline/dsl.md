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
  [b] logic.match --expr "$input.type" --cases normal,urgent --default unknown \
  [c] sekejap.query --params-expr "[$input.id, $input.data]" --read-only false -- "INSERT INTO normal_queue (id, data) VALUES ($1, $2)" \
  [d] http.request --url https://alerts.api/send --method POST \
  [e] sekejap.query --params-expr "[$input.id, $input.data]" --read-only false -- "INSERT INTO unknown_queue (id, data) VALUES ($1, $2)" \
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
| List, compact | `--auth-required-role admin,lecturer` | `["admin","lecturer"]` array |
| List, repeated | `--from "posts.csv as posts" --from "authors.csv as authors"` | `["posts.csv as posts","authors.csv as authors"]` array |
| Bool | `--http-only` | `true` (no value consumed) |
| Key-value pairs | `--claim name=$.name --claim roles=$.roles` | `{"name":"$.name","roles":"$.roles"}` object |

Key-value pairs (`--claim`, `--header`, `--set-cookie`) repeat the **same flag with a different key** each time — each occurrence adds one entry to a map.
List flags repeat **values for the same key**. Every list flag accepts either compact comma form (`--cases create,update`) or repeated form (`--cases create --cases update`).
Use one style per flag per node command. Mixed style is invalid: `--cases create,update --cases delete`.
Nodes document their recommended style: compact for short enums, repeated for long values where commas may be valid text.

---

## Engine-Level Flags

These flags are available on **every** node kind. They control engine behavior,
not node-internal logic.

### `--timeout <seconds>`

Override the project-level `pipeline_node_timeout_secs` for this specific node.
Value in seconds, clamped to 5–3600. Absent = use project default (typically 30s).

```zf
register slow-report --path /reports \
  | trigger.webhook --path /generate --method POST \
  | pg.query --credential main-db --timeout 120 -- "SELECT * FROM big_report_view" \
  | n.ai.agent --credential openai --mode strategic --timeout 300 \
  | web.response --template pages/report.tsx
```

In this example, `pg.query` gets 120s and `n.ai.agent` gets 300s while
`web.response` uses the project default (30s).

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

## SSE Streaming (Webhook Pipelines)

Any webhook pipeline supports real-time progress streaming via Server-Sent Events.
No pipeline config changes needed — the client controls the mode with the `Accept` header.

**Normal request** (waits for full result):
```bash
curl -X POST /wh/owner/project/api/search -d '{"query":"hello"}'
```

**Streaming request** (receives step events as they happen):
```bash
curl -X POST /wh/owner/project/api/search \
  -H "Accept: text/event-stream" \
  -d '{"query":"hello"}'
```

**SSE events emitted:**
- `event: signal` — signals from nodes that emit via the ExecutionBus (e.g. `n.ai.agent` emits thinking/tool_call signals) or return `__signal` in their output
- `event: done` — final pipeline result: `{"ok":true,"value":{...}}`
- `event: error` — pipeline failure: `{"ok":false,"error":{"code":"...","message":"..."}}`

The stream closes after `done` or `error`. The pipeline definition, activation, and
DSL registration are identical for both modes.

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
activate blog-home
activate pipelines/api/blog-home.zf.json
deactivate blog-home
```

The `file_rel_path` is the only argument — no `pipeline` keyword between verb and path.
`pipelines/` prefix and `.zf.json` extension are added automatically when omitted.

### execute — trigger a registered pipeline

```zf
execute blog-home
execute process-order -- {"order_id": 42, "action": "resend"}
execute pipelines/api/process-order.zf.json -- {"order_id": 42}
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
  | sekejap.query --params-expr "[$input.id, $input.status]" --read-only false -- "INSERT INTO results (id, status) VALUES ($1, $2)"

# Graph mode
run \
  [a] http.request --url https://example.com --method GET \
  [b] logic.if --expr "input.status >= 400" \
  [c] sekejap.query --params-expr "[$input.id, $input.status]" --read-only false -- "INSERT INTO errors (id, status) VALUES ($1, $2)" \
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
n.logic.match --help            # same
```

### Data & compute nodes

| Short name | Full kind | Config flags |
|---|---|---|
| `trigger.webhook` | `n.trigger.webhook` | `--path <path> --method <GET\|POST\|...> [--auth-type jwt\|hmac\|api_key] [--auth-credential <id>] [--auth-required-role <roles>]` |
| `trigger.schedule` | `n.trigger.schedule` | `--cron <expr> --timezone <tz>` |
| `trigger.manual` | `n.trigger.manual` | _(none)_ |
| `script` | `n.script` | `--lang <js\|ts>` or `-- <code>` |
| `web.response` | `n.web.response` | `--template <pages/name>` (no `.tsx`), `--status`, `--location`, `--message`, `--body <$.path>`, `--set-cookie`, `--header <key=value>`, `--load-scripts <urls>` |
| `web.static.generate` | `n.web.static.generate` | `--template <pages/name.tsx> --output-path <path> [--route <url>] [--on-conflict overwrite\|skip\|error]` — render a TSX page once and write the generated HTML into Zebflow FS; output `{ generated: { status, path, url, route, template, bytes } }` |
| `http.request` | `n.http.request` | `--url <url> --method <GET\|POST> [--timeout-ms <ms>] [--header <key=value> ...] [--merge-input]` |
| `sekejap.query` | `n.sekejap.query` | `[--params-path <dot.path>] [--params-expr <js-expr>] [--query-expr <js-expr>] -- "SELECT ... WHERE id = $1"` — raw Sekejap SQL with `$1`/`$2` bind params; output `{ rows: [...] }` |
| `pg.query` | `n.pg.query` | `--credential <credential-slug>` (**credential slug** from `get credentials`, kind=postgres) `[--params-path <dot.path>] [--params-expr <js-expr>] [--credential-expr <js-expr>] [--query-expr <js-expr>]` + `-- <sql>` |
| `auth.token.create` | `n.auth.token.create` | `--credential <jwt_key_id> [--expires-in <secs>] [--claim key=$.field ...] [--issuer <iss>] [--audience <aud>]` — append `:public` to a claim value to expose it in the browser via `ctx.auth` (e.g. `--claim name=$.fullname:public`). Use `--claim roles=$.roles:public` where `roles` is an array — role-based access control always uses the `roles` array claim. Claims without `:public` are signed but never reach the browser DOM. Secure by default — `ctx.auth` is `null` unless at least one claim is marked public. |
| `table.convert` | `n.table.convert` | `(--from <path> \| --from-expr <expr>) [--from-format csv\|json\|ndjson\|parquet] [--to <path>] [--to-format csv\|json\|ndjson\|parquet] [--to-json] [--preview <n>] [--limit <n>]` — converts CSV/JSON/NDJSON/Parquet between ZebFS and downstream row JSON. |
| `table.query` | `n.table.query` | `--from "<path-or-expr> as <alias>" ... --sql "<select>" [--engine geodatafusion] [--params-path <dot.path>] [--params-expr <js-expr>] [--to <path>] [--format csv\|json\|ndjson\|parquet] [--to-json] [--preview <n>] [--limit <n>]` — runs GeoDataFusion SQL over CSV/JSON/NDJSON/Parquet ZebFS objects or upstream row expressions. |
| `fs.list` | `n.fs.list` | `[--path <prefix> \| --prefix <prefix>]` — list immediate children under a ZebFS prefix; output `{ fs: { operation, path, count, entries } }`. |
| `fs.head` | `n.fs.head` | `--path <object-or-prefix>` — read object/prefix metadata without reading content. |
| `fs.get` | `n.fs.get` | `--path <object> [--encoding text\|base64]` — read one object; default text requires UTF-8, base64 is for binary objects. |
| `fs.put` | `n.fs.put` | `--path <object> (--from-key <dot.path> \| --text <text> \| --base64 <base64>)` — write one object from FileRef, payload, text, or base64 bytes. |
| `fs.delete` | `n.fs.delete` | `--path <object-or-prefix>` — delete one object or prefix tree; absent paths are treated as already deleted. |
| `fs.copy` | `n.fs.copy` | `--from <object> --to <object>` — copy one object. |
| `fs.move` | `n.fs.move` | `--from <object> --to <object>` — move one object by copying it then deleting the source. |
| `fs.mkdir` | `n.fs.mkdir` | `--path <prefix>` — create a prefix directory. |
| `fs.save` | `n.fs.save` | `[--field <name>] [--path <object-path>] [--folder <subdir>] [--allowed-kinds <images,pdf,csv,json,glb,audio,video>] [--max-size <mb>] [--filename <name>]` — validates/promotes an uploaded FileRef or legacy upload object to Zebflow FS; output `{ saved: { path, url, original_name, content_type, size } }`. |
| `fs.thumbnail` | `n.fs.thumbnail` | `[--width <px>] [--height <px>] [--fit cover|contain|fill] [--format jpg|png|webp] [--quality <1-100>] [--folder <subdir>] [--source-key <dot.path>] [--delete-source] [--filename <name>]` — reads a FileRef or object path from `saved.path` by default, resizes/re-encodes it, writes thumbnail to Zebflow FS; replaces the payload with `{ thumbnail: FileRef + { width, height, format } }`. |
| `ai.zebtune` | `n.ai.zebtune` | `--budget <n> --output <mode>` |
| `ai.tts` | `n.ai.tts` | `--provider piper --credential <tts_credential_id> --text-expr <expr> [--output-path <path.wav> \| --output-path-expr <expr>] [--return file\|blob\|both] [--speaker <id>] [--speed <factor>] [--volume <factor>] [--lipsync none\|basic\|timed_words\|audio_guided] [--lipsync-expr <expr>]` — synthesize speech from text. First stable provider is local Piper. Credential secret must reference `model_file` and `config_file` under Zebflow FS; `espeak_data_dir` is an optional override. When lipsync is enabled the output also includes `word_timings` and `lipsync { metadata, cues }`. |
| `trigger.ws` | `n.trigger.ws` | `--event <name> --room <id>` |
| `trigger.kv.subscribe` | `n.trigger.kv.subscribe` | `--channel <name>` — subscribes to a project KV pub/sub channel; fires whenever `kv.publish` sends to that channel |
| `ws.emit` | `n.ws.emit` | `--event <name> --to <all\|session\|others> --payload-path <ptr> [--room <id>]` — `--room` static or `{{ expr }}`; when `--room` is set this node works after **any** trigger type, not just `trigger.ws` |
| `ws.sync_state` | `n.ws.sync_state` | `--op <set\|merge\|delete> --path <ptr> --value-path <ptr> --room <id>` |
| `kv.set` | `n.kv.set` | `--key <k> --value-path <ptr> [--ttl <secs>] [--durable]` — write value from payload path into per-project KV; optional TTL in seconds |
| `kv.get` | `n.kv.get` | `--key <k> [--out-key <k>] [--default <json>] [--durable]` — read key from KV store; replaces the payload with `{ [out_key]: value }`; `--default` used when key is missing/expired |
| `kv.exists` | `n.kv.exists` | `--key <k> [--out-key <k>] [--durable]` — replaces the payload with `{ [out_key]: boolean }` (default key `exists`); does not consume the value |
| `kv.del` | `n.kv.del` | `--key <k> [--durable]` — delete key from KV store; payload passes through unchanged |
| `kv.expire` | `n.kv.expire` | `--key <k> [--ttl <secs>] [--durable]` — update TTL on an existing key without changing its value; `--ttl 0` removes expiry (persist forever) |
| `kv.incr` | `n.kv.incr` | `--key <k> [--amount <n>] [--out-key <k>] [--durable]` — atomically increment (negative to decrement) integer counter; starts at 0 if missing; replaces the payload with `{ [out_key]: new_value }` |
| `kv.publish` | `n.kv.publish` | `--channel <name> [--message-path <ptr>]` — publish a message to a project KV pub/sub channel; triggers all active `n.trigger.kv.subscribe` pipelines on that channel |
| `ms.publish` | `n.ms.publish` | `--name <id> --path <http_path> --source-path <zebfs_path> [--source-kind geojson_file\|geojson_artifact\|geoparquet] [--bbox-required] [--max-features <n>] [--allowed-properties <csv>] [--min-zoom <n>] [--max-zoom <n>] [--build-artifact]` — publish or update a map layer in the project registry |
| `ms.unpublish` | `n.ms.unpublish` | `--name <id>` — remove a map layer from the registry |
| `ms.get` | `n.ms.get` | `--name <id>` — get metadata for a published layer |
| `ms.list` | `n.ms.list` | _(none)_ — list all published layers in the project registry |

### `sekejap.query` — SQL examples

```zf
# SELECT — basic
| sekejap.query -- "SELECT id, title FROM posts LIMIT 20"

# SELECT — with one bound value from a dot path
| sekejap.query --params-path auth.sub -- "SELECT * FROM orders WHERE user_id = $1 LIMIT 10"

# SELECT — with multiple bound values from JS
| sekejap.query --params-expr "[$trigger.params.id, $trigger.query.status]" -- "SELECT * FROM orders WHERE user_id = $1 AND status = $2 LIMIT 10"

# SELECT — graph traversal
| sekejap.query --params-path params.id -- "SELECT id FROM cases TRAVERSE FORWARD caused_by TO causes HOPS 3 WHERE id = $1"

# INSERT
| sekejap.query --params-expr "[$input.id, $input.title]" --read-only false -- "INSERT INTO tasks (id, title, done) VALUES ($1, $2, false)"

# UPDATE
| sekejap.query --params-path params.id --read-only false -- "UPDATE tasks SET done = true WHERE id = $1"

# DELETE
| sekejap.query --params-path params.id --read-only false -- "DELETE FROM tasks WHERE id = $1"

# CREATE COLLECTION (schema definition)
| sekejap.query --read-only false -- "CREATE COLLECTION tasks (id STRING INDEX hash, title STRING, done BOOLEAN)"
```

### `n.table.convert` — table conversion

Reads table-like data either from a ZebFS object path or from an upstream expression. Writes back to ZebFS with `--to`, emits downstream rows with `--to-json`, or both.

```zf
# CSV object to JSON rows for the next node
| table.convert --from uploads/posts.csv --to-json --preview 20

# Upstream query rows to NDJSON object
| sekejap.query -- "SELECT id, title FROM posts"
| table.convert --from-expr "$input.rows" --to exports/posts.ndjson

# JSON object to CSV object
| table.convert --from data/posts.json --to exports/posts.csv

# Upstream rows to Parquet object
| table.convert --from-expr "$input.rows" --to exports/posts.parquet
```

### `n.table.query` — multi-source table SQL

Runs a read-only GeoDataFusion SQL query over one or more table sources.
Each source is bound with `--from "<source> as <alias>"`. Source can be a ZebFS object path
or a row-producing expression such as `$input.rows`.

```zf
# Join two ZebFS objects and emit rows downstream
| table.query \
    --from "datasets/posts.parquet as posts" \
    --from "datasets/authors.csv as authors" \
    --to-json \
    --sql "select p.id, p.title, a.name from posts p join authors a on p.author_id = a.id"

# Bind parameters with GeoDataFusion placeholders
| table.query \
    --from "datasets/posts.csv as posts" \
    --params-expr "[$trigger.params.id]" \
    --to-json \
    --sql "select * from posts where id = $1"

# Query upstream rows and write Parquet
| sekejap.query -- "SELECT id, title, author_id FROM posts"
| table.query \
    --from "$input.rows as posts" \
    --to exports/filtered-posts.parquet \
    --sql "select * from posts where author_id is not null"
```

### FileRef convention

File-like bytes should travel through pipelines as FileRef metadata, not inline base64. Current
FileRefs use `backend: "zebfs"` and may be `lifecycle: "temporary"` for ingress/intermediate files
or `lifecycle: "durable"` for promoted/generated artifacts.

```json
{
  "__zf_type": "file_ref",
  "backend": "zebfs",
  "ref": "tmp/runs/webhook-123/files/input.geojson",
  "lifecycle": "temporary",
  "filename": "input.geojson",
  "mime": "application/geo+json",
  "size": 123456,
  "sha256": "sha256:..."
}
```

The `backend` field is reserved for future storage implementations such as S3/R2/MinIO. Today only
`zebfs` is implemented.

### `n.fs.save` — saving uploaded files

Reads a file from `input.files.{field}` (set by `trigger.webhook` multipart parsing), validates it,
and saves/promotes it to Zebflow FS at `uploads/{uuid}.{ext}` by default. New webhook uploads are
already FileRef values in tmp storage; `fs.save` is now the validation/promotion step, not the only
way to get upload bytes into Zebflow FS.

The saved file is immediately accessible at the URL returned in `saved.url`.

**Flags:**

| Flag | Default | Description |
|---|---|---|
| `--field` | `file` | Multipart field name from the upload form |
| `--path` | _(empty)_ | Exact ZebFS object path. If omitted, folder + generated filename is used. |
| `--folder` | `uploads` | ZebFS object folder used when `--path` is omitted |
| `--allowed-kinds` | `images` | Comma-separated categories: `images,pdf,csv,json,glb,audio,video` |
| `--max-size` | `10` | Maximum file size in MB |
| `--filename` | _(UUID)_ | Custom filename without extension. If set, overwrites existing file with same name. Sanitized to alphanumeric, dash, underscore only. |

**Output:** `{ saved: { path, url, original_name, content_type, size } }`

- `path` — ZebFS object path (e.g. `uploads/a1b2c3.jpg`)
- `url` — ZebFS object URL (e.g. `/fs/{owner}/{project}/uploads/a1b2c3.jpg`)
- `original_name` — the original filename from the upload
- `content_type` — MIME type reported by the browser
- `size` — actual decoded file size in bytes

**Examples:**

```zf
# Accept any file, save to uploads/
register upload-file --path /api \
  | trigger.webhook --path /upload --method POST --auth-type jwt --auth-credential my-jwt \
  | fs.save \
  | web.response

# Accept images only, save to avatars/
register upload-avatar --path /api \
  | trigger.webhook --path /avatar --method POST --auth-type jwt --auth-credential my-jwt \
  | fs.save --field avatar --folder avatars --allowed-kinds images --max-size 5 \
  | sekejap.query --params-expr "[$input.saved.url, $trigger.auth.sub]" --read-only false -- "UPDATE users SET avatar_url = $1 WHERE id = $2" \
  | web.response

# Save PDF, store reference in DB
register upload-document --path /api \
  | trigger.webhook --path /documents --method POST --auth-type jwt --auth-credential my-jwt \
  | fs.save --field document --folder documents --allowed-kinds pdf --max-size 20 \
  | sekejap.query --params-expr "[$input.saved.path, $input.saved.url, $input.saved.original_name]" --read-only false -- "INSERT INTO documents (id, url, name) VALUES ($1, $2, $3)" \
  | web.response

# Deterministic filename — always saves as avatars/profile-photo.jpg (overwrites on re-upload)
register upload-avatar-fixed --path /api \
  | trigger.webhook --path /avatar --method POST --auth-type jwt --auth-credential my-jwt \
  | fs.save --field avatar --folder avatars --filename profile-photo --allowed-kinds images \
  | web.response
```

**Security notes:**
- By default, files are stored using a UUID filename (path traversal safe). Use `--filename` for deterministic names — the value is sanitized to safe characters only.
- Files live outside the git-synced `repo/` folder — they are not committed with your codebase.
- Always use `--auth-type jwt` on the webhook trigger for authenticated uploads.
- Use `--allowed-kinds` to restrict what users can upload.

### `n.web.static.generate` — persistent static page generation

Renders a TSX template through the same RWE engine used by `web.response`, then writes the
final HTML file into Zebflow FS.

This is the right primitive for:
- generated lyric pages
- cached export pages
- regeneration pipelines after content changes
- future object-storage publishing flows

The first release writes a self-contained HTML file:
- project `styles/main.css` is inlined
- Tailwind CSS extracted by RWE is inlined
- client hydration scripts are inlined

**Flags:**

| Flag | Default | Description |
|---|---|---|
| `--template` | _(required)_ | TSX page under `repo/pipelines`, e.g. `pages/lyrics.tsx` |
| `--output-path` | _(required)_ | Zebflow FS object path. Supports `{{ expr }}` interpolation. |
| `--route` | generated `/fs/...` URL | Optional `ctx.route` override seen by the template during render |
| `--on-conflict` | `overwrite` | `overwrite`, `skip`, or `error` when the destination exists and content differs |

**Examples:**

```zf
# Generate one lyric page into Zebflow FS
register generate-lyric --path /ops \
  | trigger.manual \
  | script -- "return { artist_slug: 'iwan-fals', song_slug: 'bento', artist_name: 'Iwan Fals', song_title: 'Bento' }" \
  | web.static.generate \
      --template pages/lyric.tsx \
      --output-path "artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html"

# Stop if the generated file already exists
register generate-preview --path /ops \
  | trigger.manual \
  | script -- "return { slug: 'draft-song' }" \
  | web.static.generate \
      --template pages/lyric-preview.tsx \
      --output-path "previews/{{ $input.slug }}.html" \
      --on-conflict error

# Override ctx.route so the template renders canonical links differently from its saved file path
register generate-canonical-page --path /ops \
  | trigger.manual \
  | script -- "return { artist_slug: 'iwan-fals', song_slug: 'bento' }" \
  | web.static.generate \
      --template pages/lyric.tsx \
      --route "/lyrics/{{ $input.artist_slug }}/{{ $input.song_slug }}" \
      --output-path "artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html"
```

### `n.fs.thumbnail` — image resizing and compression

Reads an image file from disk (FileRef or path resolved from the payload, default `saved.path`),
resizes and re-encodes it, then writes the result to the project's file storage. Decompression bomb
protection is built in — images over 16000×16000 or 128 MB decoded are rejected.

**Flags:**

| Flag | Default | Description |
|---|---|---|
| `--width` | `256` | Output width in pixels |
| `--height` | `256` | Output height in pixels |
| `--fit` | `cover` | `cover` — scale+crop center; `contain` — fit within box; `fill` — stretch exact |
| `--format` | `jpg` | Output format: `jpg`, `png`, or `webp` |
| `--quality` | `82` | JPEG/WebP quality (1–100). Ignored for PNG. |
| `--folder` | `thumbnails` | Zebflow FS object folder |
| `--source-key` | `saved.path` | Dot-notation path into payload for the source FileRef or relative path |
| `--delete-source` | _(off)_ | Delete the original source file after successful thumbnail write |
| `--filename` | _(UUID)_ | Custom filename without extension. If set, overwrites existing thumbnail with same name. Sanitized to alphanumeric, dash, underscore only. |

**Output:** replaces the payload with `{ thumbnail: FileRef + { path, url, width, height, format, size } }`.

**Examples:**

```zf
# Upload FileRef → thumbnail (avatar use case: re-encode strips any embedded payload)
register upload-avatar -- \
  | trigger.webhook --path /upload/avatar --method POST \
  | fs.thumbnail --source-key files.photo --width 200 --height 200 --fit cover --format jpg --quality 80 \
                  --folder avatars --delete-source \
  | sekejap.query --params-expr "[$input.thumbnail.url, $trigger.auth.sub]" --read-only false -- "UPDATE users SET avatar_url = $1 WHERE id = $2" \
  | web.response

# Thumbnail existing file in files/
register make-thumb -- \
  | trigger.webhook --path /admin/thumb --method POST \
  | fs.thumbnail --width 400 --height 300 --fit contain --format webp --quality 90 \
                  --source-key file_path --folder web-thumbs \
  | web.response
```

**Security notes:**
- Re-encoding strips all EXIF metadata and any embedded executable payloads — treat all uploads as untrusted.
- Use `--delete-source` to replace the original with the sanitized thumbnail.
- Source SVG, HEIC, and HEIF formats are rejected (use a dedicated pipeline for conversion).

### `n.kv.*` — project key/value store and pub/sub

Per-project key/value store. Ephemeral by default; use `--durable` on supported KV nodes when the value must survive server restarts. Scoped to `owner/project`.

**Use cases**: rate limiting, session TTL refresh, counters, pub/sub triggers, cache-aside pattern.

All `--key` and `--channel` flags support `{{ expr }}` template expressions.

#### Key/value operations

```zf
# Write a value from payload into the store
| kv.set --key "session:{{ $trigger.auth.sub }}" --value-path /session_data --ttl 3600

# Read a value back (replaces payload with { cached: <value> })
| kv.get --key "cache:{{ $trigger.params.slug }}" --out-key cached --default null

# Check if key exists without consuming it
| kv.exists --key "lock:{{ $input.task_id }}" --out-key is_locked

# Delete a key
| kv.del --key "session:{{ $trigger.auth.sub }}"

# Refresh TTL without changing value (extend session on activity)
| kv.expire --key "session:{{ $trigger.auth.sub }}" --ttl 3600

# Remove TTL — make key permanent
| kv.expire --key "session:{{ $trigger.auth.sub }}" --ttl 0

# Atomic counter (starts at 0 if missing)
| kv.incr --key "clicks:{{ $trigger.params.button }}" --out-key total

# Decrement
| kv.incr --key "slots:{{ $input.event_id }}" --amount -1 --out-key remaining
```

#### Pub/sub

```zf
# Publisher pipeline (triggered by webhook, schedule, etc.)
| trigger.webhook --path /api/events --method POST
| kv.publish --channel "events:{{ $input.type }}" --message-path /

# Subscriber pipeline (triggered by publisher)
| trigger.kv.subscribe --channel "events:order.created"
| script -- "return { event: input.message, received_at: Date.now() }"
| sekejap.query --params-expr "[$input.event.id, $input.event]" --read-only false -- "INSERT INTO processed_events (id, data) VALUES ($1, $2)"
```

Output payload of `n.trigger.kv.subscribe`:
```json
{
  "trigger": "kv.subscribe",
  "channel": "events:order.created",
  "node_id": "n0",
  "message": { /* original published payload */ }
}
```

#### `n.ws.emit` from non-WS triggers

`n.ws.emit` works after **any** trigger type when `--room` is specified:

```zf
# Push update to a WS room from a webhook
| trigger.webhook --path /api/board/:room_id --method POST
| sekejap.query --params-path params.room_id --read-only false -- "UPDATE boards SET updated_at = NOW() WHERE id = $1"
| ws.emit --event board.updated --to all --room "{{ $trigger.params.room_id }}" --payload-path /
```

Without `--room`, `ws.emit` reads `room_id` from the payload (set by `trigger.ws`).
With `--room`, it is fully self-contained and works from webhook, schedule, or any trigger.

---

### `n.ms.*` — mapserver layer CRUD

Dynamic map layer management for the project layer registry. Layers published via these nodes are immediately queryable on `/ms/{owner}/{project}/{path}`.

```zf
# Publish a GeoJSON layer
| ms.publish --name districts --path /districts --source-path mapserver/districts.geojson --source-kind geojson_file

# Publish with auto-artifact build for large GeoJSON files
| ms.publish --name buildings --path /buildings --source-path mapserver/buildings.geojson --build-artifact

# Publish a GeoParquet layer
| ms.publish --name parcels --path /parcels --source-path mapserver/parcels.parquet --source-kind geoparquet --max-features 5000

# Get layer metadata
| ms.get --name districts

# List all published layers
| ms.list

# Unpublish a layer
| ms.unpublish --name districts
```

**`n.ms.publish` flags:**
- `--name <id>` — unique layer identifier (required)
- `--path <path>` — URL path under `/ms/{owner}/{project}/` (required)
- `--source-path <zebfs_path>` — ZebFS path to source file (required; also accepted from input payload.source_path)
- `--source-kind <kind>` — `geojson_file` (default), `geojson_artifact`, `geoparquet`
- `--bbox-required` — enforce bbox in queries (default: true)
- `--max-features <n>` — hard feature cap per query (default: 1000)
- `--allowed-properties <csv>` — property whitelist (empty = all)
- `--min-zoom <n>` / `--max-zoom <n>` — zoom visibility range
- `--build-artifact` — auto-build chunked artifact for large GeoJSON files

**Supported source kinds:**
- `geojson_file` — reads a GeoJSON FeatureCollection from ZebFS
- `geojson_artifact` — chunked spatial index built from a GeoJSON source (fast bbox queries)
- `geoparquet` — queries a GeoParquet file via DataFusion with ST_* spatial functions

---

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
| `logic.match` | `n.logic.match` | named cases + default | `--expr <js-expression> --cases a,b,c --default <name>` or repeated `--cases a --cases b` |
| `logic.collect` | `n.logic.collect` | `out` | none |
| `logic.foreach` | `n.logic.foreach` | `item` | `--items-expr <js-expression> [--dispatch seq] [--chunk-size N] [--keep-input]` |
| `logic.reduce` | `n.logic.reduce` | `out` | `--init-expr <expr> --step-expr <expr>` |
| `logic.retry` | `n.logic.retry` | `retry`, `failed` | `--max-attempts <n> [--delay-ms <ms>]` |

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
get fs [--path uploads]
read fs uploads/logo.png
write fs uploads/data.json -- '{"key":"value"}'
delete fs old-export.csv
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
  [c] logic.if --expr "$input.status >= 400" \
  [d] http.request --url https://hooks.slack.com/xxx --method POST \
  [a] -> [b] \
  [b] -> [c] \
  [c]:true -> [d]
```

`[c]:false` has no edge → execution stops silently (no error). That's your "do nothing" branch.

### logic.match — multi-case routing

```zf
register event-router --path /webhooks \
  [a] trigger.webhook --path /events --method POST \
  [b] logic.match --expr "$input.type" --cases create --cases update --cases delete --default unknown \
  [c] script --lang js -- "return handleCreate(input);" \
  [d] script --lang js -- "return handleUpdate(input);" \
  [e] script --lang js -- "return handleDelete(input);" \
  [f] sekejap.query --params-expr "[$input.id, $input.type]" --read-only false -- "INSERT INTO unknown_events (id, type) VALUES ($1, $2)" \
  [a] -> [b] \
  [b]:create  -> [c] \
  [b]:update  -> [d] \
  [b]:delete  -> [e] \
  [b]:unknown -> [f]
```

Short form is also valid:

```zf
[b] logic.match --expr "$input.type" --cases create,update,delete --default unknown
```

### Native 1-M fan-out

```zf
register notify-all --path /jobs \
  [a] trigger.schedule --cron "0 9 * * *" --timezone UTC \
  [b] pg.query --credential main-db -- "SELECT * FROM alerts WHERE active = true" \
  [d] http.request --url https://email.api/send --method POST \
  [e] http.request --url https://sms.api/send --method POST \
  [f] http.request --url https://hooks.slack.com/xxx --method POST \
  [a] -> [b] \
  [b] -> [d] \
  [b] -> [e] \
  [b] -> [f]
```

### logic.collect — explicit together-processing

```zf
register parallel-fetch --path /jobs \
  [a] trigger.manual \
  [b] http.request --url https://source-a.com --method GET \
  [c] http.request --url https://source-b.com --method GET \
  [d] logic.collect \
  [e] script --lang js -- "return combine(input.b, input.c);" \
  [a] -> [b] \
  [a] -> [c] \
  [b] -> [d] \
  [c] -> [d] \
  [d] -> [e]
```

### logic.foreach — ordered multi-emission

```zf
register emit-rows --path /jobs \
  [a] trigger.manual \
  [b] logic.foreach --items-expr "$input.rows" --dispatch seq \
  [c] script --lang js -- "return { id: input.item.id, index: input.index };" \
  [a] -> [b] \
  [b]:item -> [c]
```

`logic.foreach` emits item-only payloads by default:

```json
{ "item": { "...": "..." }, "index": 0, "count": 10 }
```

Use `--keep-input` only when every emitted item must also carry the full upstream
payload. The default avoids fan-out amplification for large tables, files, and
numeric arrays.

### logic.reduce — ordered accumulation

```zf
register sum-rows --path /jobs \
  [a] trigger.manual \
  [b] logic.foreach --items-expr "$input.rows" \
  [c] logic.reduce --init-expr "{ total: 0 }" --step-expr "{ total: $acc.total + $input.item.amount }" \
  [a] -> [b] \
  [b]:item -> [c]
```

### logic.retry — bounded retry on `:error`

```zf
register retry-request --path /jobs \
  [a] trigger.manual \
  [b] http.request --url https://api.example.com/work --method POST \
  [r] logic.retry --max-attempts 3 --delay-ms 250 \
  [c] script --lang js -- "return input;" \
  [d] script --lang js -- "return input;" \
  [a] -> [b] \
  [b]:error -> [r] \
  [r]:retry -> [b] \
  [b] -> [c] \
  [r]:failed -> [d]
```

`logic.retry` handles the immediately failing upstream node input. The engine only routes to
`[node]:error` when that edge exists; otherwise the pipeline fails normally.

### Loops (back-edges)

```zf
register retry-job --path /jobs \
  [a] trigger.manual \
  [b] script --lang js \
       -- "const n=(input.attempts||0)+1; return {...doWork(input), attempts:n};" \
  [c] logic.match --expr "$input.status" --cases done,failed --default retry \
  [d] script --lang js -- "return { result: input };" \
  [e] sekejap.query --params-expr "[$input.id, $input.attempts]" --read-only false -- "INSERT INTO failures (id, attempts) VALUES ($1, $2)" \
  [f] logic.if --expr "$input.attempts < 5" \
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
activate get-posts
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
    { "id": "b", "kind": "n.logic.match",      "input_pins": ["in"], "output_pins": ["create","update","unknown"],  "config": { "expression": "input.type", "cases": ["create","update"], "default": "unknown" } },
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

describe pipeline blog-home && activate blog-home
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
