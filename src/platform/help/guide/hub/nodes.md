# Nodes in Hub

Nodes are a first-class learning surface because they define what a pipeline can do.

For now, the detailed node reference should remain automatic and generated from Rust definitions.

That means the node catalog stays aligned with the real runtime:

- kinds
- descriptions
- flags
- schemas

## Package Contract

All installable node sources use one source package shape:

```text
{package-slug}/
  definition.json
  icon.svg
  icons/*.svg
  functions/*.zf.json
  wasm/*.wasm
```

`definition.json` is used for one node or many nodes. A single node is simply a
package with `nodes.length == 1`.

Composite and WASM are implementation worlds, not separate installer worlds:

- composite nodes use `source: "composite"` and function pipeline artifacts
- WASM nodes use `source: "wasm"` and WASM module artifacts

Every source is normalized into Zebflow's canonical installed node format before
runtime indexing. Invalid package format must fail at install time.

Use the child node catalog entry for the live reference.
