# Native Nodes

A native node is a built in Rust node. Add one only when the capability belongs
in the core runtime or needs close access to a trusted platform service.

## Required Work

1. Create the node module under `src/pipeline/nodes/basic/`.
2. Define a typed settings struct with a JSON schema.
3. Return a complete `NodeDefinition`.
4. Implement `NodeHandler`.
5. Register the definition and handler in the built in node list and engine.
6. Add focused tests for validation, success, and failure.
7. Check generated help and the editor form.

## Design Rules

- Keep settings static and business input dynamic.
- Validate before doing external work.
- Return structured errors with stable codes.
- Resolve FileRef values through shared helpers.
- Keep credentials in the credential service.
- Use time, size, and count limits for expensive work.
- Clean partial output files after failure.
- Do not add a node for behavior already covered by a clear general node.

## Related Source

- `src/pipeline/nodes/interface.rs`
- `src/pipeline/nodes/basic/mod.rs`
- `src/pipeline/model.rs`
- `src/pipeline/engines/basic.rs`
- nearby node modules under `src/pipeline/nodes/basic/`
