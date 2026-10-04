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

Node kinds split by who curates them. Official nodes — native ones and the
composites shipped with the platform — have plain names, `family.noun.verb`
(`telegram.message.send`, `ai.embedding.generate`), and Zebflow guarantees they
are unique. Everything installed from the Hub is `x.{package}.{noun}.{verb}`, so
a kind names the package that provides it and two bundles can never claim the
same kind. Promoting a Hub package to official is therefore a rename.

A bundle node answers like a native one: the engine adds one key, the kind's
noun (`message`, `embedding`; a trigger's source, `telegram`), and keeps the
rest of the payload. A composite's answer is what its function's last node
answered, usually a `javascript.script.run` shaping the result. The function
receives the node's flags (`input.function.<config key>`), never the payload,
and the credential's secrets as `$placeholder` values.

The kind says nothing about how a node is built, and nothing about whether it is
currently installed. The first is the run binding's job, the second is answered
by the registry and `zeb.lock`.

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
