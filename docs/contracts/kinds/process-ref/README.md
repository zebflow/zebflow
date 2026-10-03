# ProcessRef

Status: **candidate** — decided 2026-10-03 by the owner as part of the 0.11
node grammar ([Node Conventions](../../node-conventions.md) §7); code lands in
0.11.

The note that travels between nodes, pages and pipelines for work that
outlasts a run: a generated video, a cloud job, an external command, a
pipeline started in the background. The work runs as a **process** the
platform records; this small object is what everything else passes around.
It is to a process what a [`FileRef`](../file-ref/README.md) is to stored
bytes.

## Identity

| | |
| --- | --- |
| Marker | `__zf_type: "process_ref"` |
| Representation | inline payload — an object inside node data, never a file of its own |
| No envelope | not a document: no `apiVersion`, `metadata` or `spec` |
| Record | the process itself is a row the platform keeps per project (`data/store`, beside the invocation records) |

## Shape

```json
{
  "__zf_type": "process_ref",
  "id": "prc_7c1e4b2a9d",
  "kind": "ai.video.generate",
  "provider": "seedance",
  "name": "clip",
  "pipeline": "media/clip",
  "run": "run_91a3f0",
  "context": { "email": "ana@example.com" },
  "status": "running",
  "started_at": "2026-10-03T08:00:00Z"
}
```

| Field | Rule |
| --- | --- |
| `__zf_type` | exactly `process_ref` |
| `id` | `prc_` + opaque id, unique per project; the only field a reader needs to find the process |
| `kind` | the node kind that started it |
| `provider` | the `--provider` it runs on, or `null` |
| `name` | the starting node's `--name`, or `null`; what `trigger.process --name` filters on |
| `pipeline`, `run` | the pipeline and run that started it |
| `context` | the starting node's `--context` map, returned untouched to whoever reads the outcome; small (≤ 4 KiB), never a secret |
| `status` | closed: `queued` `running` `done` `failed` `cancelled`, or `expired` once the outcome is gone |
| `started_at` | RFC 3339 |

A ProcessRef is a snapshot: `status` is what was true when it was written.
The live state is read with `pipeline.process.get`.

## The record and its outcome

When the process ends, the platform keeps its **outcome** with the record:
the answer the starting node would have given, under the same noun
(`video: FileRef`), or `{ ok: false, error: { code, message } }`. Outcomes are
kept 7 days, then the record answers `expired` and its outcome's temporary
files are removed; durable files it wrote stay where it wrote them.

## Who reads and writes it

| Writes | Reads |
| --- | --- |
| a node run with `--return process` | `pipeline.process.get` `.wait` `.cancel` `.list` (`--from` takes a ProcessRef or its `id`) |
| `pipeline.process.start` | `trigger.process`, which fires once when a process ends, with the ref, `status` and outcome |
| | pages and other pipelines, which may store it (`kv.entry.put`) and show it |

## Rules

- A ProcessRef names a process of its own project; a ref from another project
  is refused.
- Reading a process needs the same project role as running the pipeline that
  started it; cancelling it needs the role that may edit that pipeline.
- `context` is data, never instructions, and never holds a secret; the
  starting node refuses a value over the ceiling.
- A run started by `trigger.process` never fires `trigger.process` for a
  process it started itself.
- Additive only: a new field may be added; none is renamed or removed.

## Implementation

To be written in 0.11 (ledger: `zebflow › security › fs › zf-0-11-grammar`):
the record store, the `pipeline.process.*` nodes, `pipeline.process.start` and
`trigger.process`.
