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
| `$trigger` | the trigger snapshot: `params`, `query`, `search`, `pathname`, `headers` (`host`, `x-forwarded-proto`, `content-type`, `user-agent`, `referer`, `origin`, …), `auth` — never `body` |
| `$nodes.<id>` | the output of an upstream node by its id (in pipe mode `n0`, `n1`, …). The trigger's own output — including `body` — is `$nodes.<trigger id>` |
| `$item`, `$index`, `$count` | inside a `logic.foreach` branch |

A value that is **only** an expression keeps its JSON type; an expression
inside a longer string is stringified. There is no `ctx`, `$ctx` or `env` in
`{{ }}` (a script has `ctx.trigger.*` and `ctx.request_id`). An undefined
name throws and fails the node instead of silently writing `null`.

**Size.** The sandbox does not cap what an expression or a `script` returns
(a 4 MB string goes through both); its one byte limit, `max_output_bytes` —
256 KB by default, clamped to 256 B–1 MB (`deno_sandbox/config.rs`) — is
the ceiling for a local `fetch('/path')` read inside a script. Bytes still
do not belong in a payload: whatever a node returns is carried in `$nodes`,
the run record and every downstream payload. Pass files as FileRefs, shrink
an image with `fs.image.thumbnail` before anything reads it, and read bytes inline
only for a provider that needs them: `fs.get --path <p> --encoding base64`
answers the bytes at `input.fs.object.base64`, which a `{{ }}` body can prefix with
`data:image/jpeg;base64,` — the story pipeline sends its reference photo as
a 640 px thumbnail (~120 KB as a data URI) rather than the 1200 px original.

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
| `patch pipeline <path> note <id> [--text t] [--at x,y] [--size WxH] [--color c] [-- text]` | create or change one canvas note; `--remove` deletes it (see Notes) |
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
| `logic.retry` | `retry`, `failed`, `done` | `--max-attempts 3 [--delay-ms 250] [--backoff 2] [--max-delay-ms 8000] [--max-elapsed-ms 30000] [--when "<expr>"]`, wired from an `:error` pin or fed a verdict (`retry: true`) |

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

A failure an `:error` edge consumes is not the run failing: the record marks
the attempt `retry` (or `error_routed` when the edge reaches something other
than `logic.retry`), the canvas draws an orange ring with the count, and the
run's status is untouched. Red is for a failure nothing consumed.

**Back-off and a time cap.** `--backoff 2` doubles `--delay-ms` each attempt
and `--max-delay-ms` holds the grown wait (`--max-attempts 6 --delay-ms 500
--backoff 2 --max-delay-ms 8000` waits 500, 1000, 2000, 4000, 8000 ms);
`--max-elapsed-ms` is a wall-time budget from the first attempt — when it is
spent, or the next wait would overrun it, `failed` fires with attempts left,
and `__zf_retry.reason` (`max_attempts` | `max_elapsed_ms`) and
`__zf_retry.message` on the payload say which budget ran out.

**Polling** is a wait, not an error, so it need not throw. `logic.retry` also
takes a **verdict** on an ordinary edge: `retry: true` on the payload (or
`--when "<expr>"` true — JavaScript over `input`, as `logic.if --expr`) fires
`retry` with that payload; false passes it through on `done`; the budget
spent fires `failed`. The node counts its own attempts
(`$nodes.<r>.__zf_retry.attempt`), so a poll that replaces the payload each
round still counts 1, 2, 3; each round is a `retry` entry on the retry node
(`node_retry` on the stream: "waiting 2/40"), and nothing is ever red:

```
[t] trigger.manual
[poll] http.request --url https://api.example.com/jobs/42 --method GET
[check] script -- "const d = (input.response.body.data || [])[0] || {}; return { ...input, retry: d.status !== 'success', url: d.videoURL }"
[wait] logic.retry --max-attempts 40 --delay-ms 5000
[download] http.request --url "{{ input.url }}" --response-type bytes
[gaveup] script -- "return { gaveup: true, attempts: input.__zf_retry.attempt }"
[t] -> [poll]
[poll] -> [check]
[check] -> [wait]
[wait]:retry -> [poll]
[wait]:done -> [download]
[wait]:failed -> [gaveup]
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

## Notes on the canvas

A note is a sticky note drawn on the pipeline canvas: text for the next
reader, never executed, never reached by an edge. Use one to say which
credential to create, why a branch exists, or what is still missing. `note`
is a reserved word in both modes:

```
[t] trigger.webhook --path /signup
[m] mail.send --credential smtp-main --to "{{ input.body.email }}"
[t] -> [m]
[why] note --text "Create the `smtp-main` credential before activating." --at 120,-80 --size 320x90 --color amber

| trigger.webhook --path /signup | mail.send --credential smtp-main | note --id why --text "Create the smtp-main credential first."
```

Flags: `--text` (markdown: headings, bullets, **bold**, `code`), `--at x,y`
(canvas position, the space node positions use), `--size WxH`, `--color`
(`amber`, `blue`, `green`, `rose`, `violet`, `teal`, `orange`, `slate`; anything
else draws grey — the dot at a note's top-right on the canvas picks from the
same eight, and Save Draft keeps it), `--id`
(pipe mode only; graph mode takes the `[label]`). A `-- body` after the flags
is the text too, for anything long.

`register` replaces the nodes and edges and **keeps every existing note** the
body did not redeclare, so a note a person left on the canvas survives an
agent's re-register. To change or delete one: `patch pipeline <path> note
<id> …` / `--remove`. `describe pipeline` lists note ids next to the node ids.

## Previews under a node

A preview draws one value from the node's latest run under its box on the
canvas. Off by default, presentation only, **never read by the engine** — it
lives in `config.preview` beside `config.ui`, and removing it changes nothing
about what the pipeline does.

```
| trigger.manual | script --preview table:rows -- return { rows: await db.all() } | web.response --preview-in json:body
```

`--preview <as>[:<path>]` previews the node's **output**; `--preview-in
<as>[:<path>]` previews its **input**. `as` is one of `image`, `video`,
`audio`, `pdf`, `json`, `text`, `table`, `html`. `path` is optional: a dot path
into that payload (`response.file`, `clip`, `rows`); leave it off and the
editor picks the first value in the payload that fits the kind. A file
reference renders as the file, a `http(s)://` or `/` string as that URL.

Both flags take `off` to remove that half: `patch pipeline
pipelines/reports/daily node render --preview off`.

A panel is 220 px wide by default, with a height by kind. `@WxH` after the
value sets its size on the canvas, in canvas pixels — `--preview
image@360x240`, `--preview table:rows@420x180`, `--preview-in json@300x90` —
between 160×60 and 1200×900. The panel stays centred under its node whatever
the width. The value is the whole cell: `--preview image@400x260` means kind
image, no path, that size, and `--preview image` puts it back at the default.
The same thing by hand: drag the corner on the canvas; Save Draft keeps it,
and `describe pipeline` prints the suffix only when a size is stored.

A declared preview records that payload in the run's trace even at the
default `on-error` capture level, so it has something to draw; a pipeline at
level `none` records nothing and the cell says "capture off".

Every node kind takes both flags. The table nodes sample rows into the
payload with `--preview-rows <n>` and can preview that sample:

```
| table.query --from "datasets/orders.csv as o" --preview-rows 5 --preview table:table.preview -- "SELECT * FROM o"
```

## Inputs

A trigger delivers one envelope: `body` (fields) and `files` (FileRefs). A
file that arrives at a trigger is `lifecycle: temporary` — it lives for the run
and is deleted after, unless a node such as `fs.save` makes it durable. The
webhook already does this for a multipart post; a **manual run** delivers the
same envelope (the Studio's Run form, or `pipelines/execute` as JSON or
multipart), and **input nodes** declare and check one field of that envelope
each, so the Studio can build a Run form from the graph and an agent can read
what a pipeline expects.

An input node names one field, checks it, and passes the envelope through
**unchanged** (a missing field with a `--default` is filled in, below). It
fetches nothing and stores nothing. The set of input nodes
reachable from a trigger *is* that trigger's declaration. The same nodes work
after `trigger.manual` and after `trigger.webhook`.

| kind | value | checks |
|---|---|---|
| `input.text` | string | non-empty unless optional; `--max` length |
| `input.number` | number | `--min`, `--max` |
| `input.boolean` | bool | — |
| `input.json` | any JSON | parses (a JSON string in `body` is parsed) |
| `input.file` | one FileRef | `--accept` (comma list of FileRef kinds, mimes or extensions) |
| `input.files` | array of FileRef | `--accept`, `--max` count |
| `input.image` / `input.audio` / `input.video` | one FileRef | `input.file` with `--accept` preset to that kind |

The field name is the first bare token (also `--name`). Every kind takes
`--optional`, `--label "…"` (the form label), and `--default <v>` (text,
number, boolean, json). Body fields are read at `input.body.<name>`, files at
`input.files.<name>`. The payload leaves as it arrived, so the next node still
reads `input.body.<name>` and `$trigger.files.<name>`; the node's own value —
`$nodes.<id>` — is the checked string, number or FileRef. A body field that
was not sent and has a `--default` is written into the envelope at
`body.<name>`, so `input.body.<name>` and `$nodes.<id>` agree whether the run
came from the Run form, `execute pipeline`, MCP or a webhook.

**Required by default.** A missing or invalid value fails the node with
`FW_NODE_INPUT_MISSING` / `FW_NODE_INPUT_INVALID` (both *refused*), naming the
field and what was expected. `--optional` allows absence; the node's value is
then `null`. After `trigger.schedule` a required input is refused at
activation (`FW_NODE_INPUT_UNREACHABLE`): a tick delivers an empty envelope,
so give it `--default` or `--optional` — a schedule pipeline run by hand
still receives the real envelope and checks it as usual.

A manual run with an image and a caption — the Studio draws a text field and a
drop zone under the two nodes, and Run posts them as multipart:

```
| trigger.manual
| input.text prompt --label "Caption" --max 200
| input.image photo
| fs.image.thumbnail --source-key files.photo --width 200 --height 200 --preview image
| script --preview json -- return { caption: $trigger.body.prompt, thumb: input.thumbnail }
```

A webhook form that takes a CV — a browser posts the same multipart, and the
same Run button tries the route from the canvas:

```
| trigger.webhook --path /apply --method POST
| input.text name --label "Full name"
| input.file cv --accept pdf
| fs.save --field cv --folder applications --allowed-kinds documents
| web.response --status 200 --body "{{ { received: input.saved.ref } }}"
```

Over MCP: `pipeline_execute` with
`input: { body: { prompt: "x" }, files: { photo: "uploads/cat.png" } }` — a
`files` value is a **store path** (project files, private or public) that
becomes the durable FileRef of that object, `origin: manual`; a path that does
not exist is refused, naming it, before the run starts. The JSON form of
`pipelines/execute` takes the same spelling.

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
run, then `event: done` with the result or `event: error`. The pipeline
definition is the same either way. A signal is anything a node emits on the
execution bus (`n.ai.agent` thinking and tool calls, a script's `emit`) —
and, since 2026-09-20, the engine's own lifecycle: `run_start`, then per
node `node_start` and exactly one of `node_ok` / `node_skip` / `node_fail`
(`data: { duration_ms, error_code?, error_class? }`, the node in `node_id`),
then `run_done` (`data: { run_id, status, duration_ms }`). Since 2026-09-21
a failure an `:error` edge consumed is not `node_fail`: it is `node_retry`
(`data: { attempt, max_attempts, duration_ms, error_code, message }`) when
the edge reaches `logic.retry`, or `node_error_routed` (`data: {
duration_ms, error_code, message, to_node }`) for any other consumer;
`node_fail` is the unrouted failure that ends the run. A `logic.retry` that
sends a verdict round again announces `node_retry` for itself (`message:
"next attempt in 5000 ms"` when it has a delay). A client that only cared for a node's own signals now sees
these too; filter on `kind`.

`POST /api/projects/{o}/{p}/pipelines/execute` streams the same way with
the same header (JSON or multipart body, as without it): every signal as
`event: signal`, then one `event: result` carrying what the JSON answer
would have been — `{ ok, run_id, output }` or `{ ok: false, run_id, error }`.
The Studio's Run button uses this to light each node's badge as it runs.
Without the header the route answers JSON once the run is over, exactly as
before.

---

## Files and FileRefs

Bytes never travel inline. A file is a **FileRef** —
`{ "__zf_type": "file_ref", "ref": "…", "filename", "mime", "kind", "size", "sha256", "lifecycle", "origin", "trust" }` —
produced by an upload (`input.files.<field>`), by `http.request --response-type bytes`,
or by any `fs.*` node. `fs.save` keeps an uploaded file:

```
| trigger.webhook --path /upload --method POST
| fs.save --field photo --folder public/uploads --allowed-kinds images --max-size 10
| fs.image.thumbnail --width 320 --height 320 --fit cover --format webp --folder public/thumbs
```

`fs.save` adds `saved` — a durable FileRef and nothing else (`__zf_type`,
`backend`, `ref`, `filename`, `mime`, `kind`, `size`, `sha256`,
`lifecycle: durable`, `origin: fs.save`, `trust`); the store path is
`saved.ref`, and there is no `path`, `url` or `content_type` beside it —
and `fs.image.thumbnail` (whose `--source-key` defaults to `saved`, the FileRef
itself) adds `thumbnail` (a FileRef); the form's other fields
(`input.body.caption`) stay beside them. `fs.put`, `fs.copy`, `fs.move`
(their `object` under `fs`) and `fs.compress` (`compressed`) answer a stored
file the same way: a bare FileRef. Store `ref` in a row; a URL is not a
node's business — a page derives it from the folder (below).
Anything under `public/` is served anonymously at
`/files/{owner}/{project}/<path>`; everything else is private. The Studio
(a preview cell, an input widget, the Files page) and an MCP session read any
object, private or public, at
`GET /api/projects/{owner}/{project}/files/object?ref=<path>` with the
session — `/fs/{owner}/{project}/<path>` is a public surface for sites, off
until Settings → Addressing turns it on. Table files (`table.convert`,
`table.query`) and map layers (`ms.*`) follow the same convention.

**Provider APIs.** An image, video or speech provider (Runware, fal,
Replicate, ElevenLabs) is called with a **Secure Request** credential. The
owner creates the profile in Studio → Credentials: method, URL template,
header templates such as `Authorization: Bearer <API_KEY>`, and the secret
behind the placeholder; the node names it with `--credential <id>` and needs
neither `--url` nor `--method`. A profile whose Body Template is blank sends
the node's own `--body "{{ expr }}"`, so the payload's shape stays in the
pipeline and only the key lives in the credential. Send a FileRef to the
provider with `--body-type form-data` and a FileRef as one field of the body;
receive one with `--response-type bytes`, which stores the reply as a
temporary FileRef at `response.body`; then `fs.save` keeps it — it reads
`response.body` when there is no upload field. The key is redacted from the
run's record at every capture level. On the canvas, the download node's
`--preview image` can only ever say "temporary file — gone after the run":
the bytes were deleted with the run, by design. The `fs.save` node's preview
is the one that shows the picture, from the durable file it wrote.

```
| trigger.manual
| input.text prompt --label "Describe the image"
| script -- "return { body: input.body, taskUUID: crypto.randomUUID() }"
| http.request --credential runware --body "{{ [ { taskType: 'imageInference', taskUUID: input.taskUUID, positivePrompt: input.body.prompt, model: 'runware:101@1', width: 1024, height: 1024, numberResults: 1, outputType: 'URL', outputFormat: 'JPG' } ] }}" --preview json:response.body
| script -- "return { url: input.response.body.data[0].imageURL }"
| http.request --url "{{ input.url }}" --response-type bytes --preview image
| fs.save --folder generated/runware --preview image
```

The profile `runware` is `POST https://api.runware.ai/v1` with
`Authorization: Bearer <API_KEY>` and `Content-Type: application/json`, no
variables. Each run spends the provider's credits.

## Posters and SVG pictures

A poster is an SVG. Whoever writes it — a model, a script, a stored file —
`fs.svg.convert` draws it as a PNG, JPG or WebP with resvg, no browser.
Text is shaped with a family the project has: the bundled Inter
(400/500/600/800) or any `.ttf`/`.otf` under the repository's
`static/fonts/`, named by family, never by file; a family nobody has is
refused with the list. `<text inline-size="918">` (SVG 2) wraps a headline
into lines measured by the shaper — one `<tspan>` per line. Pictures come
from the project only: `<image href="sandbox/posters/photos/venue.jpg">`
is a store path, `repo://static/brand/logo.svg` a repository file; a URL or
a `data:` URI is refused — fetch with `http.request --response-type bytes`,
`fs.save` it, then name the path.

```
| trigger.manual
| input.text brief --label "What the poster is for"
| ai.agent --credential openrouter --output-mode final_only --schema '{"type":"object","required":["svg"],"properties":{"svg":{"type":"string"}}}' -- Write one 1080x1350 SVG poster (xmlns, width and height set, font-family Inter, the headline as a <text> with inline-size="918") for: {{ input.body.brief }}
| fs.svg.convert --source-key data.svg --folder sandbox/posters/out --preview image
```

The flags are `fs.image.thumbnail`'s. With no `--width`/`--height` the
canvas is the SVG's own size; one side scales the other in proportion; both
go through `--fit cover|contain|fill`. `--format png|jpg|webp` (default
png), `--quality` for jpg, `--folder` (default `images/`), `--filename`,
`--delete-source`. The answer adds `image` — a durable FileRef
(`origin: fs.svg.convert`) with `width`, `height`, `format` — and keeps the
rest of the payload, so `data.svg` is still there for the next node, and
`layout`: every text and picture with its box, the pairs that overlap, what
leaves the canvas, and `ok`. A `script` turns that into a verdict —
`retry: !input.layout.ok` with the overlaps as notes — and `logic.retry`
sends the agent round again with the notes, so a composition is corrected
without anyone looking at pixels; the picture nodes stay upstream of the
loop and are not paid for twice.

`--format pdf` writes the same SVG as one PDF page: vector shapes stay
vector and text stays text with the font subset embedded, so a name is
selectable. `data-fit="shrink"` beside `inline-size` shrinks a `<text>` until
it fits `data-max-lines` (default 1) — a certificate is a stored template
`.svg`, a `fs.get`, a `script` that fills the placeholders, then
`fs.svg.convert --format pdf --folder certificates --filename cert-<number>`.
Effects — shadow, blur, glow, grain, colour grading — are SVG filters
(`feDropShadow`, `feGaussianBlur`, `feColorMatrix`, `feTurbulence`); resvg
draws them and the PDF keeps them.

**A generated picture inside the poster.** Ask the image model for the
subject "on a solid flat #00ff00 green screen background", `fs.save` it,
then `fs.image.chromakey --folder sandbox/posters/cutouts` turns the screen
transparent (plain pixel maths, no model; the default key is broadcast
green `#00b140`, which is what the models paint) and answers
`image`, a PNG with alpha. The agent places it with
`<image href="{{ input.image.ref }}" x="…" y="…" width="…" height="…"/>`
and `fs.svg.convert` composites it over the background.

**Temporary previews.** The bytes of a temporary FileRef (an
`http.request --response-type bytes` answer, a Run-form upload) are deleted
with the run, so a preview of one used to say only "temporary file — gone".
Since 2026-09-21 a node whose `--preview image` (or `--preview-in image`)
points at a temporary image gets a small JPEG snapshot (longest side 540 px,
≤ 64 KB) written into the run's record beside the payload, and the canvas
draws that with the caption "temporary — not saved".

---

## The node catalog

`help(topic="pipeline/nodes")` is generated from the node definitions: one
line per kind, by family, so you can find the name before reading the flags.
`help(topic="pipeline/nodes/<kind>")` is one node in full;
`help(topic="pipeline/nodes/all")` is every node in full. Families:

Every kind, by family, from the live catalogue (`help("pipeline/nodes")` for one line each, `help("pipeline/nodes/<kind>")` for the flags):

<!-- node-families -->

Recipes that use them end to end: `help("pipeline/examples")`.
