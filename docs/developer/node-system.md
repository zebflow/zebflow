# Node System

One node definition must serve every place that needs to understand a node.

## Definition First

A complete `NodeDefinition` includes:

- kind, title, and description
- static settings schema
- input and output schemas
- input and output pins
- UI category and fields
- DSL flags and body rules
- credential needs
- examples
- error and failure meaning
- AI tool metadata when the node can be used as a tool

The editor, validator, DSL, MCP help, function tools, and generated reference
must use this definition. Do not copy the same schema into a second hand written
list.

## Three Implementations

- Native nodes run built in Rust handlers.
- Composite nodes call a packaged function pipeline.
- WASM nodes call a packaged WebAssembly module.

All three must look like normal nodes to project authors. The implementation
kind matters for packaging, trust, and runtime limits, not for basic graph use.

## Installed Nodes

The node registry combines built in definitions and installed package
definitions. It must reject duplicate kinds, invalid schemas, missing files, bad
checksums, and unsupported package formats before a node becomes available.

## Related Source

- `src/pipeline/model.rs`
- `src/pipeline/nodes/mod.rs`
- `src/pipeline/nodes/basic/mod.rs`
- `src/pipeline/engines/basic.rs`
- `src/pipeline/engines/wasm_host.rs`
- `src/platform/services/node_registry.rs`
- `src/platform/help/mod.rs`
- `src/pipeline/nodes/bundled/`
