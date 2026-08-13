# Pipeline Runtime

The pipeline runtime validates a graph, resolves expressions, executes nodes,
and sends outputs through edges.

## Stored Graph

`PipelineGraph`, `PipelineNode`, and `PipelineEdge` are the saved graph model.
`NodeDefinition` describes one node kind. `PipelineContext` and
`NodeExecutionInput` describe one run.

Do not mix these levels. A node instance stores settings for one graph. A node
definition states what every instance of that kind accepts.

## Run Flow

1. Load the active graph.
2. Validate node kinds, settings, pins, and edges.
3. Build the run context.
4. Start at entry nodes.
5. Resolve expressions for the current node.
6. Execute the node handler.
7. Route each output through matching edges.
8. Return the final result and bounded trace data.

## Payload Ownership

Normal JSON values use owned `serde_json::Value` trees. A clone can copy a large
tree. Fan out can multiply that cost. The runtime must avoid copying the full
parent payload into every item when only the item is needed.

Tracing is separate from data flow. Turning traces off reduces log work, but it
does not remove payload copies caused by graph execution. Both paths need their
own limits.

## Function and WASM Calls

A function call should pass only declared input and return only declared output.
A WASM call should use the same public node contract. Large inputs and outputs
should use stable handles instead of repeated JSON copies.

## Related Source

- `src/pipeline/model.rs`
- `src/pipeline/interface.rs`
- `src/pipeline/engines/basic.rs`
- `src/pipeline/engines/wasm_host.rs`
- `src/pipeline/expr/`
- `src/pipeline/nodes/interface.rs`
- `src/pipeline/security.rs`
- `src/pipeline/prototypes/visual.rs`
- `src/platform/services/pipeline_runtime.rs`
