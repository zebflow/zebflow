# Kekaisaran Zebflow — Gagal Adalah Mati

**Potong kepala untuk setiap kegagalan idiotkratik.**

Record of what is settled, what is not, and where I was wrong.
Written 2026-08-25.

## 1. Platform folder — settled

```
<data-root>/
├── .bootstrap/
│   └── superadmin-password                   0600
├── platform/
│   ├── catalog.db                            users, projects, sessions
│   ├── project-operations/                   transfer / export state
│   └── rwe-script-cache/                     compiled RWE, platform-wide
├── services/
│   └── hub-default/                          this instance's own hub
│       ├── hub.db                            packages, versions, publishers, tokens
│       ├── packages/                         release documents
│       └── artifacts/                        content-addressed, sha256-named
└── users/{owner}/{project}/
    ├── repo/                                 AUTHORED — git-tracked, portable
    │   ├── .git/
    │   ├── zebflow.yaml
    │   ├── zeb.lock
    │   ├── docs/
    │   ├── schemas/
    │   └── {spec.layout.source}/             default "pipelines/"
    ├── data/
    │   ├── store/                            DURABLE — sekejap/, local.db
    │   ├── cache/                            DISPOSABLE — pipelines/, agent_docs/
    │   ├── hub/                              INSTALLED — nodes/, rwe-libraries/
    │   ├── recovery/                         BOUNDED — migration copies
    │   └── logs/                             BOUNDED — invocations.db
    └── files/                                USER DATA — flat namespace
        └── .zebfs/acl.json                   per-path visibility
```

Five tiers under `data/`. A tool asks which tier, not which path, to know
whether something is safe to delete.

`files/` has no public/private directories. `ZebFsAclManifest` governs
visibility per path. Scaffolding for them was removed.

## 2. Hub kinds — settled

```
HUB
├── LOCAL HUB          this instance's own
│     publish:  hub/assets/publish
│     visible:  this instance, per its access rules
│     trust:    the publishing instance itself
│     at:       services/hub-default/
│
└── REMOTE HUB         consumed over the network
    ├── API HUB               a running Zebflow instance over HTTP
    │     publish: hub/remote/assets/publish
    │     trust:   publisher token + repository grant
    │     default: hub.zebflow.com            priority 10
    │
    └── STATIC REPOSITORY     any HTTPS location, no server logic
          index:  zebflow-repository.json     (HubRepositoryIndex kind)
          trust:  the URL the user named + locked digest
          default: github.com/zebflow/hub     priority 20
          publish: not supported — read-only
```

## 3. Install flow — settled

```
zeb install <ref>
  → resolve across hub sources in priority order, first hit wins
  → fetch release document + artifacts, verify digest
  → review → refuse violations → consent
  → write by kind:
        node bundle   → data/hub/nodes/{package}/
        RWE library   → data/hub/libraries/{package}/
        project       → repo/  (a new project)
  → record in zeb.lock: version, source_id, integrity
```

Installing from a local file uses the same path and the same destination. The
installed form is the artifact — the same contract Blender addons, npm packages
and apt packages have. No shadow blob store behind it.

## 4. Not settled

- **`data/nodes` → `data/hub/nodes`** — rename not applied in code.
- **`data/logs/` retention** — fixed window, instance-configured, or unbounded.
- **`.bootstrap/`** — no contract row owns it.
- **Map server generated artifacts** — never traced; row 18.
- **`LibraryManifest`** (row 17) — one paragraph, "pending review". Libraries
  are hub-installed like nodes. That decision was made in conversation and is
  written in no contract.
- **`OfficeTopology`** (registered kind) — "pending review". Its own open
  question: whether a project may pin itself to an office, or whether placement
  stays entirely an operator concern.

## 5. Where I was wrong

Recorded because the same mistake repeats otherwise.

- **Claimed a hub is pinned to one office.** I read one constructor setting
  `host_office_id == state_office_id` and called it the architecture.
  `OfficeTopology` has them as separate fields precisely so a service can move,
  which is what `placement_generation` counts. Each office can stand on its own.
- **Claimed RWE libraries were resolved.** I checked only that they write
  nothing to disk today, then said the question was closed. It was not.
- **Flip-flopped on whether a local install needs a blob store** — three
  positions in ten minutes, the last one only because it was pushed back on.
  The Blender comparison settled it: the unpacked install is the artifact.
- **Proposed splitting `data/` across four OS directories.** Never asked for,
  would have fought the `repo/` `data/` `files/` split already decided.
- **Proposed three cases for resolving the data root**, where case two made an
  instance depend on the working directory — the exact bug being fixed.
- **Copied `~/.zebflow` from an example** instead of working out the platform
  convention.

The pattern: asserting from a grep instead of reading, and agreeing instead of
reasoning. Both cost more to correct than they saved.
