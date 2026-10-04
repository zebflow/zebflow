# Node Conventions

Status: **decided for 0.11** — 2026-10-03, by the owner, from an inventory of
all 84 official nodes, a five-model usability survey and two adversarial
reviews of this text. The code catches up in 0.11 (ledger:
`zebflow › security › fs › zf-0-11-grammar`). From 0.11 the grammar is
**frozen**: a node may gain an optional flag, a dictionary word, a verb or a
field inside its answer, and nothing is ever renamed or removed. A redesign is
a new kind.

How an official node is named, what it takes, how it runs and what it answers.
[`NodeDefinition`](./kinds/node-definition/README.md) is the shape of a
definition and [`NodeIO`](./kinds/node-io/README.md) the wire between nodes;
the DSL's own syntax is `help("pipeline/dsl")`. Every node — official, hub,
third party — adheres before it is registered, and its definition is the one
source its help line, editor form and save-time checks are generated from.

## 1. Kinds

```
family.noun.verb      acting node    fs.image.thumbnail    ai.video.generate
trigger.<source>      entry node     trigger.webhook       trigger.process
input.<type>          run input      input.image           input.text
logic.<verb>          control node   logic.if              logic.foreach
```

- One lowercase word per segment, no prefix (`n.` is gone). The **noun** is
  what the node produces or acts on and is its **answer key** (§6); the
  **verb** says what happens to it. An entry node's answer key is its source
  (`trigger.webhook` → `webhook`); a run input's is its `--name`.
- The **family** is a domain or a brand from the closed list; only the owner
  adds one. **The swap test decides which:** if replacing the vendor leaves the
  rest of the pipeline meaningful — a video is a video, a completion is text —
  the task is one kind with `--provider` (§11). If it does not — a Telegram
  chat id means nothing to WhatsApp, a Postgres statement is not SQLite's — the
  platform's own concepts are the grammar and the **brand is the family**,
  spelled in full (`telegram`, `whatsapp`, `slack`, `discord`), as an engine
  keeps its own (`postgres`, `sqlite`, `sekejap`) and a language its own
  (`javascript.script.run`, `typescript.script.run`).
- **A family is the word a developer would search for.** A name of ours or an
  ambiguous abbreviation is spelled out (`mapserver`, not `ms`; `postgres`,
  not `pg`); a word developers already use as a word stays short (`fs`, `kv`,
  `ws`, `http`, `ai`).
- Brand families still speak the dictionary: the same nouns, verbs and words
  where the meaning is the same (`telegram.message.send` and
  `whatsapp.message.send` both take `--recipient` `--text` `--file` and answer
  `message: { id, recipient, sent_at }`); only the platform's own concepts are
  local words (`--keyboard`, `--template`). A brand's entry node is
  `trigger.<brand>`.
- **Same task, same answer — one kind.** A difference of format is a flag
  (`fs.barcode.render --symbology qr|code128`); a task only some providers of
  a swappable task offer is a new noun or verb (`ai.video.edit`).
- **Three origins, one grammar.** A *native* node (built into the engine) and
  an *official composite* (a bundle of function pipelines shipped with the
  platform, e.g. `telegram.*`) share these names: how a node is built is
  never in its name, so either can become the other without a rename; the
  catalog shows it as a badge. A *custom composite* — built by a user or
  installed from the hub — is `x.<package>.<noun>.<verb>` (the package's
  name in lowercase letters, digits and `_`; a `-` in a package name becomes
  `_`): the package owns
  every name under it, so no two packages and no official release can ever
  collide. Everything after `x.<package>.` follows this contract, its answer
  key is its noun, and the engine places the function's result under that
  noun itself.
- `logic.*` is closed: `if` `match` `foreach` `reduce` `collect` `retry`
  `concept` (a stand-in for a step not built yet; passes its input on).

| Families | |
| --- | --- |
| Now | `ai` `auth` `browser` `crypto` `fs` `function` `geo` `http` `input` `kv` `logic` `javascript` `mail` `mapserver` `mcp` `pipeline` `postgres` `sekejap` `sqlite` `table` `trigger` `typescript` `web` `ws` |
| Brands | `telegram` (now, from the curated bundle) · `whatsapp` `slack` `discord` (when built) |
| Reserved | `cloud` (provider-neutral resources: queue, function, bucket — `--provider aws\|gcp\|azure`) · `job` (external commands) · `sec` (scanning, detection) |

| Verbs | |
| --- | --- |
| Read | `get` `list` `head` `inspect` `fetch` `query` |
| Write | `put` `create` `update` `delete` `copy` `move` |
| Make | `generate` (a model's or engine's output) · `render` (deterministic drawing) · `convert` `extract` `encode` `decode` `thumbnail` |
| Talk | `send` `publish` |
| Act | `run` `call` `scan` `sign` `verify` `hash` `wait` `cancel` |

A verb two kinds share is in this list; a verb only one kind uses stays local
(`chromakey`, `increment`, `expire`, `publish` on `mapserver.layer`).

## 2. Flags

Kebab-case. A flag is a **role** (an input), a **setting** or a **switch**:

- **A role is a singular noun** naming what the value is for, never its origin
  or position (`--source-key`, `--input1` are gone). A repeatable role is
  repeated (`--image a --image b`) or given one `{{ [list] }}`.
- **`--from` or a typed role.** A node's subject of the same type as its noun
  is `--from` (`fs.image.thumbnail --from <image>`, `pipeline.process.get
  --from <process>`); an input of another type takes its typed role
  (`ai.video.generate --image <image>`).
- **A map is repeated `key=value`**, split at the first `=`
  (`--header "Accept=application/json"`).
- **A switch is a bare flag named for the behaviour it turns on**
  (`--write`, `--skip-optimize`); there is no `--no-x`.
- **No units, verbs or owner prefixes in a name** — units travel in the value
  (§3); the owner is the kind (`--name`, not `--tool-name`).
- `--path` is a store key and nothing else; no other flag ends in `-path`.
  `-expr`, `-value` pairs, `-key` pointers and `--output*` are retired. A
  config key has one name — no aliases.
- **Three presentation flags every kind takes** — `--title` (the label on the
  canvas), `--preview <as>[:<path>][@WxH]` (draw the node's answer under its
  box) and `--preview-in` (draw its input). They are stored beside
  `config.ui`, never read by the engine, and are the only flags not in a
  node's definition; a content title uses its own word (`--subject`,
  `--name`). `as` is closed: `image` `video` `audio` `pdf` `json` `text`
  `table` `html`; a ProcessRef previews its status until the outcome exists,
  then the outcome.

### The dictionary

A word two or more nodes share is here, with one meaning everywhere. A flag
only one node needs stays local and obeys the rules above; it joins the
dictionary the moment a second node needs the concept, and from then both use
it. Before adding a word: does a word here mean it? Can a word here plus a
closed choice say it (`--format json`, `--header Content-Type=…`)? Only then
add a singular, unit-free noun.

| Area | Words |
| --- | --- |
| Who | `--credential` (a secret's id, never a value) `--provider` `--model` `--name` `--description` `--label` (what a person reads) `--auth` `--role` `--function` (a function pipeline, by name) |
| Subject | `--from` (the subject, §2) `--id` (an existing object of this kind's noun: a message, record, layer) |
| Typed inputs | `--image` `--video` `--audio` `--file` (repeat) · `--text` (text content) `--html` · `--body` (what goes over a wire) · `--value` (what a store write holds) · `--argument` (k=v: what a callable receives) · `--default` |
| Generation | `--prompt` `--system-prompt` `--seed` `--duration` `--aspect` `--first-frame` `--last-frame` `--mask` `--reference` (repeat) `--schema` (a JSON schema) `--tool` (repeat) `--budget` `--option` (k=v) |
| Destination | `--folder` `--filename` `--path` `--store` `--on-conflict` (`error\|skip\|overwrite`) `--delete-source` `--recursive` |
| Shape | `--format` (what is produced) `--parse` (how input is read, else sniffed) `--encoding` (how bytes are written as text: `text\|base64\|hex`, each node listing the ones it takes) `--width` `--height` `--fit` `--quality` `--color` (`#rgb`/`#rrggbb`) `--rows` (also answer the rows as JSON) `--return` (`inline\|file\|process`) |
| Ceilings and time | `--timeout` `--ttl` `--delay` `--backoff` `--max-attempts` · `--max-size` `--max-length` `--max-items` (refusal ceilings) · `--limit` `--offset` (rows returned) · `--min` `--max` (numeric bounds) `--batch-size` · `--durable` (kept across restarts) `--batch` (applied on the next tick, not at once) `--optional` (may be absent) |
| Selection | `--query` (a statement in the target's own language) `--param` (k=v bindings; `1=` for `$1`) `--filter` `--field` (repeat) `--accept` (allowed file kinds) `--kind` (a node kind) `--layer` `--when` `--template` `--write` `--cron` `--algorithm` |
| Addressing | `--url` `--route` (a path this project serves) `--method` `--header` (k=v) `--status` `--room` `--connection` `--event` `--topic` |
| Messaging | `--recipient` (repeat: an address, a chat, a channel) `--sender` `--subject` |
| Records | `--key` `--table` `--record` `--edge` `--issuer` `--audience` `--claim` (k=v) `--context` (k=v kept with a process) |

Provider settings go through `--option key=value`, typed and closed in the
provider's profile (§11); a setting becomes a word only when a second provider
needs the same meaning.

## 3. Values

Every scalar flag takes a literal or a `{{ expression }}` (NodeIO §Value
resolution; JavaScript expressions, so `??` and `?.` work). A flag that must
stay literal — `--return`, `--provider`, `--write` — says so and refuses
`{{ }}`.

- **A choice is closed**: its words are in the definition; an unknown word is
  refused at save and at run.
- **Empty is not a value**: a needed value that resolves empty is refused.
- **Every source is explicit**: a node reads the payload only through a flag.
- **Every size has a ceiling**, declared and refused above it.
- **Units**: durations `ms` `s` `m` `h` `d` (`--timeout 30s`); sizes `B` `KB`
  `MB` `GB` (×1000) and `KiB` `MiB` `GiB` (×1024). One parser reads both
  everywhere.

## 4. Flow

The DSL already writes the flow: pipe mode chains nodes `n0, n1, …`; graph
mode labels nodes `[id]` and wires edges, `[a]:true -> [b]` for a pin; every
node has an `:error` pin. On top of that:

- **A node runs once, when every incoming edge has delivered or been
  skipped.** A node with several incoming edges waits for all of them — two
  branches meeting is a join, not two runs.
- **Skip propagates.** A node on a branch not taken is skipped; a node all of
  whose incoming edges are skipped is skipped. A reference to a skipped node
  is `null` — `{{ $nodes.big.text ?? $nodes.small.text }}` joins two branches.
- **`input` at a join** is the delivered payloads merged in DSL text order: a
  key from a later line wins. "Later" always means later in the text, never
  later in time. Every answer outside a loop stays at `$nodes.<id>.<key>`.
- **References** — `{{ $nodes.<id>.<key> }}`, `{{ input.<key> }}`,
  `{{ $trigger.<key> }}` — must point upstream; a reference to a node no edge
  path leads from is refused when the pipeline is checked or activated, so
  waiting never deadlocks. A path through a skipped node is `null`, never an
  error.
- **Loops**: the body of a `logic.foreach` is every node reachable from its
  `:item` edge before the first `logic.reduce` or `logic.collect` reached
  from it, which closes the loop. Each item runs the body as its own frame,
  in order; the close runs **once**, on the payload the foreach received,
  over the items that reached it (none for an empty or all-filtered list).
  After the close only its answer is visible. An edge into or out of the body
  other than through the foreach and the close is refused; a body node's
  `:error` handler is part of the body.
- **Re-entry**: an edge from a `logic.retry` pin back to a node that reaches
  the retry re-runs that node and everything after it in the same frame; it
  is the only cycle allowed — any other cycle is refused.
- **Failure**: a node that fails — while resolving its values, being built,
  running or timing out — delivers to its `:error` pin if one is wired;
  otherwise the run fails there.
- **A run ends** when every node has answered or been skipped. Its value is
  the answer of the last sink (a node with no outgoing edge) that ran, in DSL
  text order. A run record marks a node `skipped` (on a branch not taken) or
  `empty` (ran and emitted nothing). `web.response.send` answers the caller at
  once and later nodes keep running; a later failure is recorded on the run
  and does not change what the caller received; only the first response
  answers.

The definition declares every role — flag, type (`text` `json`
`file:image` …), `one` `repeat` or `map`, required, a ceiling on repeats. At
save: an unknown role, a missing required role, a second value for a `one`
role, a file of the wrong kind, a reference to a key no upstream node answers
("did you mean …") and an `input.<key>` two upstream nodes answer (naming both
`$nodes` paths) are refused, and an ordinary node with two incoming edges is
fine — it is a join.

## 5. Files

```
node writes ──▶ project store (the one named by --store) ──▶ answers a FileRef
                key = normalise(expand(--path | --folder/--filename))
```

- Every read and write goes through the project's store service; an engine
  that needs a path pulls into a run scratch folder and pushes back.
- **The store is explicit**: every node that declares `--store` — reader,
  writer or deleter — is saved with the project's default store at that
  moment.
- One shared normaliser runs after expansion: `..` and absolute keys are
  refused.
- A node writing one file answers a FileRef (with `store`); a node writing a
  tree answers `{ folder, items, count }`.
- **A writer takes the whole destination set**: every node that writes a file
  declares `--store`, `--folder`, `--filename`, `--path` and `--on-conflict`
  (a tree writer: `--store`, `--folder`, `--on-conflict`), with the same
  meaning and defaults rule everywhere.
- No node answers a `url` and no node exposes anything
  ([`ZebFsAcl`](./kinds/zebfs-acl/README.md)).
- A `read_only` store refuses every write itself and is never the default.

**One door.** Stored bytes are reached only through
`src/pipeline/nodes/shared/project_store.rs`. A FileRef is validated and read
from the store it names (one without `store` is refused); reads into memory
are capped (`MAX_NODE_OBJECT_BYTES`) and a received body is capped as it
arrives; no node joins a key onto a local path; repository files are read
through a reader that refuses links; an external program gets keys after `--`.

**Side effects.** A node deletes only what its run names — its own source
under `--delete-source`, once its output is written and is not that source; a
delete also forgets the object's exposure rule; a failed delete fails the node.

## 6. Answers

Every node adds **one key** to the payload — its noun (§1) — and keeps the
rest, through `with_answer`. Everything about the result nests inside it:

```
trigger.webhook        → webhook: { body, query, params, headers, files, method, path, auth }
fs.image.thumbnail     → image:   { …FileRef…, width, height, source_deleted }
postgres.query.run     → query:   { rows, columns, row_count }
kv.entry.get           → entry:   { key, value, found }
auth.token.create      → token:   { access_token, token_type, expires_in, profile }
fs.folder.list         → folder:  { path, items, count }
javascript.script.run  → script:  <what the code returned>
```

- A list answers `{ items, count }` (and `next` when `--offset` applies).
- An `:error` delivery answers the same key: `{ ok: false, error: { code, message } }`.
  A node with routed outcomes (`logic.if`, `crypto.password.verify`) states
  them in its answer (`password: { valid: true }`) as well as its pin.
- A control node passes its payload on; `logic.reduce` and `logic.collect`
  answer `reduce` and `collect`.

## 7. Long tasks and processes

Work that can outlast a run — a generated video, a cloud job, an external
command, a pipeline started in the background — runs as a **process** the
platform records. A [**ProcessRef**](./kinds/process-ref/README.md) is a
value like a FileRef: stored, passed, shown on a page, handed to another
pipeline.

```json
{ "__zf_type": "process_ref", "id": "prc_7c1e…", "kind": "ai.video.generate",
  "provider": "seedance", "name": "clip", "pipeline": "media/clip", "run": "run_91a…",
  "context": { "email": "ana@example.com" }, "status": "running",
  "started_at": "2026-10-03T08:00:00Z" }
```

- `status` is closed: `queued` `running` `done` `failed` `cancelled`. An
  outcome is kept 7 days, then the ref answers `expired`.
- A node whose work can outlast a run lists `process` among its `--return`
  words (literal, never `{{ }}`). With `--return process` it answers at once,
  its noun holding the ProcessRef, and `--name` and `--context` travel with it;
  otherwise it waits up to `--timeout` (default 60s, ceiling 1h) and answers
  the result, or fails with `…_TIMEOUT` naming the process still running.
- The finished process keeps the answer the node would have given, under the
  same noun (`video: FileRef`), or `{ ok: false, error }`.

```
pipeline.process.get     --from REF   → process: { status, result?, error? }   never blocks
pipeline.process.wait    --from REF   → process: { status, result | error }    up to --timeout
pipeline.process.cancel  --from REF   → process: { status: cancelled }
pipeline.process.list    [--kind K] [--name N] [--filter status=running] → process: { items, count }
pipeline.process.start   --route|--name PIPELINE --argument k=v                    (a pipeline in the background → process)
trigger.process          [--kind K] [--name N] [--pipeline P] → process: { …ref, status, result | error }
```

A run started by `trigger.process` never fires `trigger.process` for a process
it started itself.

## 8. Collisions

| Writes | Default `--on-conflict` |
| --- | --- |
| a named file (`--filename`, `--path`) | `error` |
| a tree | `error` when the folder is not empty |
| a site page (`web.site.generate`) | `overwrite` |
| a uuid name | never collides |

## 9. Errors

`FW_NODE_<FAMILY>_<NOUN>_<VERB>_<WHAT>` from the kind
(`fs.image.thumbnail` → `FW_NODE_FS_IMAGE_THUMBNAIL_SOURCE`), registered in
the NodeIO registry; a shared helper raises the calling node's code. `ZEBFS_*`
and `FW_FILE_REF_*` pass through because they name a contract.

## 10. Addresses

A node never holds the project's own address ([Addressing](./addressing.md)
§0). A site generator needing an absolute URL takes the `serve` origin of the
folder it writes; with none, it writes host-relative links and no sitemap.

## 11. Providers

For a swappable task (§1, the swap test) one kind serves every provider. Its definition holds the shared
roles and, per provider and model, a **profile**: which roles it accepts
(required, optional, how many), the allowed values of its choices
(`--duration 5s|10s`), and its own settings as typed, closed `--option` keys.
At save, a role the provider does not take, a value outside its list, an
unknown option, and a `--credential` whose provider disagrees with
`--provider` are refused, naming what it does take. `zeb help <kind>
--provider <name>` prints that provider's signature. The answer has one shape
for every provider. A profile is added or extended, never changed.

## 12. Enforcement

`src/pipeline/nodes/conventions.rs` holds the tests every node passes before
it is registered:

| Test | Checks, over every official node |
| --- | --- |
| `kinds_follow_the_shapes` | §1 shapes, family and verb lists, no prefix |
| `flags_follow_the_grammar` | singular roles, `--from` rule, no `--no-*`, no `-path` but `--path`, no units in names, every choice lists its words |
| `shared_words_mean_one_thing` | a flag two nodes share is in the dictionary with the same type and cardinality |
| `nodes_use_one_door` | no default store, no unbounded read, no `std::fs::read` outside the door |
| `answers_go_through_with_answer` | one key, the kind's noun, through the helper |
| `config_keys_have_one_name` | no `serde(alias)` |
| `codes_carry_the_node_family` | every raised code is the kind's or a pass-through, and registered |
| `store_nodes_declare_their_store` | every node that opens a store declares `--store`; every file writer declares the whole destination set |
| `composites_follow_the_grammar` | every shipped bundle's kinds, flags and answers pass the rows above; an installed bundle is checked the same way before it is registered, under `x.<package>.` |
| `providers_declare_profiles` | a kind with `--provider` has a profile per provider |
| `long_tasks_can_return_a_process` | a node waiting on outside work offers `--return process` |

## Evidence

Inventory 2026-10-03: 84 nodes, 181 distinct flags — six names for a source,
five for a format, three for a lifetime, units in eleven names, answers
replacing the payload in twelve families. A 77-finding review traced to six
causes. A five-model survey rated the 0.10 grammar 3–4/10 and this one 8–9/10
for a correct first pipeline; four of five chose named roles. Two reviews of
the draft (Opus, Fable) found the flow, join, loop, failure, correlation and
answer-collision gaps that §4, §6 and §7 now close.
