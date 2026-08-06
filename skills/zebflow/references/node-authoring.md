# Node Authoring

Node definitions are the source of truth for UI, DSL, validation, tooling, and documentation.

Every node needs:

- kind
- title
- description
- config fields
- inputs
- outputs
- examples when useful
- failure semantics when useful

Node implementation families:

- native Rust nodes
- composite nodes built from pipelines
- WASM nodes installed as node bundles

Installed node behavior should feel consistent across families. The implementation mechanism should not force different authoring habits unless the runtime boundary requires it.

Large payload rule:

- small JSON can flow directly
- large JSON should become a file or reference
- files should pass as FileRef-style values
- tables and geospatial data should use table, file, or map nodes instead of giant inline arrays
