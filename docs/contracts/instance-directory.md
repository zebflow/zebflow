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
├── .bootstrap/                        BOOTSTRAP SECRET (rule 9) — 0700
│   └── superadmin-password            0600; deleted by the first successful
│                                      password change. Never backed up
│
├── platform/
│   ├── layout.json                    STORE — { "version": N }; migrations key off this
│   ├── catalog.db (+ -wal, -shm)      STORE — users, projects, sessions, offices,
│   │                                          credentials, members, policies, placements
│   ├── credential-key                 STORE — 0600. The instance key that unwraps
│   │                                          every credential in catalog.db. Losing
│   │                                          it loses every stored credential; it
│   │                                          cannot be regenerated. Back it up,
│   │                                          separately from catalog.db
│   ├── cluster-signing-key            STORE — 0600. Controller identity; offices
│   │                                          that joined trust its public half
│   ├── office-join-token              STORE — 0600. This office's own issued token
│   │                                          (present on a joined office, not on
│   │                                          a controller)
│   ├── operations/                    BOUNDED (terminal + 7d) — finished transfer
│   │                                          artifacts (staging is in tmp/)
│   ├── rwe-script-cache/              CACHE — compiled RWE script blobs,
│   │   └── {hash}.blob                        content-addressed
│   └── cache/
│       └── hub-artifacts/             CACHE — remotely fetched hub artifacts,
│                                              content-addressed and digest-verified
│                                              on fetch; neither hub store may be
│                                              written by a fetch
│
├── services/
│   ├── hub-local/                     LOCAL HUB — blessed shelf, release-seeded
│   │   │                              (zebflow.*, every boot, check-first:
│   │   │                              absent coordinates only, existing ones
│   │   │                              never rewritten or deleted), immutable;
│   │   │                              no write path, no tokens.
│   │   │                              Not CACHE, though it is re-seeded: seeding
│   │   │                              only adds coordinates this release knows
│   │   │                              about, so a package published here by an
│   │   │                              older binary or installed from a local
│   │   │                              document is never restored by deleting the
│   │   │                              shelf. Back it up
│   │   ├── hub.db (+ sidecars)        STORE
│   │   ├── packages/
│   │   │   └── {id}/versions/{ver}/artifact.json      STORE
│   │   └── artifacts/
│   │       └── {sha256}               STORE if reachable from hub.db, else GC
│   └── hub-public/                    PUBLIC HUB STORE — exists only on the
│                                      office hosting the placed hub service;
│                                      the one publish target
│                                      (publisher/token/grant ACL); same
│                                      internal layout as hub-local
│
├── run/                               EPHEMERAL — created and wiped on startup.
│                                      No writer today; drawn because the wipe is
│                                      what makes it safe for the first one
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
            │   │   ├── agent_docs/    AGENTS.md, SOUL.md (see Open)
            │   │   └── mapserver-artifacts/   {instance}/{layer}/
            │   │
            │   ├── hub/               INSTALLED — from any hub or local file
            │   │   ├── nodes/
            │   │   └── rwe-libraries/ written by `rwe_library` installs — any
            │   │                      hub serving or direct ingestion;
            │   │                      `zeb.lock` entries resolve here.
            │   │                      repo derivatives never land here → repo/
            │   │
            │   ├── recovery/          BOUNDED — next success + 14d, keep 3
            │   └── logs/              BOUNDED — 30d or 512 MiB
            │
            └── files/                 OBJECT — user bytes, flat namespace
                ├── .zebfs/acl.json    STORE (reserved prefix re-grades)
                ├── tmp/runs/{request_id}/files/
                │                      EPHEMERAL by intent, OBJECT by location —
                │                      `lifecycle: temporary` FileRef bytes
                │                      (`kinds/file-ref/README.md`). Drawn 2026-08-31
                │                      because it is written and an operator backing
                │                      up `files/` copies it. Two things are owed:
                │                      the deletion the lifecycle promises, and a
                │                      home outside the OBJECT tier. Not reserved —
                │                      a project may legitimately own `files/tmp/`
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
│   ├── credential-key                 STORE — 0600, instance credential key
│   ├── cluster-signing-key            STORE — 0600, controller identity (controller only)
│   ├── office-join-token              STORE — 0600, this office's token (joined office only)
│   ├── project-operations/            BOUNDED — finished transfer artifacts
│   │   └── op-{kind}-{ts}/            one operation: archive (manifest.json embedded)
│   ├── cache/
│   │   └── hub-artifacts/             CACHE — remotely fetched hub artifacts
│   └── rwe-script-cache/              CACHE — compiled RWE, content-addressed
│       └── {hash}.blob
│
├── run/                               EPHEMERAL — wiped on startup (contents, never
│                                      the directory). No writer today
├── tmp/                               EPHEMERAL — wiped on startup
│   └── transfer/                      in-flight export/import staging; crash
│                                      residue dies at the next boot
│
├── services/
│   └── hub-local/                     this instance's own hub (the rename from
│       │                              hub-default shipped; `LOCAL_HUB_STORE_DIR`
│       │                              in `hub.rs`. `hub-default` survives only as
│       │                              the hub *service instance id*, a catalog
│       │                              row, not a directory)
│       ├── hub.db                     STORE — packages, versions, publishers, tokens
│       ├── packages/                  STORE — release documents
│       │   └── {package_id}/versions/{version}/artifact.json
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
            │   │   ├── sekejap/       project database (sekejap 0.17): data file and its WAL
            │   │   ├── local.db       project SQLite (n.sqlite.*)
            │   │   ├── kv.db          durable n.kv.* state
            │   │   └── chat_history.json  assistant conversation
            │   │
            │   ├── cache/             CACHE — regenerates from repo/
            │   │   ├── pipelines/     activated pipeline snapshots (*.zf.json)
            │   │   └── agent_docs/    AGENTS.md, SOUL.md (MEMORY.md moved to
            │   │                      store/assistant/{owner}/memory.md on first touch)
            │   │
            │   ├── hub/               INSTALLED — unpacked hub content
            │   │   ├── nodes/         node bundles (migrated from data/nodes/ on first touch)
            │   │   │   └── {package}/ definition.json, functions/, *.wasm, icons
            │   │   └── rwe-libraries/ installed RWE libraries (hub or supplied file)
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
                ├── tmp/runs/          `lifecycle: temporary` FileRef bytes; no
                │   └── {request_id}/files/    writer removes them today
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
- `services/hub-default/` → `services/hub-local/`: shipped —
  `LOCAL_HUB_STORE_DIR` in `hub.rs` is `hub-local`. The appendix line saying
  the code catch-up was owed was stale and is corrected (2026-08-31).
- `data/cache/web-assets/`: removed from both trees (rule 10) — a reader with
  no writer. `GET /static/{owner}/{project}/…` looks there first and falls
  through to `repo/{static}/`, which is where every served file comes from
  today.
  It returns the day a compiler writes it.
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

Live-verified 2026-08-31 on a scratch instance (`ZEBFLOW_PLATFORM_DATA_DIR`,
fresh root, one project through create → pipeline → upload → library →
publish → export/import → mint): every path in the appendix tree that the run
could reach was present, and the three additions above were observed as
written — `.bootstrap/superadmin-password` (0600), `platform/credential-key`
(0600), `platform/cluster-signing-key` (0600), `platform/rwe-script-cache/`,
`data/cache/agent_docs/`, and `files/tmp/runs/{request_id}/files/`.
`platform/cache/hub-artifacts/` was not reached (it needs a remote hub fetch)
and is drawn from its writer in `hub.rs`. `run/` was created and empty;
`data/cache/web-assets/` was never created, which is what a reader with no
writer looks like. `platform/office-join-token` is not written on a controller
— it is the office side's file — and is drawn from `join_token.rs`.

Live-verified 2026-08-24/25: all tiers scaffolded on fresh projects; old-shape
projects migrate with content proven intact (values queried, not file-hashed);
second runs no-op; both-paths refusal exercised; project deletion takes
`kv.db` with it. The `data/nodes` cache claim was disproved live — a
locally-installed bundle cannot be rebuilt from `zeb.lock`, hence INSTALLED,
not CACHE. History: git log of this file.


