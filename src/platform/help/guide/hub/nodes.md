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

Every installed node kind is `n.x.{package}.{rest}`, so a kind names the package
that provides it. Two bundles can never claim the same kind.

Composite and WASM are implementation worlds, not separate installer worlds. A
node declares where its code lives with one `run` binding:

- composite nodes use `run: { "function": "name" }`, a key in `functions`
- WASM nodes use `run: { "module": "name", "export": "symbol" }`, a key in `modules`

`export` is always required, so two WASM nodes in one bundle never resolve to the
same entry point. There is no `source` field: the implementation is derived from
the binding, so a bundle cannot claim an implementation that disagrees with the
artifact it points at. A node can therefore move between composite and WASM
without changing its kind, and saved pipelines keep working.

`trigger` declares a role and sits beside `run`, so a trigger may be implemented
either way. Lifecycle hooks are run bindings too.

Every source is normalized into Zebflow's canonical installed node format before
runtime indexing. Invalid package format must fail at install time.

The full rules are in `docs/contracts/kinds/node-bundle/README.md`.

Use the child node catalog entry for the live reference.
