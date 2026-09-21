# InvocationRecord

Status: **review** — spec settled 2026-08-29; secret handling re-decided and the code caught up 2026-09-01; capture levels added and implemented 2026-09-10; rule 3 implemented 2026-09-10, by value and by position. Run id form, the request id and the error group decided and implemented 2026-09-17 for webhook runs (`tests/platform/smoke.rs` `an_uncaught_failure_hides_by_default_and_is_one_error_group`); schedule, function and WebSocket runs still write the invocation record only and are owed the group. The entries under Open are open, not owed.

Two records. The **invocation record**: one row per pipeline run — when it
ran, how long it took, whether it worked, what each node received and
returned; recent and complete, bounded by newest N. The **error group**: one
row per distinct failure — first and latest occurrence in full, a count, and
the id and time of every occurrence; durable, bounded by distinct errors, not
by how often one repeats. A thousand devices failing the same way cost one
group and a count, never a thousand payloads.

## Identity

| | |
| --- | --- |
| Representation | database record, not a document on disk — no envelope |
| Written by | the pipeline engine, once per execution |
| Read by | the pipeline's run history in Project Studio |
| Adapter | `PipelineInvocationEntry` and `NodeTraceEntry` |

## Shape

```json
{
  "run_id": "run-8f2c1a",
  "at": 1787175600,
  "duration_ms": 412,
  "status": "ok",
  "trigger": "webhook",
  "trace": [
    {
      "node_id": "save-photo",
      "node_kind": "n.fs.save",
      "config": { "folder": "uploads", "access": "private" },
      "duration_ms": 180,
      "input": { "photo": { "__zf_type": "file_ref", "…": "…" } },
      "output": { "saved": { "__zf_type": "file_ref", "ref": "uploads/9f2c.jpg", "…": "…" } }
    }
  ]
}
```

| Field | Rule |
| --- | --- |
| `run_id` | the identifier of one execution: 16 random bytes as 32 lowercase hex, opaque, never a timestamp or a counter. A webhook run **adopts** an inbound `X-Request-Id` when the proxy in front sent one, so the proxy's log and this one name the same run; otherwise the engine mints it. The same value is the run's `X-Request-Id` on the way out (see Request id) |
| `at` | unix seconds when the run started |
| `duration_ms` | wall clock for the whole run |
| `status` | `ok` or `error` |
| `trigger` | what started it: `webhook`, `manual`, `schedule` |
| `error` | present only when `status` is `error` |
| `trace` | one entry per node, in execution order |

A trace entry carries `node_id`, `node_kind`, the effective `config` after
expressions resolved, `duration_ms`, `input`, `output` (null on error),
`error`, and `status`: the node's NodeIO word (`ok`, `skip`, `refused`,
`failed`) or, since 2026-09-21, one of two engine words. `retry` is a wait:
a failure an `:error` edge handed to `logic.retry` (the entry keeps `error`
and `error_code` so the log says why), or a `logic.retry` that sent a
verdict round again (no error at all); the attempt is the count of that
node's `retry` entries so far. `error_routed` is a failure an `:error` edge
handed to anything else. Neither failed the run: the run's `status` is
unaffected, and an error group's key is the last entry whose failure nothing
consumed. The NodeIO record itself still has four words; these are the
record's account of what the engine did, not a fifth outcome a node can
produce.

An entry may carry `preview_snapshot`, since 2026-09-21. It is written when
the node declares `--preview image` (or `--preview-in image`) **and** the
value that preview would draw — the declared path, or the first image
FileRef the editor's own pick would find — is a **temporary** FileRef, whose
bytes are deleted with the run. The engine keeps a small copy beside the
payload, never inside it: `{ slot: "out" | "in", mime: "image/jpeg", width,
height, data_base64 }`, the longest side 540 px, JPEG quality 70, at most
64 KB; over that, `{ slot, snapshot_skipped: "too large" }` (or the reason a
read or decode failed). The declared `out` half is tried first, then `in`;
one snapshot per entry. A durable file gets none — the Studio reads it from
the store. The snapshot is written at every capture level that records the
entry's payload for that preview (a declared preview records it even at
`on-error`), and at `none` nothing is written. Writer:
`src/pipeline/trace_capture/preview_snapshot.rs`; reader: the canvas preview
(`preview-data.ts`, `snapshotCell`), which draws it from a data URI with the
caption "temporary — not saved". JPEG rather than WebP because the tree's
`image` crate encodes WebP lossless only.

## What must never appear

A trace is written to disk and shown in the UI, so it is a place secrets leak
into. Four rules, in order of authority:

1. **A secret belongs in a credential, and then it is never in the config.**
   A node reaches a secret through a `credential_id`; only that identifier is
   in the config, so only that identifier reaches the trace. `http_request`
   accepts a `secure_request` credential that supplies the whole request — URL,
   method, headers, body — so basic auth and query-param tokens go this way
   too, and `ws_trigger` has `auth_credential`. **No node forces a secret to be
   typed inline.**
2. **Name matching redacts config keys that name a secret.** `password`,
   `secret`, `api_key`, `access_token`, `token`, `authorization`,
   `private_key` and the rest are `••••••` in the trace. It exists for the node
   this tree does not own: a third-party node may hold a secret in a plain
   field with no credential behind it, and a name it shares with everyone
   else's is the only handle available.
3. **A credential's value is never shown, at any capture level.** Anything
   resolved from the credential store — including a refreshed token, which
   arrives by a different path than the first read — is registered as
   confidential where it is resolved, not by each node remembering. No setting,
   button or level reveals it. This is the floor under Capture Level below.
4. **Payload values are shown only at the capture level that asks for them**,
   then summarised so one large run cannot fill the disk.

**A secret typed into a free-text config field is not defended, and cannot be.**
Writing `http://user:hunter2@host/` into a `url`, or a password into an
`n.script` body, puts it in the run history in full. This is the same act as
pasting a password into a chat message: the mechanism that keeps it out —
credentials — was available and was bypassed. No redaction rule can tell a
secret from ordinary text inside a field whose whole purpose is free text, and
a rule that tried would have to redact script source, which would make the
history useless.

## Request id

A run id is the run's name inside; `X-Request-Id` is the same value when it
crosses HTTP, the header the outside world already knows. There is no second
identifier.

| Where | Rule |
| --- | --- |
| Every webhook response, success or failure | header `X-Request-Id: <run_id>` |
| The error page a hidden 5xx shows (`project-configuration`, `errors`) | the first eight characters of `run_id`, readable aloud; lookups accept a prefix |
| Every outbound `http.request` a run makes, whatever its trigger | header `X-Request-Id: <run_id>` forwarded, so a failure in another service traces back to the run that caused it |
| Schedule, function, WebSocket, KV triggers | the run has the same `run_id`; nothing to answer, so no header |
| The MCP endpoint | an agent's call is a request: the same header on its response |

The name says nothing about the framework on purpose; `x-zebflow-project` is
the operator's verification header (`addressing.md`) and is not sent to a
visitor for tracing.

## Error group

One per distinct failure per pipeline. The signature is `(file_rel_path,
node_id, error code, message with runs of digits and quoted values blanked)`:
`column "x" does not exist` and `column "y" does not exist` are two groups;
`row 4521` and `row 4522` are one. The node id and code keep the grouping
from swallowing distinct faults that share a phrasing.

```json
{
  "signature": "modules/events/pages/event.zf.json · n2 · FW_NODE_PG_QUERY · column \"?\" does not exist",
  "count": 1043,
  "first_seen": 1789430000,
  "last_seen": 1789516400,
  "reopened_at": null,
  "first": { "run_id": "…", "…": "the full invocation record" },
  "latest": { "run_id": "…", "…": "the full invocation record, replaced on every occurrence" },
  "occurrences": [ { "run_id": "…", "at": 1789516400 }, "… the newest N (run_id, at) pairs" ],
  "capture": "first-latest"
}
```

| Field | Rule |
| --- | --- |
| `first`, `latest` | full invocation records, held by the group even after the invocation log's newest-N has rolled them off; `latest` is replaced on every occurrence |
| `count`, `first_seen`, `last_seen` | every occurrence counts, none is sampled away |
| `occurrences` | `(run_id, at)` for the newest `occurrence_ring` occurrences (default 500). About forty bytes each: this is what lets a visitor's reference id resolve to its group after the full record is gone |
| `capture` | `first-latest` (default) or `all`: with `all` the group keeps the full record of every occurrence up to `capture_cap` (default 200), for the one error someone is chasing; flipped per group, or per pipeline for groups not yet seen |
| `reopened_at` | set when an occurrence arrives after the group was quiet for longer than `reopen_after` (default 7 days), so a bug fixed and reintroduced is not counted as the old one |

A run with `status: ok` never touches a group. The invocation record and the
group share `run_id`: a run links to its group; an occurrence links to its
record while the record is still within newest N, and says "rolled off; first
and latest kept" when it is not.

## Capture Level

How much of a payload is recorded is a **level**, not a security setting. A
pipeline on a device has no room for run history; a pipeline being debugged
wants all of it. One axis serves both, and not capturing is cheaper than
capturing and then hiding — the scan, sample and serialize work disappears
rather than its output.

| Level | Recorded |
| --- | --- |
| `none` | Node id, kind, pins, timing, status, error code. No config, input or output. |
| `on-error` | `none`, plus the payloads of nodes that failed. |
| `full` | Every node's config, input and output, subject to the budgets above. |

Default `on-error`: it costs what `none` costs on a run that succeeds, and a run
that fails is the one anybody opens. Set per project, overridable per pipeline,
and per node for the node being investigated. A pipeline-level control may raise
or lower every node at once.

**A level governs the record, never the run.** Raising or lowering it must not
change what the next node receives, what an expression evaluates, or what a
pipeline returns. A mechanism that alters execution data to shape a log has
broken the pipeline to protect the history of it.

**A payload becomes a record once**, through one projection, before it reaches
any sink. The sinks are the stored record, the HTTP API and Studio view,
invocation MCP, the live run stream, error messages — which routinely echo the
request that failed — and anything serialized into a model's context, including
agent tool results. The next node, `{{ }}` expressions and the pipeline's own
HTTP response are not sinks: they carry real values, because that is the
pipeline running.

Rule 3 holds underneath all of it. `full` on every node still shows no
credential value.

Implemented 2026-09-10. Rule 3 holds by value: a credential's values are
registered at the two places one is resolved — the store lookup and the OAuth
refresh, which arrives by a different path — and masked in every payload at
every level, with no node declaring anything. That is what covers the seven
credential-taking nodes of which one ever marked its secrets.

By position holds through `NodeDefinition::secret_paths`: a node kind declares
where a secret sits in its own output, addressed by JSON Pointer with `*`
matching one member or element. Only the record is masked — what the next node
receives is untouched. A path matching nothing changes nothing, because shapes
differ between API versions and a missing optional field is not an error.

What neither half reaches: a secret in a response under a name nobody declared
and that never came from the store. The pipeline author closes that by declaring
the path on the node instance — the kind cannot know what a particular API
returns. Instance-level declarations are contracted here and not yet built.

Added 2026-09-10, replacing a proposed masking mechanism. Masking asked node
authors to mark secrets, and name matching asked the platform to guess other
people's field names — a live OAuth callback stored its authorization code in
full because `code` is not a name anyone listed, and no list would have held it.
Levels make the lean case the default and leave only the credential floor as a
security rule.

Corrected 2026-09-01. An earlier draft made this a defect the platform owed a
fix for, and proposed a fourth mechanism — node-declared secret fields — to
close it. That mechanism would not have closed it: a node author would never
mark `source` or `url` secret, because both are ordinarily readable and are
what makes the history worth opening.

## Retention

History is bounded, never infinite.

| Bound | Where it is set | Default |
| --- | --- | --- |
| Newest N runs kept (`max_invocations`) | project configuration, or per pipeline in its graph metadata | 20 |
| Maximum age | per pipeline in its graph metadata | none |
| Distinct error groups kept (`max_error_groups`) | project configuration, or per pipeline | 200 |
| Occurrences remembered per group (`occurrence_ring`) | project configuration | 500 |
| Full records per group under `capture: all` (`capture_cap`) | the group, or the pipeline | 200 |

The per-pipeline setting wins over the project setting when both are present.
Repetition never spends these bounds: the invocation log costs
`max_invocations` payloads, the error log two payloads per distinct error
plus forty bytes per occurrence. A device fleet at a hundred thousand runs a
day with `max_invocations: 1` costs one row per pipeline and one write per
run; whether that write is batched at higher rates is the open item below.

## Rejections

An empty `run_id`, or one that is not 32 lowercase hex characters (an adopted
`X-Request-Id` that is not is replaced by a minted one and the inbound value
kept in the trace). A `status` other than `ok` or `error`. An `error` present
on a successful run, or absent from a failed one. An error group whose `first`
is missing.

## Open

- **Trace size.** Entries are summarised, but no maximum byte size per entry or
  per run is stated. `PipelineInvocationLogPipelineStats` already measures
  `trace_bytes` and `largest_trace_bytes`, so the measurement exists and the
  limit does not.
- **Deletion.** Whether a person may delete one run, or clear a pipeline's
  history, and whether that is recorded.
- **Who may read it.** A trace can contain personal data from a form
  submission. Nothing states which project roles may open the history.
- **High throughput.** A pipeline running hundreds of times a minute writes a
  row each time. Decided 2026-09-17: writes are never sampled — repetition is
  grouped (Error group) and counted. Whether the per-run write is batched
  above some rate is still undefined.
- **Who may read it** is no longer a quiet question: a reference id is printed
  on a public error page, so a role rule for opening the history that id
  names is owed before a project prints one.
