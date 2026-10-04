# Node Authoring

The node's definition is the one source of its help line, its page
(`help(topic="pipeline/nodes/<kind>")`), its editor form and its save-time
checks. `docs/contracts/node-conventions.md` is the grammar every node passes
before it is registered; `src/pipeline/nodes/conventions.rs` enforces it.

Every node:

- is named `family.noun.verb` (native and official composite alike; a custom
  or Hub node `x.<package>.<noun>.<verb>`), its family from the closed list;
- answers **one key**, its noun, through
  `crate::pipeline::nodes::shared::util::with_answer`, and keeps the payload;
- declares every flag as a `DslFlag` with `value`, `choices` and
  `max_repeat` where they apply (`node_signature()` renders the help line
  from them); a role is a singular noun, the subject is `--from`, a map is
  repeated `key=value`, a switch is bare, units travel in values through
  `crate::pipeline::nodes::shared::units`, a choice is closed through
  `shared::limits::choice`;
- marks a password or key flag `secret: true`;
- reaches stored bytes only through `src/pipeline/nodes/shared/project_store.rs`;
  a file writer declares `--store --folder --filename --path --on-conflict`;
- raises `FW_NODE_<FAMILY>_<NOUN>_<VERB>_<WHAT>` codes, registered beside the
  node's other codes;
- has a description whose **first sentence** says what it does in one short
  line — the node index (`help(topic="pipeline/nodes")`) shows exactly that
  sentence — and whose rest says what it needs, what it answers and the usual
  mistake; plus at least one example whose DSL builds.

Implementation families: native Rust nodes (`src/pipeline/nodes/basic/`),
official composites (`src/pipeline/nodes/bundled/`), and node bundles
installed from the Hub. How a node is built is never in its name.

Large payloads: small JSON flows directly; large JSON becomes a file; files
pass as FileRefs; tables and spatial data use `table.*`, `geo.*`,
`mapserver.*`.
