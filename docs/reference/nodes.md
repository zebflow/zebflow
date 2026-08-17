# Node Reference

The live node reference is generated from `NodeDefinition` values. This keeps
the editor, validation, DSL, MCP, and documentation aligned.

Use the embedded help topic `pipeline/nodes` for the full catalog. Use
`pipeline/nodes/{kind}` for one node, such as `pipeline/nodes/n.fs.put`.

## Current Families

- `n.trigger.*` starts webhook, function, schedule, manual, MCP, KV, or
  WebSocket runs.
- `n.logic.*` branches, matches, collects, loops, reduces, and retries.
- `n.sekejap.*` queries and performs high volume inserts for Sekejap.
- `n.pg.*` queries PostgreSQL.
- `n.sqlite.*` queries and changes SQLite.
- `n.kv.*` works with short lived state and publish events.
- `n.fs.*` works with project files.
- `n.table.*` converts and queries tables.
- `n.geo.*` inspects and converts spatial data.
- `n.ms.*` publishes and manages map layers.
- `n.web.*` returns web responses and creates static or documentation output.
- `n.http.*` makes controlled outbound HTTP requests.
- `n.ws.*` sends, receives, and syncs WebSocket state.
- `n.ai.*` runs agent and media intelligence work.
- `n.function.*` calls a declared function pipeline.
- `n.script` runs a sandboxed script transformation.
- `n.crypto` provides cryptographic helper operations.
- `n.x.*` identifies installed nodes. The segment after `n.x.` is the package
  that owns the kind, and the implementation is not encoded in the name.

Every catalog item must include its kind, title, description, settings schema,
input and output schemas, pins, DSL fields, UI fields, and tool use metadata.

Related source:

- `src/pipeline/model.rs`
- `src/pipeline/nodes/basic/mod.rs`
- `src/platform/services/node_registry.rs`
- `src/platform/help/mod.rs`
