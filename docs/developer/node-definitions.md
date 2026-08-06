# Node Definitions

Node definitions are the source of truth for how nodes appear in the UI, DSL, validation, MCP tools, and generated docs.

Every node must define:

- kind
- title
- description
- inputs
- outputs
- config fields
- examples when the node is not obvious
- failure behavior when it matters

Native, composite, and WASM nodes should share the same user-facing definition shape wherever possible.
