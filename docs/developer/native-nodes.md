# Native Nodes

A native node is a built in Rust node. Add one only when the capability belongs
in the core runtime or needs close access to a trusted platform service.

## Where a Node Lives

One folder per DSL family under `src/pipeline/nodes/basic/`, one file per
node, the path mirroring the kind: `n.fs.save` is `basic/fs/save.rs`,
`n.kv.get` is `basic/kv/get.rs`, `n.ms.publish` is `basic/ms/crud.rs` (one
file may carry several operations of one family). A family with one node and
no submodules keeps that node in its folder's `mod.rs` (`concept`, `crypto`,
`script`); a node with submodules is a folder (`image/render/`). Each family's
`mod.rs` declares its files and exposes `definitions()`, which
`basic/mod.rs → builtin_node_definitions()` reads. Helpers more than one
family uses live in `src/pipeline/nodes/shared/` (`file_ref.rs`, `util.rs`);
a helper one family uses lives in that family's folder. No `.rs` file sits
beside `basic/mod.rs`, and no folder there is anything but a family of the
catalogue — `every_node_file_lives_in_its_family_folder` in
`src/pipeline/nodes/mod.rs` fails otherwise.

## Required Work

1. Create the node file under `src/pipeline/nodes/basic/<family>/` (a new
   family is a new folder, declared in `basic/mod.rs`).
2. Define a typed settings struct with a JSON schema.
3. Return a complete `NodeDefinition`.
4. Implement `NodeHandler`.
5. Register the definition in the family's `definitions()` and the handler in
   the engine.
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
- `src/pipeline/nodes/shared/`
- `src/pipeline/model.rs`
- `src/pipeline/engines/basic.rs`
- nearby node modules under `src/pipeline/nodes/basic/<family>/`
