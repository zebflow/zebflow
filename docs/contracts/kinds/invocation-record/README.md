# InvocationRecord

Status: **review** — spec settled 2026-08-29; secret handling re-decided and the code caught up 2026-09-01. The entries under Open are open, not owed.

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
into. Three rules, in order of authority:

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
3. **Payload values are redacted where a node marks them**
   (`__zf_private_redact`), then summarised so one large run cannot fill the
   disk.

**A secret typed into a free-text config field is not defended, and cannot be.**
Writing `http://user:hunter2@host/` into a `url`, or a password into an
`n.script` body, puts it in the run history in full. This is the same act as
pasting a password into a chat message: the mechanism that keeps it out —
credentials — was available and was bypassed. No redaction rule can tell a
secret from ordinary text inside a field whose whole purpose is free text, and
a rule that tried would have to redact script source, which would make the
history useless.

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
