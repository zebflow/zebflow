# InvocationRecord

Status: **review** — spec settled 2026-08-29; secret handling re-decided and the code caught up 2026-09-01; capture levels added and implemented 2026-09-10; rule 3 (a credential registered by value where it resolves) is contracted and not yet implemented — today it rests on rule 2 name matching. The entries under Open are open, not owed.

One row per pipeline run: when it ran, how long it took, whether it worked, and
what each node received and returned. This is a project's run history.

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
      "output": { "saved": { "path": "uploads/9f2c.jpg" } }
    }
  ]
}
```

| Field | Rule |
| --- | --- |
| `run_id` | stable identifier for one execution |
| `at` | unix seconds when the run started |
| `duration_ms` | wall clock for the whole run |
| `status` | `ok` or `error` |
| `trigger` | what started it: `webhook`, `manual`, `schedule` |
| `error` | present only when `status` is `error` |
| `trace` | one entry per node, in execution order |

A trace entry carries `node_id`, `node_kind`, the effective `config` after
expressions resolved, `duration_ms`, `input`, `output` (null on error), and
`error`.

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

Implemented 2026-09-10 for the levels themselves; rule 3 is still owed. Until it
lands, a secret is kept out of a record by rule 2's name list, which is why an
OAuth `code` was recorded in full — no list holds every name a third party
chooses. `on-error` narrows the exposure sharply in the meantime, because a run
that succeeds now records nothing at all.

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
| Newest N runs kept | project configuration, or per pipeline in its graph metadata | 20 |
| Maximum age | per pipeline in its graph metadata | none |

The per-pipeline setting wins over the project setting when both are present.

## Rejections

An empty `run_id`. A `status` other than `ok` or `error`. An `error` present on
a successful run, or absent from a failed one.

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
  row each time; whether writes are batched or sampled is undefined.
