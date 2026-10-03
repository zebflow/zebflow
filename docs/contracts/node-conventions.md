# Node Conventions

Status: **review** — decided 2026-10-03 from an inventory of every official
node under `src/pipeline/nodes/basic/`. Code caught up the same day for §1
destinations and sources, §3 (store pinning, FileRef `store`, no `url`), §4
and §5 on every file node (`src/pipeline/nodes/shared/project_store.rs`).
Still owed, tracked in the project ledger: §6 error codes, §7, and the
`-value` / `--input` cleanup outside the file nodes.

How an official node spells what it takes and what it answers.
[`NodeDefinition`](./kinds/node-definition/README.md) is the shape of a
definition and [`NodeIO`](./kinds/node-io/README.md) the wire between nodes;
this page is the grammar every official node uses inside them. A new node
adheres before it is registered.

## 1. One word, one meaning

Flags are kebab-case. A word means the same thing in every node, and a concept
has exactly one word.

| Concept | Flag | Meaning |
| --- | --- | --- |
| Where a written file goes | `--folder` | a store folder; every writing node has a default named after its noun |
| Its name | `--filename` | default: a uuid, or the source's name where the node has a source |
| The exact key | `--path` | a full store key; overrides `--folder` and `--filename` |
| Which store | `--store` | a store id of the project; see §3 |
| An existing target | `--on-conflict` | `overwrite`, `skip` or `error`; see §4 |
| A site's root | `--site-root` | site generators only; their `--path` is relative to it |
| A file the node reads | `--source-key` | a dot-path into the payload holding a FileRef or a store key |
| A source given directly | `--from` | a store key, or a FileRef through `{{ }}` (`fs.copy`, `fs.move`, `table.convert`, `geo.*`) |
| A secret the node uses | `--credential` | a credential id, never a value |
| A secret that checks a caller | `--auth-credential` | triggers only |
| Output encoding | `--format`, `--quality`, `--width`, `--height` | the same units everywhere |
| A switch | bare `--flag` | boolean; there is no `--no-flag` |

`--path` is a store key and nothing else: a dot-path into state or payload is
spelled `--key` (`--source-key`, `--out-key`). `--output`, `--output-path`
and `--output-dir` are retired, and so is `--to` as a destination;
`mail.send --to` stays, because there it names recipients.

## 2. Values

Every scalar flag takes a literal or a `{{ expression }}`, resolved as
[`NodeIO` §Value resolution](./kinds/node-io/README.md) says. There is no
second `-value` flag beside a literal one. A flag that must stay literal says
so in its definition and is refused if it holds `{{ }}`.

## 3. Files

```
node writes ──▶ project store (the one named by --store) ──▶ answers a FileRef
                key = normalise(expand(--path | --folder/--filename))
```

- Every read and write goes through the project's store service — never a
  path on this machine's disk. An engine that needs a path pulls into a run
  scratch folder and pushes back.
- **The store is explicit.** A node saved without `--store` is saved with the
  project's default store id at that moment; changing the default later moves
  no existing pipeline.
- Expansion happens first, then one shared normaliser: `..` and absolute keys
  are refused, before the store and its exposure rules decide.
- A node that writes one file answers a FileRef (with `store`,
  [`FileRef`](./kinds/file-ref/README.md)); a node that writes a tree takes
  `--folder` only and answers the folder plus a list of FileRefs.
- No node answers a `url`. Where a file can be reached is the owner's
  exposure decision ([`ZebFsAcl`](./kinds/zebfs-acl/README.md)), not a node's.
- A node never exposes anything.

## 4. Collisions

| Writes | Default `--on-conflict` |
| --- | --- |
| a named file (`--filename`, `--path`) | `error` |
| a tree (`--folder` of a tree node) | `error` when the folder is not empty |
| a site page (`web.static.generate`, `web.docs.generate`) | `overwrite` |
| a uuid name | — never collides |

## 5. Answers

A node adds one top-level key to the payload, named after what it produced
(`saved`, `thumbnail`, `compressed`, `audio`, `table`), and keeps the rest of
the payload. The key is fixed and documented in its definition.

## 6. Errors

`FW_NODE_<FAMILY>_<NODE>_<WHAT>`, upper snake case from the node kind
(`n.fs.image.thumbnail` → `FW_NODE_FS_IMAGE_THUMBNAIL_SOURCE`), registered in
the NodeIO error-code registry. A credential of the wrong kind is
`…_CREDENTIAL_KIND`, raised by one shared check.

## 7. Registration

A node is registered only when it is built. A placeholder is not a node.

## Evidence

Inventory 2026-10-03 (ledger: `zebflow › security › fs`): six names for a
destination, `--to` and `--input` with two meanings each, `-value` pairs on
two nodes only, `--on-conflict` on one node, error prefixes outside
`FW_NODE_` in nine families, and a registered placeholder (`n.concept`).
