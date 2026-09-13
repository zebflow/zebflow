# Reference

Exact names live where they are generated or embedded, not in a second copy:

| Facts | Where |
|---|---|
| node kinds, flags, pins, schemas | `help(topic="pipeline/nodes")`, generated from the node definitions |
| MCP tools and parameters | `help(topic="platform/agent")`; the tool list itself comes from the server (`tools/list`) |
| HTTP routes and bodies | `help(topic="platform/api")`; the router in `src/platform/web/mod.rs` |
| the DSL | `help(topic="pipeline/dsl")` |
| package kinds and the release document | `docs/contracts/kinds/hub-package/` and `help(topic="guide/hub/how-it-works")` |
| stable data shapes (pipeline, project configuration, FileRef, node bundle, …) | [`docs/contracts/kinds/`](../contracts/kinds/) |

What remains here:

- [Error code families](./errors.md)
