# Milestone 0.2.1

## Purpose

`0.2.0` establishes the baseline vocabulary and first clustered slice:

- office / controller model
- project runtime placement
- first controller-to-office sync
- first multi-office runtime routing
- baseline portability models

`0.2.1` should stabilize that baseline for real use across:

- Raspberry Pi / Jetson Nano
- one office on one server
- one controller with multiple offices
- Kubernetes multi-node deployment

This milestone is not about adding every future feature. It is about removing the most important operational gaps in portability, runtime ownership, migration, and multi-node stability.

---

## 1. First-Class Project Export / Import

### Why

Project migration is still too tied to local storage layout and operator knowledge.

We need a first-class portable unit that an operator can export, inspect, move, and import without copying whole PVCs or entire platform databases.

### First practical shape

Start with two export artifacts:

- `project.bundle.tar.zst`
  - `repo/`
  - `data/`
  - `manifest.json`
- `project.files.tar.zst`
  - `files/public/`
  - `files/private/`

### Requirements

- Export one project only, not the whole office.
- Keep project metadata scoped and explicit.
- Make the manifest human-readable and machine-usable.
- Support later import into:
  - same office
  - another office
  - another controller
  - standalone office

---

## 2. Office-Local Authoring Authority

### Why

Current project studio behavior is still controller-first, while runtime is office-first.

That is the main architectural mismatch.

### Rule

If a project is placed on an office, then project authoring should belong to that office.

The controller should remain:

- coordinator
- inventory lens
- policy authority
- migration/orchestration authority

The office should own:

- project working tree
- template editing
- pipeline editing
- project-local mutation
- runtime-local files and state

---

## 3. Automatic Sync on Mutation

### Why

Remote runtime state currently becomes stale after controller-side mutation unless the operator manually resyncs.

### Must cover

- repo file changes
- credential changes
- DB connection changes
- runtime profile changes
- bootstrap activation changes

### Minimum acceptable behavior

If full auto-sync is not ready, the system must at least expose:

- pending-sync status
- explicit sync action
- last successful sync time

---

## 4. Runtime Ingress Policy

### Why

The current cluster slice supports controller-proxy runtime access, but the long-term architecture requires explicit control over whether runtime traffic should flow through the controller or directly to the office/runtime.

### Required model

Per project, define ingress policy:

- `controller_proxy`
- `direct_runtime`

And derive stable endpoint groups for:

- pages
- APIs
- webhooks
- websocket
- `/files`

### Rule

Control traffic belongs to the controller.  
Runtime traffic belongs to the office/runtime.

---

## 5. Controller–Office Contract Persistence

### Why

Office binding must not collapse just because a heartbeat is lost.

### Required distinction

- `binding` is durable
- `status` is temporary

### Rules

- heartbeat loss changes status, not ownership
- an office remains bound until explicit detach/attach/reassign
- binding changes must be explicit mutation, never timeout-based

This is the governance contract required for resilient multi-office deployments.

---

## 6. Real Migration Workflow

### Why

Migration must be a first-class operator action, not a collection of undocumented manual steps.

### First implementation target

Clone-first migration:

1. export project
2. import into target office
3. validate target
4. cut over
5. keep rollback path

### Later

Live handoff can come later, but `0.2.1` should make clone-first operational and explicit.

---

## 7. mTLS and Stronger Internal Trust

### Why

The current shared token is acceptable only for early trusted deployments.

Serious multi-node and Kubernetes deployments need stronger office-controller trust boundaries.

### Target

- office certificate issuance
- controller trust root
- mTLS for internal control traffic
- token/bootstrap can remain only as the first join mechanism

---

## 8. StateBus Evolution Beyond Local Memory

### Why

Current `MemStateBus` is correct for:

- standalone
- one project pinned to one office
- local runtime coordination

It is not the final answer for multi-instance shared runtime state.

### Next step

Support a second backend:

- `RemoteStateBus` or
- `Redis/Valkey-backed StateBus`

### Use case

Needed once:

- one project spans multiple runtime instances
- shared leases or coordination are required across nodes

---

## 9. Kubernetes Manifest Generation

### Why

Hand-written manifests are useful for early testing but not enough for repeatable deployment at scale.

### Required commands

- `zebflow k8s render controller`
- `zebflow k8s render office`
- `zebflow k8s render project-runtime`

### Output should cover

- deployment
- service
- pvc
- storage class defaults
- env/config
- health probes
- resource profile mapping

---

## 10. Dedicated Runtime and Resource Policy

### Why

Not every project should live in the same shared office shape.

High-traffic projects need dedicated resources and independent scaling.

### Required runtime modes

- `shared`
- `pinned`
- `dedicated`

### Why this matters

It lets the platform serve:

- one heavy public site
- one medium application
- several low-traffic sites

without forcing a single runtime topology for all of them.

---

## Recommended Delivery Order

1. First-class export / import
2. Office-local authoring authority
3. Sync-on-mutation or at least visible pending-sync control
4. Runtime ingress policy
5. Controller–office contract persistence
6. Clone-first migration workflow
7. mTLS
8. StateBus second backend
9. Kubernetes manifest generation
10. Dedicated runtime deployment path

---

## Stability Goal

At the end of `0.2.1`, Zebflow should be stable enough that:

- one office can run independently on a Raspberry Pi
- one controller can manage multiple offices cleanly
- projects can move between offices with an explicit export/import process
- runtime traffic can be assigned clearly to controller-proxy or direct runtime
- Kubernetes deployment can be repeated without rediscovering storage and topology details each time
