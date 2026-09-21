# NodeIO

Status: **review** — spec settled 2026-09-09 after design review; implementation
lands with this document. This is the wire format of the whole system — what
one node hands the engine, and how values move between nodes — so it copies
the discipline of the longest-lived wire format there is: a tiny fixed
envelope, an opaque body, one extension point, closed status classes, and an
engine that never reads the body.

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `NodeIO` |
| Spec | the output record, the payload rules, the value-resolution rule, and the scope names |
| Adapter | `src/pipeline/nodes/interface.rs` (record), `src/pipeline/error_class.rs` (registry), `src/pipeline/expr/` (resolution) |
| Written by | every node — native, composite, and WASM implement against this pair of shapes |
| Persisted twin | [`InvocationRecord`](../invocation-record/README.md) is this record's disk projection through the trace-capture budgets (`src/pipeline/trace_capture/README.md`) — one shape, two lifetimes |

## The record

```
NodeOutput
├── status      "ok" | "skip" | "refused" | "failed"     closed — four words, forever
├── pins        ["out"]                                  where the payload flows next
├── payload     { ... }                                  THE BODY — bare JSON, the author's, opaque to the engine
├── error?      { class, code, message }                 present exactly when status is refused/failed
├── meta        { "zf.cache": "hit" }                    flat string map — the ONE extension point
├── cost        { duration_ms, payload_bytes }           stamped by the engine, never by the node
└── trace       ["n.mail.send: delivered …"]             bounded lines; trace_capture governs the bound
```

The mapping that explains every choice: payload = HTTP body, status/error =
status line, meta = headers, cost = the access log, the `$` scope vars =
request context. HTTP's clarity comes from the body being bare and the
metadata riding outside it; so does this one's.

## Slot rules

| Slot | Rule |
| --- | --- |
| `status` | `ok`: output on pins. `skip`: ran and deliberately emitted nothing — a false `logic.if` branch, a match with no case; not an error, invisible to retry and weberror, grey in a run view (HTTP's 204, Comfy's bypass). `refused`: the caller's fault — bad config, bad input, wrong credential kind; retrying is useless (4xx). `failed`: the world's fault — relay down, timeout, disk full; retrying may help (5xx). No fifth word inside v1. |
| `pins` | non-empty on `ok`, empty on `skip`; names must exist in the node definition's `output_pins` |
| `payload` | one bare JSON object; see Payload rules |
| `error` | `class` repeats the status word; `code` comes from the registry; `message` is for a person and carries no secret |
| `meta` | string → string, flat. `zf.*` is the engine's; a node's own keys use its kind as prefix (`mail.relay_host`). Keys are append-only — a shipped key never changes meaning |
| `cost` | engine-stamped so profiling is trustworthy — a node that measures itself flatters itself. `duration_ms` always; `payload_bytes` when trace capture already sizes the body. Fields may be absent, never wrong |
| `trace` | for a person reading a run; never parsed by machines — anything a machine needs goes in `meta` or the payload |

**The engine reads the envelope only.** Routing, retention, retry, preview,
scheduling — every engine decision depends on slots outside `payload`. The
engine touches the body solely through the resolution rule below, acting on
the *next* node's behalf, and through the trace-capture projection, which
borrows and never mutates. This is the rule that keeps payload hand-off
zero-copy and makes future lazy serialization legal — Astra's capture engine
already respects this line; the contract makes it law.

## The error-code registry

Codes (`FW_NODE_MAIL_ADDRESS`, …) are a registry with IANA manners: each code
is declared once with its class, **append-only, never renamed, never reused**.
A code's class never changes — if a refusal turns out to be a system fault,
that is a new code, not an edit. `logic.retry` retries only `failed`;
`trigger.weberror` matches on class or code.

## Payload rules

- **Bare.** No wrapper: `$.rows`, never `$.result.rows`. An in-band envelope
  was considered and rejected — see Deliberately absent.
- **Named for what it holds.** `{ rows: [...] }`, `{ sent: true }`,
  `{ similarity: 0.43 }` — a scalar lives under its own name, not under a
  generic `result`. A node whose output name the caller should control
  exposes `--out-key`.
- **Reserved prefix.** Keys beginning `__zf_` belong to the platform
  (`__zf_type`, `__zf_response`). A node never invents one, never strips one
  it does not own.
- **Bytes cross as claim checks.** Binary content travels as a
  [`FileRef`](../file-ref/README.md) — a small, digest-verified reference; the
  bytes stay in storage and are redeemed at the storage boundary by the node
  that needs them. A FileRef is a *value*: nestable (`{ user: { avatar } }`),
  plural (`{ pages: [...] }`). There is deliberately no sibling binary
  channel. Inlining bytes is legal when explicit (`fs.get --encoding base64`).
- **Trigger context is initial payload.** `query`, `body`, `auth`, `params`,
  `files` arrive as ordinary keys, freely overwritable downstream — the
  originals are reachable forever via `$trigger`.
- **One envelope, whatever the trigger.** A trigger delivers `body` (fields)
  and `files` (FileRefs). A file that arrives at a trigger is
  `lifecycle: temporary` — it lives for the run and is deleted after, unless a
  node such as `fs.save` makes it durable. A manual run delivers the same
  envelope a webhook does. The `input.*` nodes are pass-through validators of
  one envelope field each: they change no value that was sent and fetch and
  store nothing; the one thing an input node writes is its `--default`, at
  `body.<name>`, when that field was not sent — so the Run form, `execute
  pipeline`, MCP and a webhook all leave the same envelope, and
  `input.body.<name>` agrees with `$nodes.<id>`. The set of them after a
  trigger is that trigger's declaration, and `$nodes.<id>` of one is the
  value it checked.
- **Manners are per family.** A producer (query, convert, generate) replaces
  the payload with its product; a reader (`kv.get`, `kv.exists`, `kv.incr`)
  merges into it; a doer (`kv.set`, `ws.emit`) passes it through or returns a
  receipt. Sibling nodes never differ in manner, and every node's
  `output_schema` states which it is.

## Value resolution — one mechanism

**Any config field: a literal, or `{{ expr }}`.**

- `{{ }}` runs engine-side on every string config value of every node before
  execution — which is why composite and WASM nodes inherit it for free: a
  node kind never implements resolution, it receives resolved config.
- Interpolated inside a string, the result is stringified into place
  (JSON-encoded for objects, plain for scalars). As the **whole** field, the
  expression's *typed* value replaces the string — objects, arrays, and
  numbers flow intact: `--params "{{ [input.user_id, 10] }}"`.
- Object literals are sanctioned: the evaluator's parenthesized wrap
  (`return (expr)`) is contract, not accident — `{{ { id: input.id } }}`
  is an object, never a block.
- Expressions are **synchronous**. Anything that waits — network, storage,
  time — is a script or a node.
- Program bodies are exempt and reach their own compiler untouched: config
  keys `markup`, `source`, `code`.
- Literal is the default because the common case must never break: a URL is
  a URL, not a division of undefined by undefined. `{{ }}` marks intent.
- The expression *is* the subject — not a dynamic twin — for
  `logic.if/match/foreach/reduce --expr` and `script`; those keep their
  form.

**Retired by this rule** (migration is its own loop, after this document):
the paired `--*-expr` twins (~30), every payload-extraction flag in all four
of its spellings (`-path` as pointer, `-path` as dot, `-from`, `-key`), the
`$.` literal-or-path convention, and `{name}` interpolation in
`ws.sync_state --path`. Each becomes the one mechanism:
`--value "{{ input.user.email }}"`.

## Scope — one set of names, two worlds

| Name | Meaning | Exists in |
| --- | --- | --- |
| `input` | the flowing payload | expressions, scripts |
| `$trigger` | the entry event, immutable, full trust (unfiltered auth) | expressions, scripts |
| `$nodes` | every finished node's payload, by node id | expressions, scripts |
| `$item` `$index` `$count` | iteration scope (`foreach`) | expressions, scripts |
| `$acc` | the accumulator (`reduce`) | reduce expressions |
| `$run` | `{ pipeline, request_id }` | expressions, scripts |
| `$placeholder` | the credentials a composite's manifest declared; exists only inside that composite's inner pipeline | expressions, scripts |
| `ctx` | **browser-safe projections only**: `ctx.auth` (public claims), `ctx.params`, `ctx.query`, `ctx.headers` | TSX templates only |

**The two-world rule — a security invariant wearing a naming convention:**
`$` names are server scope and never reach a browser; `ctx` is the filtered
browser context and exists only in templates. The prefix tells the reader the
trust level: `$nodes` in a TSX file is visibly wrong; `ctx` in a script is
too. Scripts drop their old `ctx.*` for the `$` names (the rename rides the
sandbox loop); TSX keeps `ctx` unchanged, deliberately different, because
`ctx.auth` is the *filtered* claim set and must stay so.

UI and DSL are one dialect: the same config key, the same text, the one
evaluator. A UI expression field is an editor of the same string the DSL
flag carries — the UI never grows its own syntax.

## Rejections

A fifth status word. A slot added to the record inside v1. An engine
decision that reads the payload. A node stamping its own `cost`. An error
code reused or reclassified. A `meta` key that changes meaning. A second
resolution mechanism. A secret in `error.message`, `meta`, or `trace`. A `$`
scope name reaching a browser.

## Version rules

Slots are never renamed or removed; a new slot is v2, and v2 carries the same
ceremony HTTP/2 did. Status words, error codes, and shipped `meta` keys are
append-only. Until first release, v1 may be amended in place with a dated row
below; after first release this contract freezes harder than
[`Pipeline`](../pipeline/README.md) — it is the wire format, and wire formats
do not move.

### Amendments

| Date | Change | Why it was safe |
| --- | --- | --- |
| 2026-09-21 | An `input.*` node writes its `--default` at `body.<name>` when the field was not sent. | The Run form already posted defaults; the DSL, MCP and webhook paths now leave the same envelope, and a sent value is never touched. |

## Enforcement

- The record struct carries exactly the seven slots; the completeness test
  refuses an eighth.
- Every registered error code declares a class exactly once; a duplicate or
  reclassified code fails the build.
- A flag resolving through anything but the one evaluator fails the contract
  test (grace: the retired mechanisms, until their migration loop lands).
- The invocation-record adapter round-trips a `NodeOutput` without loss
  beyond the declared capture budgets.

## Deliberately absent

- **An in-band envelope** (`{ result, binary, meta }` inside the payload).
  It re-imports n8n's `$json.` tax — every reference one level deeper forever
  — to solve problems this design solves out-of-band, and it forces every
  author to adjudicate "result or meta" at every node. HTTP itself keeps the
  body bare; that is where its clarity comes from.
- **A binary channel.** n8n's flat `binary` slot cannot nest or pluralize
  and hauls bytes between nodes; the FileRef claim-check does both and
  doesn't.
- **Node-supplied timing.** A node that measures itself is a node that
  flatters itself.
- **Node invocation from scripts.** The graph is the truth of what runs; a
  script computes between its pins. Reading `$nodes` (what already ran) is
  data; invoking is a declared door (`n.function_call`).

## Open

- Whether `cost` gains queue-wait time when the scheduler grows one.
- The exact `meta` keys `zf.*` ships with (cache, worker id) — settled by
  the first consumers, appended with dates.

## Request id across the boundary

A run's `run_id` (`kinds/invocation-record`) crosses HTTP under the name the
outside world knows, `X-Request-Id`:

- a `trigger.webhook` run answers with `X-Request-Id: <run_id>` on every
  response, success or failure, and adopts an inbound `X-Request-Id` as its
  `run_id` when a proxy sent one;
- every `http.request` a run makes carries `X-Request-Id: <run_id>` outward,
  whatever trigger started the run, so a failure two services away traces
  back;
- no other trigger has a caller to answer; the run keeps the same id in the
  log and nothing else happens.

Decided and implemented 2026-09-17: `webhook_run_id` adopts or mints, the ingress sets the header on every response, `http.request` forwards it unless the author set one.
