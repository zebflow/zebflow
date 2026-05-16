# Zebflow Pipeline DSL — Full Reference

This is the complete DSL reference. For the mental model and quick patterns, read the main SKILL.md first.

---

## Syntax

```
<command> [<resource>] [<name>] [--flag value] [-- <body>]
```

- **Multiline**: end a line with `\` to continue on the next line.
- **Chaining**: `&&` runs the next command only if the previous succeeded.
- **Body**: `-- <content>` passes inline content (script source, SQL, JSON input, etc.).

### Flag value kinds

| Kind | Example | Produces |
|------|---------|---------|
| Scalar | `--template pages/foo.tsx` | `"pages/foo.tsx"` string |
| Comma list | `--auth-required-role admin,lecturer` | `["admin","lecturer"]` array |
| Bool | `--http-only` | `true` (no value consumed) |
| Key-value pairs | `--claim name=$.name --claim roles=$.roles` | `{"name":"$.name","roles":"$.roles"}` object |

Key-value pairs (`--claim`, `--header`, `--set-cookie`) repeat the same flag with a different key each time.
Comma lists pass values comma-separated in one flag — `--role admin,lecturer`, not `--role admin --role lecturer`.

---

## Pipeline Lifecycle

| Status | Meaning |
|---|---|
| `draft` | Registered but never activated |
| `active` | Live and serving traffic |
| `stale` | Live but source has changed since last activation |
| `inactive` | Explicitly deactivated; source retained |

---

## Commands

### Pipeline commands

```zf
# List
get pipelines
get pipelines --path /webhooks --status active

# Inspect
describe pipeline blog-home

# Create/update (pipe mode)
register <name> --path <vpath> | node --flag val | node ...

# Create/update (graph mode)
register <name> --path <vpath> \
  [id] node --flag val \
  [id] -> [id]

# Targeted update
patch pipeline <name> node <id> --flag value
patch pipeline <name> node <id> -- "new body"
patch pipeline <name> node trigger.webhook --auth-type jwt

# Lifecycle
activate <name>
deactivate <name>
execute <name> -- {"key": "value"}
delete pipeline <name>

# Ephemeral (not persisted)
run | pg.query --credential main-db -- "SELECT 1"
run --dry-run | node ...
```

`patch` accepts node ID (`b`), kind (`trigger.webhook`), or kind+index (`pg.query[1]`).

### Connection commands

```zf
get connections
describe connection main-db --scope tables --schema public
delete connection old-db
```

Connection create/update is UI-only.

### Credential commands

```zf
get credentials    # names + kinds only — values never exposed
```

Credential create/update/delete is blocked in DSL — use the UI.

### Template commands

```zf
get templates
get templates --path /components
describe template components/ui/button.tsx
delete template components/ui/old-button.tsx
```

### File commands

```zf
get files [--scope public|private]
read file public/logo.png
write file public/data.json -- '{"key":"value"}'
delete file private/old-export.csv
```

### Doc commands

```zf
get docs
read doc README.md
write doc AGENTS.md -- "# Agents\n..."
```

### Git commands

```zf
git status
git log [--limit 10]
git diff [<path>]
git add <path>
git commit -- "message"
```

Blocked: `reset`, `rebase`, `force`, `checkout .`, `clean`, `branch -D`.

---

## Dynamic Config Expressions — `{{ expr }}`

Any string field can contain `{{ js_expr }}` placeholders, resolved before the node runs in a sandboxed Deno context.

### Scope variables

| Variable | Contents |
|---|---|
| `$input` | Current payload flowing into this node |
| `$trigger.auth` | Full JWT claims from the original request |
| `$trigger.params` | URL path params (`:id`, `:slug`) |
| `$trigger.query` | Query string params (`?page=2`) |
| `$trigger.headers` | Safe subset of request headers |
| `$nodes.id` | Output of a completed upstream node by graph ID |

### Type preservation

- Whole-field `{{ expr }}` → native JS type (object, array, number)
- Interpolated `"Hello {{ name }}!"` → string

### Security

Expressions run with `capabilities: []` — no DB access, no HTTP, no side effects. Op budget: 500 max.

---

## Full Node Catalog

### Triggers

| Node | Flags |
|---|---|
| `trigger.webhook` | `--path --method [--auth-type jwt\|hmac\|api_key] [--auth-credential] [--auth-required-role]` |
| `trigger.schedule` | `--cron --timezone` |
| `trigger.manual` | _(none)_ |
| `trigger.ws` | `--event --room` |
| `trigger.memsubscribe` | `--channel` |

### Data & compute

| Node | Flags |
|---|---|
| `script` | `[--lang js\|ts] -- "code"` |
| `pg.query` | `--credential [--params-path] [--params-expr] -- "SQL"` |
| `sekejap.query` | `-- "SELECT ..."` |
| `sekejap.mutate` | `-- "INSERT/UPDATE/DELETE ..."` |
| `http.request` | `--url --method [--timeout-ms] [--header key=val] [--merge-input]` |
| `web.response` | `--template [--status] [--header key=val] [--set-cookie] [--load-scripts]` |
| `web.static.generate` | `--template --output-path [--scope public\|private] [--route] [--on-conflict]` |
| `file.save` | `[--field] [--dest] [--allowed-types] [--max-size] [--filename]` |
| `img.thumbnail` | `[--width] [--height] [--fit] [--format] [--quality] [--folder] [--delete-source]` |
| `auth.token.create` | `--credential [--expires-in] [--claim key=$.field] [--issuer] [--audience]` |
| `ai.zebtune` | `--budget --output` |
| `ai.tts` | `--provider piper --credential --text-expr [--output-path] [--speaker] [--lipsync]` |

### Logic / control flow (graph mode)

| Node | Output pins | Flags |
|---|---|---|
| `logic.if` | `true`, `false` | `--expr "js"` |
| `logic.match` | named cases + default | `--expr "js" --cases a,b --default x` |
| `logic.collect` | `out` | _(none)_ |
| `logic.foreach` | `item` | `--items-path [--dispatch seq\|parallel] [--chunk-size]` |
| `logic.reduce` | `out` | `--init-expr --step-expr` |
| `logic.retry` | `retry`, `failed` | `--max-attempts [--delay-ms]` |

### In-memory KV + pub/sub

| Node | Flags |
|---|---|
| `mem.set` | `--key --value-path [--ttl]` |
| `mem.get` | `--key [--out-key] [--default]` |
| `mem.exists` | `--key [--out-key]` |
| `mem.del` | `--key` |
| `mem.expire` | `--key [--ttl]` |
| `mem.incr` | `--key [--amount] [--out-key]` |
| `mem.publish` | `--channel [--message-path]` |

### WebSocket

| Node | Flags |
|---|---|
| `ws.emit` | `--event --to all\|session\|others --payload-path [--room]` |
| `ws.sync_state` | `--op set\|merge\|delete --path --value-path --room` |

---

## Graph Mode: Control Flow

### logic.if

```zf
[b] logic.if --expr "$input.status >= 400"
[b]:true  -> [error_handler]
[b]:false -> [success_handler]
```

### logic.match

```zf
[b] logic.match --expr "$input.type" --cases create,update,delete --default unknown
[b]:create  -> [c]
[b]:update  -> [d]
[b]:delete  -> [e]
[b]:unknown -> [f]
```

### logic.collect (fan-in)

```zf
[a] -> [b]    # fan-out
[a] -> [c]
[b] -> [d]    # collect waits for all
[c] -> [d]
[d] logic.collect
[d] -> [e]    # e runs once with grouped input
```

### logic.foreach

```zf
[b] logic.foreach --items-path /rows --dispatch seq
[b]:item -> [c]    # c runs per item
```

Inside downstream: `$item` (current), `$index`, `$count`.

### logic.reduce

```zf
[c] logic.reduce --init-expr "{ total: 0 }" --step-expr "{ total: $acc.total + $input.amount }"
```

### logic.retry

```zf
[b]:error  -> [r]     # route errors to retry
[r] logic.retry --max-attempts 3 --delay-ms 250
[r]:retry  -> [b]     # back-edge
[r]:failed -> [d]     # give up
```

### Loops (back-edges)

```zf
[f]:true -> [b]    # back-edge = loop
```

---

## Webhook Request Payload

| Source | Where in payload |
|---|---|
| JSON body | Fields merged to root |
| Form body | Fields merged to root |
| Multipart text fields | Merged to root |
| Multipart files | `input.files.{field}` |
| Query params | Root + `input.query` |
| URL path params | `input.params` |
| JWT claims | `input.auth` |

---

## JSON IR

Both modes compile to `PipelineGraph` JSON:

```json
{
  "kind": "zebflow.pipeline",
  "version": "0.1",
  "nodes": [
    { "id": "a", "kind": "n.trigger.webhook", "config": { "path": "/events" } }
  ],
  "edges": [
    { "from_node": "a", "from_pin": "out", "to_node": "b", "to_pin": "in" }
  ]
}
```

No conditions on edges. Routing is via named output pins on logic nodes.
