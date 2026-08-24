# Project directory

Status: **draft**. This is the permanent shape; the reclassification below has
not yet been applied to the code. Stability-matrix row 2.

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

## 5. What changes, concretely

- `repo/*.pre-*.json` recovery files move to `data/recovery/`, named with the
  date the migration ran (`zebflow-config-2026-08-20.json`), so `repo/` never
  carries a machine-generated file again.
- `data/sekejap` and any equivalent store move under `data/store/`.
- `data/runtime/*` moves under `data/cache/`.
- Invocation records and traces get a stated home, `data/logs/`, rather than
  living wherever the pipeline runtime currently puts them.
- `ProjectFileLayout` gains fields for the four tiers, the way it already
  carries `repo_source_dir()` and the others `spec.layout` resolves.

## 6. What this does not do

It does not move anything in `repo/` or `files/` — that split was already
correct. It does not change `spec.layout` or pipeline identity. It does not
define backup, transfer, or recovery *behavior* — rows 10, 11, and 12 in the
stability matrix depend on this classification existing, and are where that
behavior belongs.

## 7. Open

- Exact migration path for existing instances: today's `data/sekejap` and
  `data/runtime` are not empty on a running instance, so this needs a move, not
  a fresh default.
- Whether `data/logs/` retention is instance-configured or fixed.
- `local.db`'s actual role was not verified against the code before this
  document was written; confirm it belongs in `store` rather than `cache`
  before implementing.
