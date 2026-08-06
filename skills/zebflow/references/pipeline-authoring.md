# Pipeline Authoring

Authoritative code and docs:

- `src/pipeline/model.rs`
- `src/pipeline/engines/basic.rs`
- `src/pipeline/nodes/basic/mod.rs`
- `src/pipeline/nodes/basic/**/*.rs`
- `src/platform/help/pipeline/index.md`
- `src/platform/help/pipeline/dsl.md`
- `src/platform/help/pipeline/authoring.md`

User-facing docs:

- `docs/usage/pipelines.md`
- `docs/usage/pipeline-language.md`
- `docs/usage/pipeline-examples.md`

Pipeline formats:

| Format | Use when | Notes |
|---|---|---|
| Pipe DSL | Linear workflows | Starts with `|`; easiest for simple APIs/pages/jobs. |
| Graph DSL | Branching or named pins | Uses `[id] node` and `[a]:pin -> [b]`. |
| JSON graph | Exact storage and generated output | `.zf.json` files store nodes, edges, schemas, metadata. |

Canonical lifecycle:

1. Choose `file_rel_path`, usually `pipelines/{area}/{name}.zf.json`.
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

- `n.trigger.webhook`
- `n.trigger.function`
- `n.trigger.schedule`
- `n.trigger.manual`
- `n.trigger.ws`
- `n.trigger.ws.client`
- `n.trigger.kv.subscribe`
- `n.trigger.mcp`
- `n.trigger.weberror`

Common work nodes:

- `n.script`
- `n.pg.query`
- `n.sqlite.query`
- `n.sqlite.mutate`
- `n.sekejap.query`
- `n.sekejap.insert`
- `n.http.request`
- `n.function.call`
- `n.logic.*`
- `n.kv.*`
- `n.fs.*`
- `n.table.*`
- `n.geo.*`
- `n.ms.*`
- `n.web.*`
- `n.ws.*`
- `n.ai.*`

Safety rules:

- Do not guess node flags. Read `help(topic="pipeline/nodes/{kind}")`.
- Do not use malformed case pins. `n.logic.match` output edges must use declared case pins.
- Do not carry huge arrays through `n.logic.foreach` unless the node is configured for item-only flow or the input is intentionally small.
- Use FileRef/files for upload and artifact movement.
- Use `n.fs.put` for content writes and `n.fs.save` when validating/promoting uploads.
- Use `n.table.convert` or `n.table.query` for structured file data instead of hand-parsing large CSV/JSON in scripts.
