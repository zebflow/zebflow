# DatabaseSchema

Status: **review** — spec settled 2026-08-29, code catch-up owed.

How a project's database structure and starting rows live in `repo/`, so a
project installed anywhere comes up working.

The repo is the carrier. A database does not travel as a snapshot in the
ordinary path; it travels as files a fresh store replays.

## Identity

| | |
| --- | --- |
| Canonical directory | `repo/initial-data/{engine}/` |
| Accepted aliases | `repo/init/{engine}/`, `repo/seeds/{engine}/` (legacy; new projects use the canonical form) |
| Engines | `sekejap`, `sqlite` — the project's **own** store under `data/store/` |
| Declared by | `zebflow.yaml` `repo_layout.initial_data`, defaulting to `INITIAL_DATA_DIRS` |
| Format | plain text statements the engine executes, one file per step |

Engines today are the project's own store. Postgres and MySQL are reached
through connections and nodes, and saving their structure is a safe read that
this contract is shaped to extend to, once Zebflow is stable (see Open).
Applying a
structure *into* them is the separate, riskier half: it writes into a server
the user already owns.

## Two halves, two owners

```
repo/initial-data/sekejap/
├── 000_structure.sql      platform-owned: overwritten by every save
├── 010_categories.sql     yours: never touched
└── 020_sample_posts.sql   yours: never touched
```

| Half | Content | Written by |
| --- | --- | --- |
| Structure | collections, tables, indexes | the **save** action, reading the live store |
| Rows | `INSERT`s: lookup values, categories, sample content | a human — no machine knows which rows matter |

Rules:

- The save action owns `000_structure.sql` and nothing else. Any other file in
  the directory is authored state and is never read, rewritten, or deleted by
  the platform.
- Files replay in filename order; `000_structure.sql` sorts first so structure
  precedes rows.
- Saving is explicit — a button in the project's data section. The platform
  never writes `repo/` on its own schedule.

## Publish is the checkpoint

Publishing to a hub compares the live store's structure against
`000_structure.sql`. If they differ, publish stops and names the file: the
saved structure is stale, save it and publish again. Nothing is written on the
author's behalf; a stale file is refused rather than corrected.

The check exists because staleness stops being the author's problem the moment
the package leaves: the installer gets a project whose rows insert into tables
that do not exist.

## Load is creation-only

A project created with an empty store replays every file in its declared
directories, in order, once. That is the whole of it: hub install and any
create-from-repo path.

- A store that already exists is never touched. There is no second run, so
  there is nothing to track and nothing to check.
- A failed step fails the creation: the project is not kept half-born.
- A project created **with** a store snapshot (`ProjectBundle` carrying
  `store`) replays nothing — the snapshot is the state
  ([`project-bundle/README.md`](../project-bundle/README.md)).

The rule is not about the channel. It is about whether a store snapshot came
with the project: no snapshot, replay; snapshot, trust it.

## Open

Deliberately unbuilt. Each is a decision, not an oversight.

- **Postgres and MySQL, saving.** The project already connects to them
  through nodes; capturing their structure is the same read, into the same
  `000_structure.sql` shape, under the same engine directory. Not excluded —
  deferred until Zebflow is stable.
- **Postgres and MySQL, applying.** The dangerous half: writing into a server
  the user already owns needs target selection and consent.
- **Capturing rows.** Saving structure is safe; saving data would copy a
  customer table into every clone. Which rows are shippable is a marking
  problem nobody has needed yet.
- **Changing an existing store.** No upgrades, no diffing, no drift detection.
  When a live project needs a new table, that design starts here: a record of
  what a store has applied, which the creation-only rule makes unnecessary
  today.
- **Re-runnable steps.** Every file is run-once. A class that re-runs when its
  content changes (Flyway `R__`) is the ecosystem's answer for reference data
  and can be added without disturbing the above.
- **Collapsing the aliases.** Three directory names mean one thing.
