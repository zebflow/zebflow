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
│   ├── operations/                    BOUNDED (terminal + 7d) — finished transfer
│   │                                          artifacts (staging is in tmp/)
│   └── cache/                         CACHE — rwe script blobs, content-addressed
│
├── services/
│   └── hub-default/                   LOCAL HUB — curated shelf, seeded
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
            │   │   └── assistant/{user_id}/memory.md
            │   │
            │   ├── cache/             CACHE — regenerates
            │   │   ├── pipelines/
            │   │   ├── web-assets/
            │   │   └── mapserver-artifacts/   {instance}/{layer}/
            │   │
            │   ├── hub/               INSTALLED — from any hub or local file
            │   │   ├── nodes/
            │   │   └── rwe-libraries/ written by `rwe_library` installs from
            │   │                      the seeded local hub; `zeb.lock` entries
            │   │                      resolve here.
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
8. **Generated media leaves files/ only when the engine can remake it:** map
   artifacts moved to `data/cache/mapserver-artifacts/` (shipped — the engine
   rebuilds them from the uploaded source). Thumbnails and TTS output were
   read for the same move and refused it: TTS requires a user-chosen
   `output_path` (`ai_tts.rs`) and thumbnails are handed back as durable
   FileRefs at user-chosen folders whose source `--delete-source` may have
   destroyed (`fs_thumbnail.rs`) — user-addressed, possibly irreplaceable,
   hence OBJECT, and they stay. `data/cache/media/` is not drawn until a
   machine-addressed media writer exists (rule 10).
9. **Bootstrap secret gets a death:** password file is removed after first
   successful authentication change; class is "bootstrap secret", not STORE.
10. **Unwritten paths are not drawn:** `rwe-libraries/` returned to the tree
    when its writer landed — installing an `rwe_library` package from the
    seeded local hub copies its bytes there and records them in `zeb.lock`.

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
- MEMORY eviction. The tier move shipped: MEMORY lives at
  `data/store/assistant/{user_id}/memory.md`, kept as the markdown every
  reader/writer already speaks (the contract's `.json` was a sketch;
  a record store remains a possible future). `{user_id}` is the owner slug —
  the only identity the memory writers (assistant chat tools, MCP
  `docs_agent_write`) carry today; a finer acting-user key waits on identity
  being threaded into those paths.

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
│   ├── project-operations/            BOUNDED — finished transfer artifacts
│   │   └── op-{kind}-{ts}/            one operation: archive (manifest.json embedded)
│   └── rwe-script-cache/              CACHE — compiled RWE, content-addressed
│       └── {hash}.blob
│
├── run/                               EPHEMERAL — wiped on startup (contents, never
│                                      the directory); locks, pids, sockets
├── tmp/                               EPHEMERAL — wiped on startup
│   └── transfer/                      in-flight export/import staging; crash
│                                      residue dies at the next boot
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
            │   │   ├── agent_docs/    AGENTS.md, SOUL.md (MEMORY.md moved to
            │   │   │                  store/assistant/{owner}/memory.md on first touch)
            │   │   └── web-assets/    compiled web assets
            │   │
            │   ├── hub/               INSTALLED — unpacked hub content
            │   │   ├── nodes/         node bundles (migrated from data/nodes/ on first touch)
            │   │   │   └── {package}/ definition.json, functions/, *.wasm, icons
            │   │   └── rwe-libraries/ RWE libraries installed from the local hub
            │   │       └── {package}/  manifest.json + versioned runtime/wrappers
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
                │   └── .artifacts/                 generated chunks — moved to
                │       └── {instance}/{layer}/     data/cache/mapserver-artifacts/
                │           └── {chunk}.ndjson      on first touch
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
| EPHEMERAL | wiped on startup, before anything serves. `run/` and `tmp/` contents only, never through a symlink (a symlinked tier refuses boot). Never backed up, never exported. |

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
- `data/cache/agent_docs/MEMORY.md` → `data/store/assistant/{owner}/memory.md`:
  shipped — transparent first-touch move on any MEMORY read/write,
  both-paths-present refuses; the pre-move project-wide file migrates to the
  project owner's file (the only writer identity threaded today).
  AGENTS.md and SOUL.md stay in `data/cache/agent_docs/` per the open item.
- `files/mapserver/.artifacts/` → `data/cache/mapserver-artifacts/`: shipped —
  whole-directory first-touch rename at every artifact writer, reader, and
  cleanup site; the layer registry (`{instance}.layers.json`, STORE, stays in
  `files/`) is not rewritten — stored `mapserver/.artifacts/...` rel paths
  keep resolving into the moved tree alongside new `mapserver-artifacts/...`
  entries.

### Evidence

Live-verified 2026-08-24/25: all tiers scaffolded on fresh projects; old-shape
projects migrate with content proven intact (values queried, not file-hashed);
second runs no-op; both-paths refusal exercised; project deletion takes
`kv.db` with it. The `data/nodes` cache claim was disproved live — a
locally-installed bundle cannot be rebuilt from `zeb.lock`, hence INSTALLED,
not CACHE. History: git log of this file.


