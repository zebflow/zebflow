# DatabaseSchema

Status: **review** — describes what ships today; the SQL convergence below is
agreed and not yet built.

How a project's database structure and starting rows live in `repo/`, so a
project installed anywhere comes up working.

The repo is the carrier. A database does not travel as a snapshot in the
ordinary path; it travels as files a fresh store replays.

## What is in the repository

```
repo/
├── schemas/
│   ├── sekejap/schema.json    structure, machine-written: a DatabaseSchema envelope
│   └── sqlite/schema.sql      structure, machine-written: plain CREATE statements
└── initial-data/
    ├── sekejap/*.sql          rows, hand-authored: plain statements
    └── sqlite/*.sql           rows, hand-authored: plain statements
```

| | |
| --- | --- |
| Structure, declared by | `spec.layout.schema` (default `schemas/sekejap`), `spec.layout.sqlite_schema` (default `schemas/sqlite`) |
| Rows, declared by | `spec.layout.initial_data`, defaulting to `INITIAL_DATA_DIRS` |
| Engines | `sekejap`, `sqlite` — the project's **own** store under `data/store/` |
| Envelope | structure: sekejap yes (`DatabaseSchema`), sqlite no. Rows: never |

Engines are the project's own store. Postgres and MySQL are reached through
connections and nodes; capturing their structure is a safe read this contract
is shaped to extend to, and applying a structure *into* them is the separate,
riskier half — it writes into a server the user already owns.

## Two halves, two owners

| Half | Content | Written by |
| --- | --- | --- |
| Structure | tables, columns, indexes | the platform, reading the live store |
| Rows | `INSERT`s: lookup values, categories, sample content | a human — no machine knows which rows matter |

Rules:

- The platform owns the structure documents and nothing else. Every file under
  an `initial_data` directory is authored state and is never read, rewritten,
  or deleted by the platform.
- Rows replay in filename order.
- Sekejap's structure document is rewritten after every DDL the project runs.
  SQLite's is written when a project bundle is exported.

## Load is creation-only

A project created with an empty store applies its structure documents and
replays every declared initial-data step, in order, once — `initiate_project_store`
in `services/hub.rs`. That is the whole of it: hub install and any
create-from-repo path.

- A store that already exists is never touched. A table that already exists is
  skipped. There is no second run, so there is nothing to track.
- A failed step fails the creation: the project is not kept half-born.
- A project created **with** a store snapshot (`ProjectBundle` carrying
  `store`) replays nothing — the snapshot is the state
  ([`project-bundle/README.md`](../project-bundle/README.md)).

The rule is not about the channel. It is about whether a store snapshot came
with the project: no snapshot, replay; snapshot, trust it.

## Structure is read from the database, never from a cache

Nothing mirrors the live structure. The studio's table list, the properties
panel, and the export all ask the store:

- sekejap — `SHOW TABLES` names them, including tables declared but never
  written to; `table_schema()` reports each field with its type and index
  kinds; `collection().count()` gives the row count.
- sqlite — `sqlite_master`.

A structure fact the engine cannot answer is a fact Zebflow does not keep. Two
consequences hold today: a table carries no display title of its own, and no
column carries a `DEFAULT` other than the `UUIDV4()` on `_key`, because
sekejap's parser accepts no other and reports none back
(`sql.rs::parse_field_default`).

Evidence: `src/platform/sekejap.rs` (`live_tables`, `sync_schema_to_repo`,
`apply_schema_from_repo`), `src/platform/sqlite_schema.rs`,
`src/platform/services/hub.rs` (`initiate_project_store`,
`execute_project_initial_data`).

## What the engine owes this contract

Structure now comes from the engine on every read, so what the engine cannot
report, Zebflow cannot show. For sekejap these are the gaps, in the order they
cost the most:

| Gap | Today | Wanted |
| --- | --- | --- |
| Column defaults | `parse_field_default` accepts `UUIDV4()` and `UUIDV5()`; every other `DEFAULT` is parsed and discarded without an error, and `TableSchema` reports none back | accept a literal default, store it, report it, and reject what it will not honour instead of dropping it silently |
| Nullability | `ALTER TABLE … NOT NULL` parses; `FieldDef` has no nullable flag and nothing reports one | report `NOT NULL` per column |
| Indexes in generated DDL | `SHOW CREATE TABLE` emits columns only; `dump_sql` emits `CREATE INDEX` too | have the single-table DDL match the dump |

Postgres answers all three through `information_schema.columns`, and SQLite
through `PRAGMA table_info` plus `sqlite_master`. Sekejap answering them is what
lets the studio drop the last of its own bookkeeping and lets sekejap's
structure document become plain SQL.

A silent discard is the worst of the three: it lets a user set a default in the
studio, see it accepted, and get a column that has none. Zebflow removed that
input rather than keep the promise it could not honour.

## Open

Deliberately unbuilt. Each is a decision, not an oversight.

- **Sekejap structure as SQL.** Agreed target, not built. SQLite already writes
  plain `CREATE` statements and both engines already replay plain `.sql` rows;
  sekejap alone writes a JSON envelope, and sekejap already emits its own DDL
  (`SHOW CREATE TABLE`, `dump_sql`). Converging it retires this kind's envelope
  and leaves one shape for every engine. Held until sekejap's API settles.
- **Explicit save.** Sekejap's structure document is written on every DDL, not
  on a user's action. The agreed shape is a button, next to a second that
  captures current rows as an initial-data file.
- **Staleness.** Nothing compares the saved structure against the live store, so
  publishing can ship a stale document. The agreed answer is that publish
  refuses and names the file rather than correcting it.
- **Export selection.** Which initial-data files a bundle carries is not asked;
  the agreed shape asks at export time.
- **Postgres and MySQL.** Saving is the same safe read. Applying needs target
  selection and consent.
- **Changing an existing store.** No upgrades, no diffing, no drift detection.
  When a live project needs a new table, that design starts here.
- **Re-runnable steps.** Every file is run-once. A class that re-runs when its
  content changes (Flyway `R__`) can be added without disturbing the above.
