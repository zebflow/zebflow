# Pipeline Authoring

Authoritative code and docs:

- `docs/contracts/node-conventions.md` — the frozen node grammar (kinds, flags, values, answers)
- `src/pipeline/model.rs` — `NodeDefinition`, `DslFlag`
- `src/platform/shell/parser.rs` — the DSL parser
- `src/pipeline/engines/basic.rs` — how a run moves the payload
- `src/pipeline/nodes/basic/**/*.rs` — native nodes; `src/pipeline/nodes/bundled/` — official composites
- `src/platform/help/pipeline/dsl.md` — the grammar as agents read it
- `help(topic="pipeline/nodes")` and `help(topic="pipeline/nodes/<kind>")` — generated from the definitions

## The grammar

- A node is written as its kind: `family.noun.verb` (`fs.file.put`,
  `postgres.query.run`, `mapserver.layer.publish`, `javascript.script.run`,
  `telegram.message.send`). Entry nodes are `trigger.<source>`, run inputs
  `input.<type>`, control `logic.<verb>`; a custom or Hub node is
  `x.<package>.<noun>.<verb>`. There is no prefix and no alias.
- Every node adds **one key** to the payload — its noun — through
  `with_answer`, and keeps the rest: `input.webhook.body`, `input.query.rows`,
  `input.file`, `input.script`, `input.token.access_token`. A trigger answers
  under its source; `$trigger` is that envelope for the whole run.
- `--from` names the subject of the node's own type; other inputs are typed
  roles from the dictionary (`--text`, `--image`, `--file`, `--body`,
  `--value`, `--argument`, …). Maps are repeated `key=value` (`--param 1=…`,
  `--header "K=V"`), switches bare (`--write`), units travel in values
  (`--timeout 30s`, `--max-size 10MB`), choices are closed.
- Headers are sent exactly as written. Statement text never carries values;
  `--param` binds them and `--write` allows a change.
- Swappable vendors are one kind with `--provider`; each provider's signature
  is printed on the node's page.

## Formats

| Format | Use when | Notes |
|---|---|---|
| Pipe DSL | Linear workflows | Starts with `|`. |
| Graph DSL | Branching, named pins, joins, loops | `[id] kind …` and `[a]:pin -> [b]`. |
| JSON graph | Exact storage | `.zf.json` files store nodes, edges, notes, metadata. |

## Lifecycle

1. Choose `file_rel_path`, relative to the source root: `api/{name}`,
   `pages/{name}`, `jobs/{name}` (or the project's own layout). No
   `pipelines/` prefix.
2. Register the DSL (`pipeline_register`) — a draft.
3. Activate (`pipeline_activate`) when it should receive traffic.
4. Prove it through `route_fetch`, `pipeline_execute` or `pipeline_run`, and
   `pipeline_get_invocations`.

## Rules

- Never guess a node's flags or answer: read `help(topic="pipeline/nodes/<kind>")`.
- Hand-written docs, skills and MCP texts carry no per-node flag tables or
  answer shapes. They teach the grammar and point to the node's page, or
  embed `<!-- node-flags:<kind> -->`, which renders the table from the
  definition.
- Every DSL example in the help, the skills, these references and the MCP
  texts is parsed against the live catalogue by
  `cargo test --lib guide_lint` — an unknown kind, an undeclared flag, a
  choice word outside the list or a retired `input.<key>` fails it.
- Files move as FileRefs; tables and spatial data go through `table.*`,
  `geo.*`, `mapserver.*` rather than huge inline arrays.
