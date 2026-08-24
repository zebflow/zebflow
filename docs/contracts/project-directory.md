# Project directory

Status: **review, with evidence**. The reclassification below is applied to
the code: `ProjectFileLayout` carries the four `data/` tiers, every writer
named in §5 has moved, and a project built before this change migrates its
`data/sekejap`, `data/local.db`, and `data/runtime` transparently the first
time each is touched — verified live against a seeded pre-tier project, with
before/after content identical and a second run a no-op (§7). `data/nodes/`
is the one real path this document still does not classify; see §7. Not yet
**candidate**: a directory-tier policy has no document envelope of its own to
version, so most of the stability-matrix freeze checklist (versioned root,
canonical writer, negative tests over malformed bytes) does not apply to it in
the usual sense, and that gap itself needs a decision before this row can be
declared frozen. Stability-matrix row 2.

This is a contract, not a kind. It defines no document format. It defines
**which paths under a project are durable, which are generated and disposable,
and which are user data** — so a backup, a "clear cache" action, and a transfer
can each answer their question by which directory something is in, without
reading source.

## 1. Why this exists

`data/` is documented today as "what the machine derived," which reads as
disposable. It is not: `data/sekejap` is the project's database. A directory
whose own name implies "safe to delete" holding the one thing that is not safe
to delete is the defect this document closes.

Two migrations this week (`zebflow.json`→`.yaml`, pipeline identity) each wrote
a `.pre-*.json` recovery copy into `repo/`. `repo/` is git-tracked and portable,
so a recovery file — machine-generated, not authored — is committed and travels
to every clone that pulls the project.

Neither is a design mistake. `repo/` / `data/` / `files/` was decided correctly.
This finishes it.

## 2. The tree

```
<project>/
├── repo/                  AUTHORED — git-tracked, portable, back this up
│   ├── .git
│   ├── zebflow.yaml       the project's declared layout, identity, config
│   ├── zeb.lock           dependency lock
│   ├── docs/
│   ├── schemas/
│   └── {spec.layout.source}/
│       ├── *.zf.json
│       ├── pages/*.tsx
│       ├── styles/*.css
│       └── assets/
│
├── data/
│   ├── store/             DURABLE, GENERATED — irreplaceable, back this up
│   │   ├── sekejap/       the project's database: WAL, indexes, snapshot
│   │   └── local.db
│   │
│   ├── cache/              DISPOSABLE — delete anytime, rebuilds from repo/
│   │   ├── pipelines/      materialized pipeline runtime
│   │   └── agent_docs/
│   │
│   ├── recovery/           DISPOSABLE, BOUNDED — migration safety copies
│   └── logs/                DISPOSABLE, BOUNDED — invocation records, traces
│
└── files/                  AUTHORED, USER DATA — one flat object namespace
    ├── {any path an app chose}
    ├── {any path an app chose}
    └── .zebfs/
        └── acl.json         ZebFsAclManifest: path → {Private|PublicRead} × {Object|Prefix}
```

## 3. The four tiers

| Tier | Rule | Directories |
| --- | --- | --- |
| **authored** | what a person wrote. Never auto-deleted, never auto-generated, back it up. | `repo/`, `files/` |
| **store** | what the machine wrote and cannot regenerate. Losing it loses user data. Back it up. | `data/store/` |
| **cache** | what the machine wrote and can regenerate from `repo/`. Safe to delete at any time; nothing reads a deletion as data loss. | `data/cache/` |
| **bounded** | what the machine wrote, disposable, kept only for a retention window. Not backed up; not silently deleted before its window either. | `data/recovery/`, `data/logs/` |

Four words. A tool that needs to know whether a path is safe to delete asks
which tier it is in, not what the path is named or what code wrote it.

## 4. `files/` has no physical public/private split

Checked against the code, not assumed: `ZebFsAclManifest` (`src/zebfs/acl.rs`)
is a map from path to `{ZebFsAccess::Private | PublicRead} × {ZebFsAclScope::Object
| Prefix}`, layered over one flat object namespace. There is no storage-layer
directory that is inherently public. A path named `public/…` is a convention an
application chose when it called `put`; the manifest in `.zebfs/acl.json` is
what actually governs who can read it.

This document previously drew `files/public/` and `files/private/` as though
the storage layer enforced the split. It does not, and this section exists so
that mistake is not repeated.

## 5. What changed, concretely

- `repo/*.pre-*.json` recovery files (the JSON-to-YAML config migration, the
  pre-v1 lock migration, and the pipeline-identity migration) now write to
  `data/recovery/`, named with the date the migration ran
  (`zebflow-config-2026-08-24.json`, `zeb-lock-2026-08-24.lock`,
  `pipeline-identity-2026-08-24.json`), so `repo/` never carries a
  machine-generated file again. Directory creation is deferred to the moment
  of writing, so a refused migration leaves no trace, not even an empty
  `data/recovery/`.
- `data/sekejap` moved under `data/store/sekejap` (`src/platform/sekejap.rs`,
  `project_dir()`). `local.db` moved under `data/store/local.db`
  (`src/platform/adapters/project_data/mod.rs`,
  `src/platform/sqlite_schema.rs::local_db_path()`, and the two nodes that
  build the path independently, `n.sqlite.query` and `n.sqlite.mutate`).
- `data/runtime/*` moved under `data/cache/*`, whole-directory, in
  `FilesystemFileAdapter::ensure_project_layout`
  (`src/platform/adapters/file/mod.rs`).
- Invocation records already had a stated home before this document was
  written: `src/platform/adapters/data/sqlite.rs::project_invocations_db_path`
  writes `data/logs/invocations.db`. This document's earlier claim that they
  "had no stated home" was wrong, the way §4 corrected an earlier claim about
  `files/`; the code, read directly, is what governs.
- `ProjectFileLayout` gained `data_store_dir()`, `data_store_sekejap_dir()`,
  `data_store_local_db_file()`, `data_cache_dir()`,
  `data_cache_pipelines_dir()`, `data_cache_agent_docs_dir()`,
  `data_recovery_dir()`, and `data_logs_dir()` — accessor methods derived from
  `data_dir`, the same way `repo_source_dir()` and the other `repo_*` methods
  derive from `repo_dir` and `repo_layout` rather than being stored
  separately. `ensure_project_layout` creates all four tier directories (and
  `cache/pipelines`, `cache/agent_docs`) for every project, the same way it
  already created `data/runtime/pipelines` and `data/runtime/agent_docs`.
- Migration is transparent, not a `zebflow project data migrate` command —
  see §7 for the argument against the `config`/`lock`/`pipelines` precedent.
  A pre-tier `data/sekejap`, `data/local.db`, or `data/runtime` moves to its
  new home the first time each is touched, via
  `crate::infra::io::durable::migrate_tier_entry`: one `fs::rename` per
  entry, atomic on a same-filesystem move, so a project is never left in a
  state recognizable as neither the old nor the new shape. Both `old` and
  `new` present is refused rather than guessed.

## 6. What this does not do

It does not move anything in `repo/` or `files/` — that split was already
correct. It does not change `spec.layout` or pipeline identity. It does not
define backup, transfer, or recovery *behavior* — rows 10, 11, and 12 in the
stability matrix depend on this classification existing, and are where that
behavior belongs.

It does not classify `data/nodes/` — see §7.

It does not remove the empty `files/public/` and `files/private/` directories
`ensure_project_layout` still scaffolds despite §4: nothing reads them, but
removing them is a `files/` change and out of scope for a `data/`-only sweep.

## 7. Open

- **`data/nodes/` is not one of the four tiers.** Installed node bundles are
  materialized from `zeb.lock` (an authored, git-tracked document) the same
  way `data/cache/pipelines/` is materialized from `repo/`, which argues for
  `cache`. It was left at `data/nodes/` rather than moved, for two reasons:
  the tree in §2 does not name it, so moving it would be guessing rather than
  applying a stated rule; and a bundle's `entry` path in `zeb.lock` resolves
  against it by convention across `node_registry.rs`, `hub.rs`,
  `dependency_lock.rs`, and `composite_host.rs`, with roughly a dozen tests
  hardcoding the literal path — a real move, not a one-line rename. Deciding
  its tier and applying that decision is the next piece of this row.
- Whether `data/logs/` retention is instance-configured or fixed — unchanged
  by this task; closer to stability-matrix row 16 (invocation and temporary
  state) than to this row's directory classification.
- **Migration is transparent, not a CLI command, and that is a deliberate
  departure from the `config`/`lock`/`pipelines` precedent
  (`zebflow project <noun> migrate`).** Those three migrate an ambiguous
  *document format* — more than one legacy shape can appear at the same
  path, so picking one is a judgment call an operator should invoke on
  purpose — and the old shape stays fully readable until they do: `zebflow.json`
  keeps loading through `read_path_or_default`'s legacy fallback, a pre-v1
  `zeb.lock` keeps resolving, and a repository-relative pipeline id keeps
  resolving through the same tolerant reads the migration exists to stop
  relying on. Nothing breaks if an operator never runs them.
  The `data/` tier move has neither property. It is a deterministic path
  relocation with no second reading — `data/sekejap` is always exactly the
  Sekejap store, never something else depending on how it parses — and after
  this change shipped, nothing reads the old path as a fallback: code that
  used to build `data/sekejap` or `data/local.db` now only ever builds
  `data/store/...`. A manual-only migration would mean every project on an
  upgraded instance looks empty — a fresh, empty store gets created at the
  new path next to the abandoned old one — until an operator finds and runs
  a command for each project, which is a worse failure mode than the
  precedent's "still works, just not on the canonical shape." Transparent
  migration at the point of first use is what makes the cutover safe rather
  than a silent per-project data-loss trap. It is proven live in this task's
  report: a seeded pre-tier project migrates on first touch, with before/after
  content identical, and a second touch is a no-op.
- `local.db`'s role is now verified against the code rather than asserted:
  `ProjectSqliteEngine` (`src/platform/adapters/project_data/mod.rs`) opens it
  as a persistent WAL-mode database that `n.sqlite.query`/`n.sqlite.mutate`
  read and write as durable application data, never regenerated from `repo/`.
  It belongs in `store`, confirming what this document guessed before
  implementing.
