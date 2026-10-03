# Pipeline Authoring

Authoritative code and docs:

- `src/pipeline/model.rs`
- `src/pipeline/engines/basic.rs`
- `src/pipeline/nodes/basic/mod.rs`
- `src/pipeline/nodes/basic/**/*.rs`
- `src/platform/help/pipeline/index.md`
- `src/platform/help/pipeline/dsl.md`
- `src/platform/help/pipeline/authoring.md`

Contract boundary:

- `src/platform/help/pipeline/index.md`
- `src/platform/help/pipeline/dsl.md` (the DSL and `{{ }}` expressions)
- `help(topic="pipeline/nodes")` — generated from the node definitions
- `docs/contracts/project.md`
- `docs/contracts/versioning.md`

Pipeline formats:

| Format | Use when | Notes |
|---|---|---|
| Pipe DSL | Linear workflows | Starts with `|`; easiest for simple APIs/pages/jobs. |
| Graph DSL | Branching or named pins | Uses `[id] node` and `[a]:pin -> [b]`. |
| JSON graph | Exact storage and generated output | `.zf.json` files store nodes, edges, schemas, metadata. |

Canonical lifecycle:

1. Choose `file_rel_path`, relative to the source root (the repo root by default): `api/{name}.zf.json`, `pages/{name}.zf.json`, `jobs/{name}.zf.json`. No `pipelines/` prefix.
2. Write title and description.
3. Register the pipeline.
4. Activate when it should receive live traffic.
5. Test through `pipeline_execute`, webhook, function call, schedule, or MCP trigger.
6. Inspect invocation only when debugging or validating.

Required mental model:

- `input` is business payload moving along edges.
- `ctx` is run context and does not flow through edges.
- Triggers start runs.
- Nodes transform payloads.
- Edges decide what downstream nodes receive.
- Pins must match node definitions.

Common trigger families:

- `trigger.webhook`
- `trigger.function`
- `trigger.schedule`
- `trigger.manual`
- `trigger.room`
- `trigger.socket`
- `trigger.topic`
- `trigger.mcp`
- `trigger.error`

Common work nodes:

- `javascript.script.run`
- `typescript.script.run`
- `postgres.query.run`
- `sqlite.query.run`
- `sekejap.query.run`
- `sekejap.record.create`
- `http.response.fetch`
- `function.result.call`
- `logic.*`
- `kv.*`
- `fs.*`
- `table.*`
- `geo.*`
- `mapserver.*`
- `web.*`
- `ws.*`
- `ai.*`

Safety rules:

- Do not guess node flags. Read `help(topic="pipeline/nodes/{kind}")`.
- Do not use malformed case pins. `logic.match` output edges must use declared case pins.
- Do not carry huge arrays through `logic.foreach` unless the node is configured for item-only flow or the input is intentionally small.
- Use FileRef/files for upload and artifact movement.
- Use `fs.file.put` for every file write: `--from` an upload (checked by content, `--accept`), `--text`, or `--value` JSON; it answers `file`.
- Use `table.data.convert` or `table.query.run` for structured file data instead of hand-parsing large CSV/JSON in scripts.
