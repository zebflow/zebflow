# Pipeline DSL

The DSL is the text form of a pipeline: one line per node, `|` between them.
It reaches the platform three ways, all equivalent:

| Channel | Form |
|---|---|
| MCP | `pipeline_register file_rel_path="api/posts" body="| trigger.webhook … | …"`, `pipeline_run body="…"` (unsaved) |
| Project console | `register api/posts --title "Posts" | trigger.webhook … | …`, then `activate pipeline api/posts` |
| HTTP | `POST /api/projects/{o}/{p}/pipelines/dsl` with `{"dsl": "register …"}` (write the JSON to a file and `-d @file` — the flags do not survive shell quoting) |

The parsed result is the JSON document described in `help("pipeline/authoring")`.

---

## Two modes

**Pipe mode** — a linear chain. The first node is the entry; each node
receives the previous node's output as `input`. Node ids are `n0, n1, …`.

```
| trigger.webhook --path /posts/:slug --method GET
| sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT * FROM posts WHERE slug = $1"
| web.response --template pages/post.tsx
```

**Graph mode** — label each node `[id]`, then wire edges. Needed for
branching, fan-out, fan-in and loops.

```
[a] trigger.webhook --path /status --method GET
[b] http.request --url https://example.com/health --method GET
[c] logic.if --expr "input.response.status >= 400"
[d] http.request --url https://hooks.example.com/alert --method POST --body "{{ input }}"
[e] web.response --body "{{ { ok: true } }}"
[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [e]
[d] -> [e]
```

Edge syntax: `[from] -> [to]` (pin `out` to pin `in`), `[from]:pin -> [to]`,
`[from]:pin -> [to]:pin`. A pin with no edge ends that branch silently. Every
node needs a path from the single entry — a node with no incoming edge is a
second entry and runs on every trigger. Any node's failure can be caught from
the pin `:error`.

---

## Syntax

```
<verb> [<resource>] [<name>] [--flag value]… [-- <body>]
```

- **Flags** belong to the node that declares them; an undeclared flag is a
  parse error (`unknown flag --x`). `help(topic="pipeline/nodes/<kind>")` lists them.
- **Body** — `-- "…"` — is the node's main text: SQL for query nodes, code for
  `script`, JSON for `execute --input`.
- **Quoting** — a value containing a space or `{{ }}` is one double-quoted
  argument. `--location {{ input.url }}` unquoted is cut at the first space
  and refused with a message that says so.
- **Multiline** — end a line with `\` to continue; in a console, `&&` chains
  commands and stops at the first failure.
- **Kinds** — `sekejap.query` and `n.sekejap.query` are the same node. Installed
  third-party nodes are `n.x.<bundle>.<node>`.

Flag value kinds, as each node declares them:

| Kind | Example | Config value |
|---|---|---|
| scalar | `--template pages/post.tsx` | `"pages/post.tsx"` |
| bool | `--durable` | `true` — no value consumed |
| comma-list | `--auth-required-role admin,editor` or `--cases a --cases b` | `["admin","editor"]` — one style per flag |
| key-value-pairs | `--claim "sub={{ input.id }}" --claim "name={{ input.name }}:public"` | `{ sub: …, name: … }` — repeat the flag, one key each |

Two flags exist on every node: `--timeout <seconds>` (engine timeout for this
node, clamped 5–3600, default the project's `pipeline_node_timeout_secs`) and
`--title "…"` (the label shown in the editor).

```
| pg.query --credential pg_main --timeout 120 -- "SELECT * FROM big_report_view"
```

---

## `{{ expr }}` — dynamic config

Any flag value may hold `{{ js_expression }}`, resolved right before the node
runs in a sandbox with no I/O (no `fetch`, no timers, no `n.*`, a bounded op
budget).

| Name | Meaning |
|---|---|
| `input`, `$input` | the payload flowing into this node |
| `$trigger` | the trigger snapshot: `params`, `query`, `search`, `pathname`, `headers`, `auth` — never `body` |
| `$nodes.<id>` | the output of an upstream node by its id (in pipe mode `n0`, `n1`, …). The trigger's own output — including `body` — is `$nodes.<trigger id>` |
| `$item`, `$index`, `$count` | inside a `logic.foreach` branch |

A value that is **only** an expression keeps its JSON type; an expression
inside a longer string is stringified. There is no `ctx`, `$ctx` or `env` in
`{{ }}` (a script has `ctx.trigger.*` and `ctx.request_id`). An undefined
name throws and fails the node instead of silently writing `null`.

```
| sekejap.query --params "{{ [$trigger.params.id] }}" -- "SELECT * FROM users WHERE id = $1"
| http.request --url "https://api.example.com/{{ $nodes.n1.rows[0].slug }}"
| http.request --url https://notify.example.com/send --method POST --body "{{ { userId: $trigger.auth.sub, data: input } }}"
| web.response --location "{{ $trigger.query.next || '/dashboard' }}"
```

(`http.request` answers `{ request, response: { status, headers, body } }`.)

---

## Commands

| Command | Effect |
|---|---|
| `register <path> [--title t] [--description d] [--as-json] | …` | save (or replace) the file as a draft; with `--as-json` print the JSON and save nothing |
| `activate pipeline <path>` | promote the file to live traffic (validates node config, node availability, libraries) |
| `deactivate pipeline <path>` | stop serving; the file stays |
| `execute pipeline <path> --input '{"k":"v"}'` | run the live version once with that payload |
| `run | trigger.function | script -- "return 1"` | run a body once, unsaved and unlogged; `run --dry-run` only parses |
| `patch pipeline <path> node <id> [--flag v]… [-- body]` | change one node's config; the pipeline becomes `stale` until activated |
| `get pipelines | nodes | connections | credentials | templates | docs` | list |
| `describe pipeline <path> [--compact]` | status, hash, hits, the DSL, and the node ids for `patch` |
| `describe connection <slug>`, `describe node <kind>`, `node help <kind>` | one resource |
| `git status | log --max-count=10 | diff | add <path> | commit -m "…"` | the project repository |

`<path>` is the `file_rel_path`; `.zf.json` may be omitted. Status is `draft`,
`active`, or `stale` (live but changed since activation). There is no
`delete` verb (deletion goes through the pipelines API or Studio), no
`--help`, no `read`/`write` of files or docs from the console — those are the
MCP file tools.

Over MCP the same verbs are `pipeline_register`, `pipeline_activate`,
`pipeline_deactivate`, `pipeline_execute`, `pipeline_run`, `pipeline_patch`,
`pipeline_list`, `pipeline_describe`, `pipeline_get`, `pipeline_get_invocations`,
`pipeline_search`, `git_command`; there is no MCP tool that takes a raw DSL
string.

---

## Control flow

| Node | Pins | Flags |
|---|---|---|
| `logic.if` | `true`, `false` | `--expr "input.count > 0"` |
| `logic.match` | one per case + `--default` | `--expr "input.body.type" --cases create,update --default other` |
| `logic.foreach` | `item` | `--items-expr "input.rows" [--chunk-size N] [--keep-input]` |
| `logic.collect` | `out` | none — fires once every wired input has arrived; payload `{ <upstream id>: payload, … }` |
| `logic.reduce` | `out` | `--init-expr "{ total: 0 }" --step-expr "{ total: $acc.total + $input.item.amount }"` |
| `logic.retry` | `retry`, `failed` | `--max-attempts 3 [--delay-ms 250]`, wired from an `:error` pin |

**Fan-out** is any node with several outgoing edges; **fan-in** is `logic.collect`:

```
[a] trigger.manual
[b] http.request --url https://source-a.example.com/data --method GET
[c] http.request --url https://source-b.example.com/data --method GET
[d] logic.collect
[e] script -- "return { a: input.b.response.body, b: input.c.response.body }"
[a] -> [b]
[a] -> [c]
[b] -> [d]
[c] -> [d]
[d] -> [e]
```

**foreach** emits `{ item, index, count }` per element (add `--keep-input` to
carry the whole upstream payload — off by default so a large table is not
copied per row); **reduce** folds the series:

```
[a] trigger.manual
[b] logic.foreach --items-expr "input.rows"
[c] logic.reduce --init-expr "{ total: 0 }" --step-expr "{ total: $acc.total + $input.item.amount }"
[a] -> [b]
[b]:item -> [c]
```

**retry** listens on `:error` and re-enters the failing node:

```
[a] trigger.manual
[b] http.request --url https://api.example.com/work --method POST
[r] logic.retry --max-attempts 3 --delay-ms 250
[c] script -- "return input"
[d] script -- "return { failed: true }"
[a] -> [b]
[b] -> [c]
[b]:error -> [r]
[r]:retry -> [b]
[r]:failed -> [d]
```

**Loops** are back-edges; bound them with a counter in the payload:

```
[a] trigger.manual
[b] script -- "const n = (input.attempts || 0) + 1; return { ...input, attempts: n, status: n < 3 ? 'retry' : 'done' }"
[c] logic.match --expr "input.status" --cases done --default retry
[d] script -- "return { result: input }"
[e] logic.if --expr "input.attempts < 5"
[f] script -- "return { gave_up: true }"
[a] -> [b]
[b] -> [c]
[c]:done -> [d]
[c]:retry -> [e]
[e]:true -> [b]
[e]:false -> [f]
```

---

## Webhooks: auth and streaming

```
| trigger.webhook --path /admin/posts --method POST --auth-type jwt --auth-credential jwt_main --auth-required-role admin,editor
```

`--auth-type` is `none`, `jwt`, `hmac` or `api_key`; `--auth-credential` is
the credential id; `--auth-required-role` matches one entry of the token's
`roles` array. Payload shape (`input.body`, `input.params`, `input.query`,
`input.files`, `input.auth`): `help("pipeline/authoring")`.

Any webhook pipeline streams when the client asks: a request with
`Accept: text/event-stream` receives `event: signal` messages while nodes
run (anything a node emits on the execution bus, such as `n.ai.agent`
thinking and tool calls), then `event: done` with the result or
`event: error`. The pipeline definition is the same either way.

---

## Files and FileRefs

Bytes never travel inline. A file is a **FileRef** —
`{ "__zf_type": "file_ref", "ref": "…", "filename", "mime", "kind", "size", "sha256", "lifecycle", "origin", "trust" }` —
produced by an upload (`input.files.<field>`), by `http.request --response-type bytes`,
or by any `fs.*` node. `fs.save` keeps an uploaded file:

```
| trigger.webhook --path /upload --method POST
| fs.save --field photo --folder public/uploads --allowed-kinds images --max-size 10
| fs.thumbnail --width 320 --height 320 --fit cover --format webp --folder public/thumbs --source-key saved.path
```

`fs.save` adds `saved: { path, url, original_name, content_type, size }` to
the payload and `fs.thumbnail` adds `thumbnail` (a FileRef); the form's other
fields (`input.body.caption`) stay beside them.
Anything under `public/` is served anonymously at
`/files/{owner}/{project}/<path>`; everything else needs a session
(`/fs/{owner}/{project}/<path>`). Table files (`table.convert`, `table.query`)
and map layers (`ms.*`) follow the same convention.

---

## The node catalog

`help(topic="pipeline/nodes")` is generated from the node definitions and is
the only complete list. Families:

- **Triggers** `trigger.webhook · schedule · function · manual · ws · ws.client · kv.subscribe · mcp · weberror`
- **Data** `sekejap.query · sekejap.insert · pg.query · sqlite.query · sqlite.mutate · table.query · table.convert · kv.get · kv.set · kv.incr · kv.exists · kv.del · kv.expire · kv.publish`
- **Compute** `script · logic.* · crypto · function.call · ai.agent · ai.embedding · ai.tts · browser.run · concept`
- **I/O** `http.request · mail.send · telegram.* · fs.* · geo.inspect · geo.convert · ms.*`
- **Answer** `web.response · web.static.generate · web.docs.generate · ws.emit · ws.sync_state · ws.client.send · auth.token.create · auth.token.verify`

Recipes that use them end to end: `help("pipeline/examples")`.
