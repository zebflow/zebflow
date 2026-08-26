# Instance directory

Stability-matrix row 2. Status: **review, with evidence**. This is the whole
tree, from the data root down. One question, one answer: "is this safe to
delete?" is answered by the tier, never by reading source.

## The contract

Adopted 2026-08-26 after three independent external reviews (grok, qwen, agy)
unanimously refused the prior shape. Code migrates toward this; the appendix
records what is on disk today.

### The tree

```
<data-root>/
├── platform/
│   ├── layout.json                    STORE — { "version": N }; migrations key off this
│   ├── catalog.db (+ -wal, -shm)      STORE — users, projects, sessions, offices,
│   │                                          credentials, members, policies, placements
│   ├── operations/                    BOUNDED (terminal + 7d) — transfer staging
│   └── cache/                         CACHE — rwe script blobs, content-addressed
│
├── services/
│   └── hub-default/                   LOCAL HUB (type three) — curated shelf, seeded
│       │                              with blessed zebflow.*, superadmin-write-only,
│       │                              read-only for all, delete = retraction
│       ├── hub.db (+ sidecars)        STORE
│       ├── packages/
│       │   └── {id}/versions/{ver}/artifact.json      STORE
│       └── artifacts/
│           └── {sha256}               STORE if reachable from hub.db, else GC
│
├── run/                               EPHEMERAL — wiped on startup: locks, pids, sockets
├── tmp/                               EPHEMERAL — staging, upload chunks, scratch
│
└── users/
    └── {owner}/
        └── {project}/
            ├── repo/                  SOURCE — git-tracked, yours
            │   ├── .git/
            │   ├── zebflow.yaml       declares internal layout
            │   ├── zeb.lock           machine-written
            │   └── ...                any layout you declare
            │
            ├── data/
            │   ├── store/             STORE — irreplaceable
            │   │   ├── sekejap/
            │   │   ├── local.db (+ sidecars)
            │   │   ├── kv.db (+ sidecars)
            │   │   ├── chat_history.json
            │   │   └── assistant/{user_id}/memory.json
            │   │
            │   ├── cache/             CACHE — regenerates
            │   │   ├── pipelines/
            │   │   ├── web-assets/
            │   │   ├── media/         thumbnails, tts
            │   │   └── mapserver-artifacts/
            │   │
            │   ├── hub/               INSTALLED — from any hub or local file
            │   │   ├── nodes/
            │   │   └── rwe-libraries/ (writer arrives with seeding)
            │   │                      repo derivatives never land here → repo/
            │   │
            │   ├── recovery/          BOUNDED — next success + 14d, keep 3
            │   └── logs/              BOUNDED — 30d or 512 MiB
            │
            └── files/                 OBJECT — user bytes, flat namespace
                ├── .zebfs/acl.json    STORE (reserved prefix re-grades)
                └── mapserver/
                    ├── *.geojson      OBJECT — uploaded sources
                    └── *.layers.json  STORE — machine-written intent
```

### Rules

1. **Reserved-namespace rule.** A dot-prefixed segment (or listed machine
   subtree) takes the tier annotated on it; the parent tier covers only
   non-reserved content. `.zebfs/` STORE, `.git/` machine, `zeb.lock` machine.
2. **AUTHORED splits into SOURCE and OBJECT** — one word could not cover both
   git-tracked source (machine writes lockfiles) and user object bytes.
3. **EPHEMERAL tier added** (`run/`, `tmp/`) — wiped on startup, never backed
   up, never exported.
4. **BOUNDED carries numbers** or does not exist: logs 30d/512 MiB,
   operations terminal+7d, recovery next-success+14d keep-3.
5. **SQLite rule:** `{name}.db`, `-wal`, `-shm` are one object; hot file-copy
   is not a restore image — checkpoint or SQLite backup API.
6. **Backup column per tier:** SOURCE/OBJECT/STORE yes; INSTALLED yes (airgap
   restore); CACHE/BOUNDED/EPHEMERAL no.
7. **Not in the project tree, said out loud:** credentials, members, policies,
   DB connections, hub bindings live in catalog.db. Zipping
   `users/{owner}/{project}` is NOT a project backup, and export does not
   carry them.
8. **Generated media leaves files/:** map artifacts, thumbnails, tts output
   move to `data/cache/`; if an engine made it and can remake it, it is not
   an OBJECT.
9. **Bootstrap secret gets a death:** password file is removed after first
   successful authentication change; class is "bootstrap secret", not STORE.
10. **Unwritten paths are not drawn:** `rwe-libraries/` returns to the tree
    the day a writer exists.

### Migration from the current disk state

Keyed off `platform/layout.json` version, one tick per relocation. Both-paths
conflicts quarantine into `data/recovery/conflicts/` instead of refusing boot.
Legacy names (`kv_durable.db`, `data/nodes`, `data/runtime`, `data/sekejap`,
`data/local.db`) live in a Legacy appendix with the last version that reads
them.

### Open

- Assistant docs (AGENTS.md, SOUL.md): stay in `data/cache/agent_docs/` for
  now. Defined later when chat activation lets a user write them; planned home
  `repo/.assistant/`. The cache-tier mismatch is known and accepted until then.


- `{owner}/{project}` path coupling: renames/transfers move gigabytes. A
  stable-id layer was proposed (agy) and deferred — re-architecture, not a
  contract fix.
- MEMORY: file vs record store, and eviction.

## Appendix — on-disk state today (pre-migration)

### The tree

```
<data-root>/                           resolved: ZEBFLOW_PLATFORM_DATA_DIR, else OS user-data path
│
├── .bootstrap/
│   └── superadmin-password            0600. bootstrap secret — dies on the first successful
│                                      password change (the change deletes it)
│
├── platform/
│   ├── layout.json                    STORE — { "version": N }; migrations key off this
│   ├── catalog.db                     STORE — users, projects, sessions, offices
│   ├── project-operations/            BOUNDED — transfer/export staging
│   │   └── op-{kind}-{ts}/            one operation: staged copy + manifest.json + archive
│   └── rwe-script-cache/              CACHE — compiled RWE, content-addressed
│       └── {hash}.blob
│
├── services/
│   └── hub-default/                   this instance's own hub (local hub)
│       ├── hub.db                     STORE — packages, versions, publishers, tokens
│       ├── packages/                  STORE — release documents
│       │   └── {package_id}/{version}/package.json
│       └── artifacts/                 STORE — content-addressed blobs
│           └── {sha256}
│
└── users/
    └── {owner}/
        └── {project}/
            │
            ├── repo/                  AUTHORED — git-tracked, portable
            │   ├── .git/              machine-managed
            │   ├── zebflow.yaml       project config; declares internal layout
            │   ├── zeb.lock           dependency lock, machine-written
            │   └── ...                everything else yours — source, docs, schemas,
            │                          any layout, declared in zebflow.yaml
            │
            ├── data/
            │   ├── store/             STORE — irreplaceable
            │   │   ├── sekejap/       project database: *.bin, wal.log, snapshot.json,
            │   │   │                  tables.json
            │   │   ├── local.db       project SQLite (n.sqlite.*)
            │   │   ├── kv.db          durable n.kv.* state
            │   │   └── chat_history.json  assistant conversation
            │   │
            │   ├── cache/             CACHE — regenerates from repo/
            │   │   ├── pipelines/     activated pipeline snapshots (*.zf.json)
            │   │   ├── agent_docs/    AGENTS.md, SOUL.md, MEMORY.md
            │   │   └── web-assets/    compiled web assets
            │   │
            │   ├── hub/               INSTALLED — unpacked hub content
            │   │   ├── nodes/         node bundles (migrated from data/nodes/ on first touch)
            │   │   │   └── {package}/ definition.json, functions/, *.wasm, icons
            │   │   └── rwe-libraries/ RWE libraries (none materialize yet)
            │   │
            │   ├── recovery/          BOUNDED — dated migration copies
            │   │   └── {name}-{date}.{ext}
            │   │
            │   └── logs/              BOUNDED — runtime records
            │       └── invocations.db
            │
            └── files/                 AUTHORED — user objects, flat namespace,
                │                      visibility per path via ACL
                ├── .zebfs/
                │   └── acl.json       path → {Private|PublicRead} × {Object|Prefix}
                ├── mapserver/         map feature area
                │   ├── {source}.geojson            uploaded sources
                │   ├── {instance}.layers.json      layer registry, machine-written
                │   └── .artifacts/
                │       └── {instance}/{layer}/     generated chunks
                │           └── {chunk}.ndjson
                └── ...                anything applications store (uploads,
                                       thumbnails, tts output, exports)
```

### Tiers as currently enforced

| Tier | Rule |
| --- | --- |
| AUTHORED | a person wrote it. Never auto-deleted, never machine-overwritten, back it up. |
| STORE | machine-written, cannot be regenerated. Back it up. |
| INSTALLED | unpacked from a hub or local document. The installed form is the artifact. Removal is `uninstall`, recorded in `zeb.lock` — never a cleanup. |
| CACHE | machine-written, regenerates. Deleting is never data loss. |
| BOUNDED | disposable after a retention window. Not backed up; not deleted early. |

### Migration mechanics shipped

Tier moves are transparent, on first touch, one atomic rename
(`migrate_tier_entry`); both-paths-present refuses. The global `kv_durable.db`
predating per-project `kv.db` drains row-level per namespace on first touch;
orphan rows for deleted projects keep the file alive until hand-drained.

### Superseded open items

- `data/logs/` and `project-operations/` retention windows: unset.
- `.bootstrap/superadmin-password` lifecycle: shipped — the first successful
  password change deletes it (rule 9), and `zeb admin reset-password` rotates
  the credential offline without recreating the file.
- `data/nodes/` → `data/hub/nodes/`: shipped — transparent first-touch move,
  both-paths-present refuses; `zeb.lock` entries resolve against `data/hub/`.

### Evidence

Live-verified 2026-08-24/25: all tiers scaffolded on fresh projects; old-shape
projects migrate with content proven intact (values queried, not file-hashed);
second runs no-op; both-paths refusal exercised; project deletion takes
`kv.db` with it. The `data/nodes` cache claim was disproved live — a
locally-installed bundle cannot be rebuilt from `zeb.lock`, hence INSTALLED,
not CACHE. History: git log of this file.


