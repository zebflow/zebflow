# OfficeTopology

Status: pending review

This contract defines where a Zebflow deployment may run work: which offices
exist, what role each holds, where platform services are placed, and which
runtime nodes are registered under each office.

It is the declaration. [`RuntimeBundle`](../runtime-bundle/README.md) is the
materialisation. Topology states *where things may run*; a runtime bundle
carries *what runs there*. Keeping them apart is the same split as a pipeline
and its activated snapshot, or a dependency lock and an installed bundle.

Its review will cover office identity and role, service placement and its
generation counter, node registration and declared capabilities, the on-disk
cluster file, and how a topology change is applied without stranding work.

## What it covers today

| Thing | Where it lives now |
| --- | --- |
| `PlatformOffice` | office id, slug, role, base URL, status |
| `PlatformServiceInstance` | which office hosts a service, which owns its state, `placement_generation` |
| `PlatformOfficeNode` | a runtime node under an office, with declared capabilities |
| `.zebflow.cluster.lock` | the provisioning topology on disk, moved between machines |

The records are persisted through the `DataAdapter` and the cluster file is
written by `src/provision/k8s.rs`. A migration governs the columns; nothing
governs what an office *is*. The Public Hub service is one such placed
`PlatformServiceInstance`; its hub semantics live in `distribution.md` §1b.

## Why it exists

Recorded during the contract review. Office topology has every property that
makes a durable contract worth having: it survives upgrades, it is edited by
operators rather than produced by the system, it is transmitted between
machines, and a wrong migration of it takes down a deployment rather than one
project. `placement_generation` already exists, so the need for versioned
placement was felt before the contract was written.

## Open question

Whether a project may express placement — pinning itself to an office or to a
class of runtime node. If it may, topology and `ProjectConfiguration` touch, and
the boundary has to be stated rather than discovered. If it may not, placement
stays entirely an operator concern and the two never meet.
