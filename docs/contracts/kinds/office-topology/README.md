# OfficeTopology

Status: **review** — spec settled 2026-08-30; code caught up. Audited 2026-09-01: `office_kind` is the `ClusterRole` enum, an office may not be written without a `base_url`, `office_slug` is `UNIQUE`, and a service instance's host is a foreign key. The entries under Open are open, not owed.

Where a Zebflow deployment may run work: which offices exist, what role each
holds, where platform services are placed, and which runtime nodes are
registered under each office.

What an office *is* — what it keeps, what it accepts on joining, how it leaves —
is [`offices.md`](../../offices.md). This kind is the record of who exists.

It states *where things may run*. Nothing materialises a project from one
instance onto another: a project is created on the office that will run it, and
moves between instances only as a
[`ProjectBundle`](../project-bundle/README.md), in any direction.

## What it covers

| Record | Holds |
| --- | --- |
| `PlatformOffice` | `office_id`, `office_slug`, `label`, `office_kind`, `base_url`, `status` |
| `PlatformServiceInstance` | which office hosts a service, which owns its state, `placement_generation` |
| `PlatformOfficeNode` | a runtime node under an office, with its declared capabilities |
| `management.yaml` | the provisioning topology on disk, written and read by the k8s commands |

```json
{
  "office_id": "of-8f2c",
  "office_slug": "sg-1",
  "label": "Singapore",
  "office_kind": "office",
  "base_url": "https://sg-1.example.com",
  "status": "online"
}
```

`office_kind` is `controller`, `office`, or `standalone` — the surfaced names of
`ClusterRole::{Master, Worker, Standalone}`.

`base_url` is load-bearing rather than cosmetic: it is where the public reaches
every project that office runs, and it is how any other instance reaches it.
The controller carries no traffic of its own (`offices.md` §3).

The Public Hub is one placed `PlatformServiceInstance`; its hub semantics live
in `distribution.md` §1b and are not restated here.

## Placement is the operator's

**A project does not express where it runs.** It cannot pin itself to an office
or to a class of runtime node, and `ProjectConfiguration` never names a
destination. The operator chooses, at the moment a project is created or
imported.

This is a deliberate limit, not an omission. Resource provisioning driven by
project-declared needs is a second system: a matcher, a notion of capacity, a
failure mode where a project is unplaceable, and a new surface where a project
author influences where their code lands. None of that is contracted yet, and
none of it is needed to run several offices.

What would reopen it: data residency that must be enforced rather than
arranged, or hardware a project genuinely cannot run without. Until then,
`PlatformOfficeNode.capabilities` is a description an operator reads, never an
input a matcher consumes.

## Rejections

An `office_kind` outside the three roles. An office without a `base_url` — an
office nothing can reach is not an office. Two offices sharing a slug. A
service instance whose host office does not exist.

## Open

- **Applying a change without stranding work.** `placement_generation` exists
  because versioned placement was felt before it was written, but what happens
  to work in flight when a service moves is undefined.
- **Status.** `status` is a free-text summary with no stated vocabulary and no
  rule for how stale an entry may be before it stops being shown. Minting a
  join token (`offices.md` §8) now creates the office record before anything
  registers and writes `planned` — the first word meaning "exists, has never
  been reached", and the case a vocabulary would most need to name.
- **`management.yaml`.** Written and read by the provisioning commands, with no
  stated schema and no version field.
- **Correction, 2026-08-30.** An earlier draft named `.zebflow.cluster.lock` as
  the on-disk topology. It is a mutex — opened with a five-second timeout and a
  retry loop so two provisioning commands cannot run at once. It holds no
  topology and never moves between machines.
