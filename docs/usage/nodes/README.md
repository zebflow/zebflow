# Node Authoring

Users can add reusable node kinds without changing Zebflow's built in Rust
source.

Use a [composite node](./composite.md) when existing nodes can perform the work.
Use a [WASM node](./wasm.md) when the work needs a portable compiled module.

Both use the same node definition model. The visual editor, DSL, validation,
MCP, and generated help should see the same title, description, fields, pins,
schemas, examples, credentials, and errors.

One node package can contain one or many nodes. A package with one node is still
a normal node bundle with a one item node list.

Built in Rust node work belongs in the
[native node developer guide](../../developer/native-nodes.md).
