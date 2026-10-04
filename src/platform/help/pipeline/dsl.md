# Pipeline DSL

The DSL is the text form of a pipeline: one node per line, `|` between them.
This page is the grammar every node follows. What one node takes and answers
is generated from its definition: `help("pipeline/nodes/<kind>")`.

| Channel | Form |
|---|---|
| MCP | `pipeline_register file_rel_path="api/posts" body="| trigger.webhook … | …"`; `pipeline_run body="…"` runs a body once, unsaved |
| Project console | `register api/posts --title "Posts" | trigger.webhook … | …`, then `activate pipeline api/posts` |
| HTTP | `POST /api/projects/{o}/{p}/pipelines/dsl` with `{"dsl": "register …"}` (write the JSON to a file and `-d @file` — flags do not survive shell quoting) |

The parsed result is the JSON document described in `help("pipeline/authoring")`.

---

## Two modes

**Pipe mode** — a straight chain. The first node is the entry; each node
receives the previous node's payload as `input`. Node ids are `n0, n1, …`.

```
| trigger.webhook --route /posts/:slug --method GET
| sekejap.query.run --param "1={{ $trigger.params.slug }}" -- "SELECT * FROM posts WHERE slug = $1"
| web.response.send --template pages/post.tsx
```

**Graph mode** — label each node `[id]`, then wire edges. Needed to branch,
fan out, join and loop.

```
[a] trigger.webhook --route /status --method GET
[b] http.response.fetch --url https://example.com/health --method GET
[c] logic.if --when "input.response.status >= 400"
[d] http.response.fetch --url https://hooks.example.com/alert --method POST --body "{{ input.response }}"
[e] web.response.send --body "{{ { ok: true } }}"
[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [e]
[d] -> [e]
```

Edges are `[from] -> [to]` (pin `out` to pin `in`), `[from]:pin -> [to]` or
`[from]:pin -> [to]:pin`. A pin with no edge ends that branch. Every node
needs a path from the one entry — a node with no incoming edge is a second
entry and runs on every trigger. Every node has an `:error` pin.

---

## Kinds

A node is written as its kind. Every kind has one of four shapes:

```
family.noun.verb       an acting node     fs.image.thumbnail   postgres.query.run   telegram.message.send
trigger.<source>       starts a run       trigger.webhook      trigger.schedule
input.<type>           declares an input  input.text           input.image
logic.<verb>           control            logic.if             logic.foreach
```

- The **family** is the word you would search for — a domain (`fs`, `kv`,
  `http`, `ai`), an engine (`postgres`, `sqlite`, `sekejap`), a language
  (`javascript`, `typescript`) or a brand (`telegram`).
- The **noun** is what the node makes or acts on, and it is the node's
  answer key (below). The **verb** says what happens to it: `get` `list`
  `put` `create` `delete` `run` `send` `generate` `render` `convert` ….
- A node installed from the Hub or built in the project is
  `x.<package>.<noun>.<verb>`; everything after the package follows the same
  rules.

Find a node in three steps: `help("pipeline/nodes")` (every kind on one
line, by family) → `help("pipeline/nodes/<kind>")` (its signature, flags,
answer and examples) → `help_search query="…"` when you only know the task.

---

## One answer key

Every node adds **one key** to the payload — its noun — and keeps the rest.
Everything about its result is inside that key:

```
| trigger.webhook --route /photos --method POST
| fs.file.put --from "{{ input.webhook.files.photo }}" --accept image
| fs.image.thumbnail --from "{{ input.file }}" --width 320 --height 320 --fit cover
| sekejap.query.run --write --param "1={{ input.file.ref }}" --param "2={{ input.image.ref }}" -- "INSERT INTO photos (original, thumb) VALUES ($1, $2)"
| web.response.send --status 201 --body "{{ { original: input.file.ref, thumb: input.image.ref } }}"
```

After the trigger the payload is `{ webhook }`; after `fs.file.put` it is
`{ webhook, file }`; after the thumbnail `{ webhook, file, image }`; after the
query `{ webhook, file, image, query }`. Nothing is replaced, so the last node
still reads `input.file.ref` two nodes later.

- A trigger answers under its source: `input.webhook.body` right after
  `trigger.webhook`, `input.schedule` after `trigger.schedule`. The same
  envelope is `$trigger` for the whole run, so a later node writes
  `$trigger.body`, `$trigger.params`, `$trigger.query`, `$trigger.files`,
  `$trigger.auth` — the request does not move when the payload grows.
- An input node answers under its `--name`: after `input.text prompt` the
  value is `input.prompt`.
- Any upstream node's answer stays at `$nodes.<id>.<key>`
  (`$nodes.n1.query.rows`, `$nodes.thumb.image.ref`).
- A node that routes its own failures answers the same key as
  `{ ok: false, error: { code, message } }` on its error pin; its page says
  when it does.

The shape inside each key is on the node's page; read it there rather than
guessing from a neighbour.

---

## Flags

`--kebab-case`, declared per node; an undeclared flag is a parse error
(`unknown flag --x`). The words mean the same thing on every node:

- **`--from` names the subject** — the thing of the node's own noun it reads:
  `fs.image.thumbnail --from "{{ input.file }}"`, `crypto.password.verify
  --from "{{ $trigger.body.password }}"`, `logic.foreach --from "input.query.rows"`.
  No node reads a payload key on its own; every source is a flag.
- **Other inputs take typed roles** — `--text`, `--image`, `--file`,
  `--body` (what goes over a wire), `--value` (what a store write holds),
  `--argument` (what a callable receives), `--prompt`. A role is a singular
  noun; repeat it for several (`--recipient a@example.com --recipient
  b@example.com`).
- **A map is repeated `key=value`**, split at the first `=`:
  `--param "1={{ $trigger.params.id }}" --param "2=draft"`,
  `--header "Accept=application/json"`, `--claim "sub={{ input.query.rows[0]._key }}"`.
- **A switch is a bare flag** for the behaviour it turns on — `--write`,
  `--durable`, `--optional`. There is no `--no-x`.
- **Units travel in the value**: durations `ms` `s` `m` `h` `d`
  (`--timeout 30s`, `--ttl 1h`, `--delay 250ms`), sizes `B` `KB` `MB` `GB`
  or `KiB` `MiB` `GiB` (`--max-size 10MB`). A bare number for a duration or
  a size is refused.
- **A choice is closed**: `--method POST`, `--fit cover`, `--on-conflict
  overwrite`. A word the node does not list is refused, never mapped to a
  default; the signature shows the words (`--fit cover|contain|fill`).
- **A file writer names its destination** with `--store`, `--folder`,
  `--filename`, `--path` (an exact store key — the only flag that is a path)
  and `--on-conflict`.
- **Every node takes** `--title "…"` (the canvas label), `--timeout <duration>`
  (1s to 1h; default the project's node timeout), `--preview` and
  `--preview-in` (below).

How each kind of value is written:

| Value | Example | Stored config |
|---|---|---|
| one | `--template pages/post.tsx` | `"pages/post.tsx"` |
| switch | `--write` | `true` — consumes no value |
| repeated | `--case create --case update` | `["create","update"]`; a comma stays inside its value |
| key=value | `--param "1={{ $trigger.body.id }}" --param 2=draft` | `{ "1": …, "2": "draft" }` |

A **body** after a standalone `--` is the node's main text — the SQL of a
query node, the code of a script node, the prompt of `ai.text.generate`:

```
| sqlite.query.run --param "1={{ $trigger.query.q }}" -- "SELECT id, title FROM notes WHERE instr(title, ?1) > 0"
```

**Quoting.** A value with a space or `{{ }}` is one double-quoted argument.
`--header Location={{ input.url }}` unquoted is cut at the first space and
refused with a message that says so. End a line with `\` to continue it.

---

## Data, headers and providers

Three conventions hold on every node that has them:

- **Values never go into statement text.** SQL is the body; values bind
  through `--param` (`1=` is `$1`, or `?1` in SQLite; `name=` is `:name`).
  A whole `{{ }}` keeps its JSON type. A query node is read-only unless it
  has `--write`:

  ```
  | trigger.webhook --route /api/notes --method POST
  | sekejap.query.run --write --param "1={{ $trigger.body.title }}" --param "2={{ new Date().toISOString() }}" -- "INSERT INTO notes (title, created_at) VALUES ($1, $2)"
  | web.response.send --status 303 --header "Location=/notes"
  ```

- **Headers are sent as written.** `--header "K=V"`, repeated; nothing is
  added or rewritten, `Set-Cookie` included, so a cookie writes its own
  attributes:

  ```
  | web.response.send --status 303 --header "Location=/home" --header "Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"
  ```

- **Secrets are credentials, named by id.** `--credential <id>` takes an id
  from `credential_list`; no value is ever written in a pipeline. A task
  several vendors offer is one kind with `--provider` (literal), and the
  provider's own settings go through `--option key=value`. The node's page
  prints one signature per provider:

  ```
  | trigger.manual
  | input.text question
  | ai.text.generate --provider openrouter --credential openrouter_main --prompt "{{ input.question }}"
  ```

---

## `{{ expr }}` — values computed at run time

Any flag value may hold `{{ js_expression }}`, resolved just before the node
runs, in a sandbox with no I/O and a bounded op budget.

| Name | Meaning |
|---|---|
| `input`, `$input` | the payload arriving at this node |
| `$trigger` | the trigger's envelope for the whole run — for a webhook `body`, `query`, `params`, `headers`, `files`, `method`, `path`, `auth`, plus `search` and `pathname` |
| `$nodes.<id>` | an upstream node's payload by id (`n0`, `n1`, … in pipe mode); its answer is `$nodes.<id>.<key>`. Only a node an edge path leads from can be named — any other is refused when the pipeline is activated, with "did you mean" — and one that was skipped is `null` |
| `$item`, `$index`, `$count` | inside a `logic.foreach` branch |

A value that is **only** an expression keeps its JSON type (`"{{ [1, 2] }}"`
is an array); an expression inside a longer string is stringified. There is
no `ctx`, `$ctx` or `env` in `{{ }}` — a script has `ctx.trigger.*` and
`ctx.nodes.*` instead. An undefined name fails the node rather than writing
`null` — except a path through `$nodes`: a skipped node is `null` and
`$nodes.big.text` reads as `$nodes.big?.text`, so a path through it is `null`
too. JavaScript's `??` and `?.` work: `{{ $nodes.big.text ?? $nodes.small.text }}`
joins two branches.

```
| sekejap.query.run --param "1={{ $trigger.params.id }}" -- "SELECT * FROM users WHERE id = $1"
| http.response.fetch --url "https://api.example.com/users/{{ input.query.rows[0].slug }}"
| web.response.send --status 302 --header "Location={{ $trigger.query.next || '/dashboard' }}"
```

**Size.** Whatever a node answers is carried in `$nodes`, the run record and
every later payload, so bytes do not belong there. Pass files as FileRefs,
shrink an image with `fs.image.thumbnail` before anything reads it, and read
bytes inline only for a provider that needs them (`fs.file.get --encoding base64`).

---

## Control flow

`logic.*` nodes route the payload; they pass it on rather than answer.
Their pins and flags are on their pages (`help("pipeline/nodes/logic.match")`).

**Branch** — `logic.if` on `true`/`false`, `logic.match` on one pin per case:

```
[a] trigger.webhook --route /ingest --method POST
[b] logic.match --from "input.webhook.body.type" --case normal --case urgent --default other
[c] sekejap.query.run --write --param "1={{ $trigger.body }}" -- "INSERT INTO normal_queue (data) VALUES ($1)"
[d] http.response.fetch --url https://alert.example.com/send --method POST --body "{{ $trigger.body }}"
[e] sekejap.query.run --write --param "1={{ $trigger.body }}" -- "INSERT INTO other_queue (data) VALUES ($1)"
[a] -> [b]
[b]:normal -> [c]
[b]:urgent -> [d]
[b]:other -> [e]
```

**Fan out and join.** A node with several outgoing edges fans out. A node
runs **once**, when every edge into it has delivered or been skipped — two
branches meeting is a join, not two runs. Its `input` is the delivered
payloads merged in DSL text order: a key from a node written later wins,
whichever finished first. A branch not taken is **skipped**, and so is every
node only it feeds; a reference to a skipped node is `null`:

```
[a] trigger.webhook --route /greet --method POST
[b] logic.if --when "input.webhook.body.formal"
[c] javascript.script.run -- "return { text: 'Good morning' };"
[d] javascript.script.run -- "return { text: 'Hi' };"
[e] web.response.send --body "{{ $nodes.c.script.text ?? $nodes.d.script.text }}"
[a] -> [b]
[b]:true -> [c]
[b]:false -> [d]
[c] -> [e]
[d] -> [e]
```

`logic.collect` where branches meet also lists what arrived —
`input.collect.items` in text order, a skipped branch missing — and each
branch's answer stays at `$nodes.<id>`:

```
[a] trigger.manual
[b] http.response.fetch --url https://source-a.example.com/data --method GET
[c] http.response.fetch --url https://source-b.example.com/data --method GET
[d] logic.collect
[e] javascript.script.run -- "return { a: $nodes.b.response.body, b: $nodes.c.response.body, count: input.collect.count }"
[a] -> [b]
[a] -> [c]
[b] -> [d]
[c] -> [d]
[d] -> [e]
```

**Loop over a list** — `logic.foreach` runs the nodes after its `item` pin
once per element (`input.item`, or `$item` anywhere in the body), each element
on its own — its own skips, its own `$nodes` — and one finished before the
next starts. The body ends at the `logic.reduce` or `logic.collect` that
closes it, which runs **once**, after the last element: `logic.reduce` folds
what reached it into `input.reduce`, `logic.collect` lists it in
`input.collect.items`. An element whose branch was skipped is left out, and an
empty list closes too (`--initial`; `{ items: [], count: 0 }`). After the
close the payload is the one the foreach received plus the close's answer,
and the body's nodes are no longer in `$nodes`. An edge from the body to
anywhere but its close is refused:

```
[a] trigger.manual
[b] sekejap.query.run -- "SELECT amount FROM orders"
[c] logic.foreach --from "input.query.rows"
[d] logic.reduce --initial "{ total: 0 }" --step "{ total: $acc.total + $input.item.amount }"
[e] web.response.send --body "{{ input.reduce }}"
[a] -> [b]
[b] -> [c]
[c]:item -> [d]
[d] -> [e]
```

**Retry and poll** — `logic.retry` listens on a node's `:error` pin, or takes
a verdict on an ordinary edge, and sends the payload round again on `retry`
until its budget is spent (`failed`) or the verdict says done (`done`). The
`retry` edge back is the one edge that may point backwards: it does not count
as an input of the node it reaches, and delivering it runs that node again,
with everything after it. Any other cycle is refused when the pipeline is
activated, naming it:

```
[t] trigger.manual
[poll] http.response.fetch --url https://api.example.com/jobs/42 --method GET
[wait] logic.retry --max-attempts 40 --delay 5s --when "input.response.body.status !== 'done'"
[next] javascript.script.run -- "return { result: input.response.body.result }"
[gaveup] web.response.send --status 504 --body "{{ { error: 'job did not finish' } }}"
[t] -> [poll]
[poll] -> [wait]
[wait]:retry -> [poll]
[wait]:done -> [next]
[wait]:failed -> [gaveup]
```

**Failure.** A node that fails — running, timing out, or resolving its
`{{ }}` flags — delivers `{ input, error: { code, message } }` to its
`:error` pin when one is wired, its other edges are skipped, and the run goes
on; otherwise the run fails there. A failure an edge consumed is drawn orange
on the canvas with a count, never red.

**The end of a run.** A run ends when every node has answered or been
skipped. Its result is the answer of the last node with no outgoing edge
that ran, in text order. `web.response.send` answers the caller the moment it
runs; the nodes after it keep running, and a failure after it is recorded on
the run without changing what the caller received. The first response wins:
a second `web.response.send` in the same run sends nothing.

---

## Inputs

A trigger delivers one envelope: `body` (fields) and `files` (FileRefs). A
file that arrives is `lifecycle: temporary` — it lives for the run unless a
node such as `fs.file.put` keeps it. **Input nodes** (`input.text`,
`input.number`, `input.image`, … — the `input` family in the index) each
declare and check one field of that envelope and answer it under their
`--name`. The input nodes reachable from a trigger *are* its declaration:
the Studio builds the Run form from them, and an agent reads them to know
what to send. They work after `trigger.manual` and after `trigger.webhook`.

```
| trigger.manual
| input.text caption --label "Caption" --max 200
| input.image photo
| fs.image.thumbnail --from "{{ input.photo }}" --width 200 --height 200 --preview image
| javascript.script.run --preview json -- "return { caption: input.caption, thumb: input.image.ref }"
```

A missing required field refuses the run, naming the field. Over MCP,
`pipeline_execute` takes the envelope:
`input: { body: { caption: "x" }, files: { photo: "uploads/cat.png" } }` —
a `files` value is a store path that becomes the FileRef of that object.

---

## Webhooks: auth and streaming

```
| trigger.webhook --route /admin/posts --method POST --auth jwt --credential jwt_main --role admin --role editor
```

The trigger refuses an unauthenticated request before any node runs. The
envelope's shape (`input.webhook.body`, `.params`, `.query`, `.files`,
`.auth`) is in `help("pipeline/authoring")`; the recipe for sessions is
`help("pipeline/examples/cookie-jwt-auth")`.

Any webhook pipeline streams when the client asks: a request with
`Accept: text/event-stream` receives `event: signal` messages while nodes
run, `event: response` with what `web.response.send` answered the moment it
runs, then `event: done` with the result or `event: error`. A signal is
anything a node emits (`ai.text.generate` thinking and tool calls, a
script's `emit`) and the engine's own lifecycle: `run_start`; per node
`node_start` and one of `node_ok`, `node_empty` (ran, emitted nothing),
`node_fail`; `node_retry` or `node_error_routed` for a failure an `:error`
edge consumed; `node_skipped`, with no `node_start`, for a node on a branch
not taken; then `run_done`. Filter on `kind`. `POST /api/projects/{o}/{p}/pipelines/execute`
streams the same way with the same header and ends with `event: result`.

---

## Files

Bytes never travel inline. A file is a **FileRef** (`help("pipeline/authoring")`
shows one) — from an upload (`$trigger.files.<field>`), from
`http.response.fetch --parse bytes`, or from any `fs.*` node. Every `fs.*`
node names its file with `--from`: a FileRef, an upload or a store key.

```
| trigger.webhook --route /upload --method POST
| fs.file.put --from "{{ input.webhook.files.photo }}" --folder uploads --accept image --max-size 10MB
| fs.image.thumbnail --from "{{ input.file }}" --width 320 --height 320 --fit cover --format webp --folder thumbs
```

Store `input.file.ref` in a row — a store key, never a URL. No node answers a
URL and no node exposes a file: every object is private until the owner
exposes its folder in Studio → Files (`help("platform")`).

**Provider APIs over HTTP.** An outside API whose key must stay secret is
called with `http.response.fetch --credential <id>`, where the credential is
a Secure Request profile the owner created (method, URL, headers with the key
behind a placeholder). The pipeline keeps the body's shape; the credential
keeps the key, and the run record redacts it:

```
| trigger.manual
| input.text prompt --label "Describe the image"
| http.response.fetch --credential image_api --body "{{ [ { taskType: 'imageInference', positivePrompt: input.prompt, width: 1024, height: 1024 } ] }}"
| http.response.fetch --url "{{ input.response.body.data[0].imageURL }}" --parse bytes
| fs.file.put --from "{{ input.response.body }}" --folder generated --preview image
```

**Pictures from SVG.** `fs.image.render` draws an SVG — written by a model,
a script or a stored template — as PNG, JPG, WebP or PDF, no browser;
`fs.barcode.render` draws a QR or Code 128 code to place inside it;
`fs.image.chromakey` cuts a green screen out of a generated picture. Their
pages carry the details:

```
| trigger.manual
| input.text brief --label "What the poster is for"
| ai.text.generate --provider openrouter --credential openrouter_main --answer-only --schema '{"type":"object","required":["svg"],"properties":{"svg":{"type":"string"}}}' -- "Write one 1080x1350 SVG poster with font-family Inter for: {{ input.brief }}"
| fs.image.render --text "{{ input.text.data.svg }}" --folder posters --preview image
```

---

## Notes on the canvas

A note is a sticky note on the pipeline canvas: text for the next reader,
never executed, never reached by an edge. `note` is a reserved word in both
modes:

```
[t] trigger.webhook --route /signup --method POST
[m] mail.message.send --credential smtp_main --recipient "{{ input.webhook.body.email }}" --subject "Welcome" --text "Thanks for signing up."
[t] -> [m]
[why] note --text "Create the `smtp_main` credential before activating." --at 120,-80 --size 320x90 --color amber
```

In pipe mode a note is `| note --id why --text "…"`. Flags: `--text`
(markdown), `--at x,y`, `--size WxH`, `--color` (`amber` `blue` `green`
`rose` `violet` `teal` `orange` `slate`), `--id` (pipe mode; graph mode takes
the `[label]`); a `-- body` is the text too. `register` keeps every existing
note the body did not redeclare; `patch pipeline <path> note <id> …` changes
one and `--remove` deletes it.

## Previews under a node

`--preview <as>[:<path>][@WxH]` draws one value of the node's latest answer
under its box on the canvas; `--preview-in` draws its input. `as` is `image`
`video` `audio` `pdf` `json` `text` `table` `html`; `path` is a dot path
into the payload (`file`, `query.rows`), or left off for the first value
that fits; `@WxH` sets the panel size (160×60 to 1200×900); `off` removes
it. Presentation only — the engine never reads it, and a declared preview
records its value even at the default capture level.

```
| table.query.run --from "datasets/orders.csv as o" --limit 20 --preview table:query.rows -- "SELECT * FROM o"
```

---

## Commands

| Command | Effect |
|---|---|
| `check | …` or `check pipeline <path>` | what a save would refuse (unknown kind or flag, missing required flag, a word outside a closed choice, a duration or size without its unit, the flow rules) and, as warnings, keys no upstream node answers; saves nothing — `pipeline_check` over MCP |
| `register <path> [--title t] [--description d] [--as-json] | …` | save (or replace) the file as a draft, refused with every problem `check` names; `--as-json` prints the JSON and saves nothing |
| `activate pipeline <path>` | promote to live traffic (checks node config, node availability, libraries) |
| `deactivate pipeline <path>` | stop serving; the file stays |
| `execute pipeline <path> --input '{"k":"v"}'` | run the live version once with that payload |
| `run | trigger.function | javascript.script.run -- "return 1"` | run a body once, unsaved and unlogged; `run --dry-run` only parses |
| `patch pipeline <path> node <id> [--flag v]… [-- body]` | change one node; the pipeline is `stale` until activated |
| `patch pipeline <path> note <id> …` | create, change or `--remove` one canvas note |
| `get pipelines | nodes | connections | credentials | templates | docs` | list |
| `describe pipeline <path> [--compact]` | status, hash, hits, the DSL, and the node ids for `patch` |
| `describe connection <slug>`, `describe node <kind>`, `node help <kind>` | one resource |
| `git status | log --max-count=10 | diff | add <path> | commit -m "…"` | the project repository |

`<path>` is the `file_rel_path`; `.zf.json` may be omitted. There is no
`delete` verb (deletion goes through the pipelines API or Studio). Over MCP
the same verbs are `pipeline_register`, `pipeline_activate`,
`pipeline_deactivate`, `pipeline_execute`, `pipeline_run`, `pipeline_patch`,
`pipeline_list`, `pipeline_describe`, `pipeline_get`,
`pipeline_get_invocations`, `pipeline_search` and `git_command`.

---

## The node catalogue

Every kind, by family, from the live catalogue — `help("pipeline/nodes")`
for one line each, `help("pipeline/nodes/<kind>")` for one in full:

<!-- node-families -->

Recipes that use them end to end: `help("pipeline/examples")`.
