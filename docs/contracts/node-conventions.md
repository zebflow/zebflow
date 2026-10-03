# Node Conventions

Status: **review** — decided 2026-10-03 from an inventory of every official
node under `src/pipeline/nodes/basic/`. Code caught up the same day for §1–§6
on every file node (`src/pipeline/nodes/shared/project_store.rs`), and in a
second sweep for the rest: `ms.publish --route/--from`, `fs.put --source-key`,
`sekejap.insert --records-key/--edges-key/--record-key`, `crypto --value`
(no payload fallback), `fs.list` without `--prefix`, `ms.*` without `url`,
and one `with_answer` for §5. `function.call --input` stays: it supplies what
`trigger.function --input` declares.

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
| A switch | bare `--flag` | boolean; there is no `--no-flag`; a negative is named for what it does (`--skip-optimize`) |

`--path` is a store key and nothing else: a dot-path into state or payload is
spelled `--key` (`--source-key`, `--out-key`), and no other flag ends in
`-path`. A config key has one name — no aliases. `--output`, `--output-path`
and `--output-dir` are retired, and so is `--to` as a destination;
`mail.send --to` stays, because there it names recipients.

## 2. Values

Every scalar flag takes a literal or a `{{ expression }}`, resolved as
[`NodeIO` §Value resolution](./kinds/node-io/README.md) says. There is no
second `-value` flag beside a literal one. A flag that must stay literal says
so in its definition and is refused if it holds `{{ }}`.

- **A choice is closed.** A flag with a fixed set of words lists them in its
  definition, and an unknown word is refused — never mapped to a default
  word.
- **Empty is not a value.** A value the node needs that resolves empty is
  refused, never replaced by a default or hashed, written or sent as `""`.
- **No hidden payload reads.** A node reads the payload only where a flag
  points (`--source-key`); there is no fallback to a well-known payload
  field.
- **Every size has a ceiling.** A dimension, count or length a node produces
  (pixels, dpi, rows held inline, bytes generated) has a declared maximum and
  is refused above it.

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
- A store registered `read_only` (an `s3` credential with `access: read_only`)
  refuses every write itself (`ZEBFS_READ_ONLY`) and cannot be the project's
  default store; a node may read from it, never write.

**One door.** A node reaches stored bytes only through
`src/pipeline/nodes/shared/project_store.rs`: `open_store`, `open_source`,
capped reads, streaming pulls and pushes, owned deletes. In particular:

- A FileRef is validated and read from the store it names; a FileRef without
  `store` is refused, never read from the default store.
- Every node that takes `--store` — readers and deleters as well as writers —
  is pinned at registration.
- A read into memory is capped (`MAX_NODE_OBJECT_BYTES`); larger work streams
  through a scratch file. A received body (an upload, an HTTP response) is
  capped while it arrives, not after.
- A node never joins a key or a name onto a local path itself. Repository
  files are read through the repository's own reader, which refuses links.
- An external program receives store keys after `--`, never as options.

**Side effects.** A node deletes only what its run names: its own source
under `--delete-source`, and only once its output is written and is not that
source. A delete also forgets the object's exposure rule. A failed delete
fails the node; it is never logged and ignored.

## 4. Collisions

| Writes | Default `--on-conflict` |
| --- | --- |
| a named file (`--filename`, `--path`) | `error` |
| a tree (`--folder` of a tree node) | `error` when the folder is not empty |
| a site page (`web.static.generate`, `web.docs.generate`) | `overwrite` |
| a uuid name | — never collides |

## 5. Answers

A node adds one top-level key to the payload, named after what it produced
(`saved`, `thumbnail`, `compressed`, `audio`, `table`, `rows`), and keeps the
rest of the payload — through the one helper, `with_answer`. The key is fixed
and documented in its definition. Details of the same result nest under it
(`image.layout`, `audio.word_timings`). A node never removes another key; a
deleted source is recorded in the answer (`source_deleted: true`).

Outside this rule, by name:

- a **router** (`logic.*`) passes its payload through unchanged on the pin it
  chooses;
- a **terminal** node (`web.response`) answers the response envelope that
  ends the run;
- a **transform** (`script`) answers exactly what its code returns — that is
  its job.

Every other node merges. Seven answers predate the one-key rule and keep
their top-level names until the owner decides their key (ledger:
`zf-answer-keys`): `pg.query` and `sqlite.query` (`rows`), `sqlite.mutate`
(`affected_rows`), `sekejap.query` (`rows`, `columns`, `row_count`, …),
`sekejap.insert` (`inserted_records`, …), `auth.token.create`
(`access_token`, `token_type`, `expires_in`, `profile`), `ai.agent`
(`response`, `data`, …). They merge like the rest; no new node joins the
list.

## 6. Errors

`FW_NODE_<FAMILY>_<NODE>_<WHAT>`, upper snake case from the node kind
(`n.fs.image.thumbnail` → `FW_NODE_FS_IMAGE_THUMBNAIL_SOURCE`), registered in
the NodeIO error-code registry. A credential of the wrong kind is
`…_CREDENTIAL_KIND`, raised by one shared check. A shared helper raises the
calling node's code. Two families pass through unchanged because they name a
contract, not a node: `ZEBFS_*` (the store's own refusal) and `FW_FILE_REF_*`
(an invalid FileRef).

## 7. Addresses

A node never holds or writes the project's own address
([Addressing](./addressing.md) §0). A site generator that needs an absolute
URL (a sitemap, a canonical link) takes it from the `serve` origin of the
folder it writes ([`ZebFsAcl`](./kinds/zebfs-acl/README.md)); with none, it
writes host-relative links and no sitemap.

## 8. Registration

A node is registered only when it is built. `n.concept` is built: it is the
deliberate stand-in for a step a pipeline describes but does not do yet, and it
passes its input through untouched.

## 9. Enforcement

A rule that is not tested is a wish. `src/pipeline/nodes/conventions.rs` holds
the tests, and a new node passes them before it is registered:

| Test | Checks, over every official node |
| --- | --- |
| `flags_follow_the_grammar` | no `--no-*`, no `-path` but `--path`, no `url` in an output schema, every choice flag lists its words |
| `nodes_use_one_door` | no `open_files()`, no store `get(`, no `std::fs::read` outside `project_store.rs` and named engine adapters |
| `answers_go_through_with_answer` | no hand-built `payload:` outside routers and terminals |
| `config_keys_have_one_name` | no `serde(alias)` in node configs |
| `codes_carry_the_node_family` | every raised code is `FW_NODE_<FAMILY>_…`, `ZEBFS_*` or `FW_FILE_REF_*`, and registered |

## Evidence

Inventory 2026-10-03 (ledger: `zebflow › security › fs`): six names for a
destination, `--to` and `--input` with two meanings each, `-value` pairs on
two nodes only, `--on-conflict` on one node, and error prefixes outside
`FW_NODE_` in nine families. Review 2026-10-03 by two independent models
(77 findings) traced to six causes: five roads to the store, 36 hand-built
payloads, 23 open choice flags, no limit or side-effect rule, a hand-kept
pinning list, and no enforcement — §2, §3, §5 and §9 answer them.
