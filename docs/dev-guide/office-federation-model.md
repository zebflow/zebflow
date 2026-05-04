# Office Federation Model

> Companion to the normative [Office Federation Contract](./office-federation-contract.md).
> This document focuses on the formal governance model and invariants.

> Formal governance model for Zebflow offices.
> This document defines the intended contract. It is normative for terminology and invariants.
> Implementation may lag the model, but should not contradict it without explicit revision here.

## 1. Scope

This document formalizes the control and runtime relationship between:

- Zebflow offices
- projects hosted by offices
- platform services hosted by offices
- the management authority that may govern one office or many offices

This model is intentionally separate from:

- application-level user authentication inside a project
- particular transport implementations such as HTTP, gRPC, or mTLS
- particular deployment substrates such as Docker, Kubernetes, or a single laptop

## 2. Modeling Stance

Every Zebflow installation is an **Office**.

`controller` is not a different species of machine. It is a governance role exercised by an
self-controlled office over itself and, optionally, over other offices.

So the product is modeled as:

- offices
- projects hosted by offices
- platform service instances hosted by offices
- control contracts between offices

This stance avoids the conceptual weakness of `master/worker`, because offices are not merely
subordinate executors. An office hosts runtime, state, draft source, and project-local mutation.
It also hosts platform service embodiments such as marketplace.

Formal stance:

> An office owns the local state required by the projects and platform services
> it hosts. Federation lends management authority to a controller; it does not
> move the office's local truth into the controller.

## 2a. Governance Intuition (Non-Normative)

As an internal reasoning metaphor, the federation can be understood as a kingdom-style
subjugation model:

- a self-controlled office is a sovereign kingdom
- a federated office is a subjugated kingdom
- the ruling office defines the management constitution
- local runtime still remains local to the subjugated office
- local operators in a federated office are appointed governors, not sovereign rulers

The important consequence is:

- subjugation changes **sovereignty**
- it does not automatically erase **local runtime embodiment**

So a federated office still:

- serves its own projects
- holds local runtime state
- acts as the embodiment of the hosted projects

But it no longer defines independent management sovereignty while federated.

This metaphor extends to ownership recovery after dissolution. Consider the Mongol Empire: the
Great Khan in Karakorum governs the Khanate of Persia. A Mongol-appointed governor in Tabriz
manages a silk road caravanserai under the Great Khan's authority. When the Great Khan's court
dissolves, the Khanate of Persia does not lose its caravanserai — it still stands, still trades,
still operates. But the governor's appointment traced back to Karakorum. The local Khan must now
formally claim that caravanserai under local authority — reassigning it from a Mongol governor's
name to a Persian administrator's name. The building does not move. The trade routes do not
change. Only the ownership record is updated.

In Zebflow terms: a controller office creates a project under a controller-side user (e.g.
`mongol-governor/silk-caravanserai`). When the controller dissolves, the local office detaches to
self-control. The project still runs — runtime is local. But the owner principal is a shadow user
that traces back to the dissolved controller. The local superadmin must be able to transfer project
ownership to a local user (e.g. `persian-admin/silk-caravanserai`), completing the sovereignty
recovery.

This metaphor is useful for reasoning about authority transfer, detachment, ownership recovery,
and delegated local administration. The formal contract, however, is the office/parent/root model
defined in the rest of this document.

## 3. Core Sets And Functions

Let:

- `O` be the set of offices
- `P` be the set of projects
- `S` be the set of platform service instances

Define:

- `host : P -> O`
  - `host(p)` is the office that currently hosts project `p`
- `service_host : S -> O`
  - `service_host(s)` is the office that currently hosts platform service
    instance `s`
- `state_host : S -> O`
  - `state_host(s)` is the office that owns the operational state for platform
    service instance `s`
- `service_manager : S -> O`
  - `service_manager(s)` is the office currently allowed to govern service
    instance `s`
- `parent : O -> O`
  - `parent(o)` is the office that currently governs office `o`
- `root : O -> O`
  - `root(o)` is the terminal office reached by repeated application of `parent`

`root(o)` is defined only if the office graph is acyclic. Acyclicity is therefore a required
invariant, not an optional optimization.

Base service invariants:

- `state_host(s) = service_host(s)`
- `service_manager(s) = root(service_host(s))`

This means service data stays with the host office, while management authority
follows the host office's current governance root.

Minimum platform service instance record:

```text
service_instance_id
service_kind
display_label
host_office_id
state_office_id
public_base_url
enabled
status
placement_generation
created_at
updated_at
```

Marketplace instance example:

```text
service_instance_id: marketplace-default
service_kind: marketplace
host_office_id: office-market-01
state_office_id: office-market-01
public_base_url: https://market.zebflow.com/api
```

## 4. Management Domain And Local Runtime Domain

For each office `o`, define two logical domains:

- `M(o)` = the management domain of office `o`
- `L(o)` = the local runtime domain of office `o`

`M(o)` governs mutation:

- source editing
- configuration changes
- project placement
- migration
- credential mutation
- MCP or other management actions

`L(o)` serves runtime:

- public pages
- APIs
- webhooks
- websocket or SSE
- runtime-served files and assets
- project-local state access
- office-local platform service APIs
- office-local platform service state access

This distinction is foundational.

Runtime must not require live controller availability in steady-state operation.

Platform service instances include:

- marketplace service
- future artifact registry
- future scheduler or queue service
- future model runner or agent execution service

These are not projects, even if their runtime packaging reuses project/app
deployment machinery internally.

## 5. Intrinsic Local Superadmin

Every office `o` has exactly one intrinsic local sovereign admin principal:

- `superadmin(o)`

`superadmin(o)` is part of the identity of the office.

It may be:

- active
- dormant

It is not deleted merely because the office becomes governed by another office.

## 6. Self-Controlled And Federated Offices

An office `o` is:

- **self-controlled** iff `parent(o) = o`
- **federated** iff `parent(o) != o`

Define effective mutation authority:

- `EMA(o) = M(root(o))`

Meaning:

- runtime authority is local to `o`
- mutation authority flows from the root office of the control tree containing `o`

## 7. Binding Versus Status

The model must separate:

- **binding**
- **status**

### 7.1 Binding

Binding is durable governance state.

Binding answers:

- which office governs this office?
- which office hosts this project?

Binding does not change merely because a process is offline.

### 7.2 Status

Status is ephemeral liveness or health state.

Status may be:

- `online`
- `offline`
- `degraded`
- `compromised`

Status does not by itself change binding.

This yields the core rule:

**Binding changes only by explicit mutation, never by timeout.**

## 8. Invariants

The following invariants define the intended contract.

### I1. Every Project Has Exactly One Host Office

For every `p in P`, `host(p)` exists and is unique.

There is no steady-state split-brain host for one project in the base model.

### I1a. Every Platform Service Instance Has Exactly One Host Office

For every `s in S`, `service_host(s)` exists and is unique.

There is no steady-state split-brain host for one platform service instance in
the base model.

The host office is authoritative for the service's local runtime and operational
state.

### I1b. Platform Service State Belongs To The Host Office

For every `s in S`, `state_host(s) = service_host(s)` in the base model.

Separating runtime and state host would require a future explicit contract. It
must not be implied by controller governance.

### I1c. Platform Service Management Follows Office Governance

For every `s in S`, `service_manager(s) = root(service_host(s))`.

If the host office is federated, its controller may manage the service. If the
host office detaches to self-control, service management becomes local to that
office.

### I2. Every Office Has Exactly One Parent Office

For every `o in O`, `parent(o)` exists and is unique.

An office is therefore either:

- self-controlled
- federated under exactly one parent office

### I3. The Office Control Graph Is Acyclic

There must be no cycle in repeated application of `parent`.

So `root(o)` is well-defined for every office.

### I4. Every Office Has An Intrinsic Local Superadmin

For every `o in O`, `superadmin(o)` exists.

This principal may be dormant, but the office must remain recoverable as a sovereign unit.

### I5. Local Management Is Active Iff The Office Is Self-Controlled

`M(o)` is active iff `parent(o) = o`.

If `parent(o) != o`, then `M(o)` is dormant and mutation authority comes from `EMA(o)`.

### I6. Runtime Availability Is Independent Of Controller Reachability

If office `o` is online, `L(o)` should continue serving its hosted projects even if `root(o)` is
temporarily unreachable.

The same rule applies to platform services hosted by `o`.

Controller failure degrades governance, not runtime service.

### I7. Platform Auth Is Mutation-Only

Platform-level users are required for mutation and management, not for steady-state runtime
service.

Application auth inside projects is a separate concern and may use arbitrary project-selected
backends.

### I8. Normal UI Must Not Rebind Offices

Rebinding an office to another parent office is a high-impact infrastructure mutation.

Therefore it must not be a casual web-UI action.

It belongs to CLI or equivalent machine-level execution paths.

### I9. Runtime Must Not Perform Per-Request Controller Lookups For Core Operation

Direct runtime handling must use local materialized state for:

- credentials
- DB connections
- project runtime config
- local membership or mutation cache where needed

Per-request controller lookups would violate runtime independence.

## 9. Failure Semantics

### 9.1 Controller Office Offline

If `root(o)` is offline but `o` is online:

- `L(o)` continues
- projects hosted by `o` continue serving
- platform services hosted by `o` continue serving from local materialized state
- `M(o)` remains dormant if `o` is federated
- new mutation from the controller path is unavailable
- binding remains unchanged

This is an unmanaged-but-still-serving state, not a detachment event.

### 9.2 Federated Office Offline

If federated office `o` is offline:

- its binding remains
- hosted projects on `o` are unavailable unless separately migrated
- controller records status as offline
- office may later return and resume under the same parent

### 9.3 Compromised Office

If office `o` is compromised, this is not a normal liveness case.

The correct response is:

- revoke trust
- rotate credentials
- mark status as compromised
- recover or replace the office

A compromised office is not merely an office that should be casually reattached elsewhere.

## 10. Mutation Operations

The model permits explicit high-impact operations.

### 10.1 `detach_to_self(o)`

Set:

- `parent(o) := o`

Effect:

- `o` becomes self-controlled
- `M(o)` becomes active
- governance no longer flows from the previous root

### 10.2 `attach_to_parent(o, x)`

Set:

- `parent(o) := x`

Preconditions:

- `x in O`
- acyclicity is preserved

Effect:

- `o` becomes federated under `x`
- `M(o)` becomes dormant

### 10.3 `reparent_only(o, x)`

This is equivalent to `attach_to_parent(o, x)` and changes only the direct parent of `o`.

Children of `o` remain children of `o`.

Therefore sovereignty of the whole subtree changes through `root`, but direct bindings of
descendants do not silently change.

This is the default safe operation.

### 10.4 `transfer_subtree(o, x)`

Rebind `o` and all descendant offices under a new root relationship according to an explicit
algorithm.

This is stronger than `reparent_only` and must be explicit.

It must never be inferred automatically from a parent office changing its own parent.

### 10.5 `move_service(s, o)`

Set:

- `service_host(s) := o`

Effect:

- platform service instance `s` is migrated to office `o`
- the old host stops being authoritative for `s` after successful cutover
- the new host owns runtime and operational state for `s`

This is distinct from moving a project and distinct from rebinding an office.
It must be explicit and journaled.

Minimum journaled operation kinds:

- `service.enable`
- `service.disable`
- `service.configure`
- `service.move_host`
- `service.rotate_secrets`
- `service.rebuild_public_projection`

### 10.6 `transfer_project_owner(p, new_owner)`

Reassign project `p` from its current owner principal to `new_owner` within the same host office.

This operation exists primarily for sovereignty recovery: when a controller office dissolves, the
local office detaches and reclaims self-control. Projects created by controller-side users are now
owned by shadow principals that no one can authenticate as. The local superadmin must be able to
reassign these projects to local users.

Preconditions:

- `new_owner` must be an active (non-shadow) user on `host(p)`
- the requesting principal must have superadmin authority on `host(p)`
- `host(p)` must be self-controlled (`parent(host(p)) = host(p)`)

Effect:

- all storage keyed by `(old_owner, project)` is re-keyed to `(new_owner, project)`
- file paths migrate from `users/{old_owner}/{project}/` to `users/{new_owner}/{project}/`
- URL structure changes from `/projects/{old_owner}/{project}` to `/projects/{new_owner}/{project}`
- credentials, connections, pipelines, and all project-scoped catalog records are re-keyed
- materialized runtime state (pipeline activations, caches) is refreshed under the new owner
- the shadow user may be cleaned up if no other projects reference it

This is the Mongol caravanserai scenario from section 2a: the building does not move, the trade
routes do not change, only the ownership record is updated.

Marketplace operation kinds include:

- `marketplace.publisher.create`
- `marketplace.publisher.update`
- `marketplace.publisher.disable`
- `marketplace.token.create`
- `marketplace.token.revoke`
- `marketplace.package.publish`
- `marketplace.package.unpublish`
- `marketplace.package.install`

## 11. Project Mobility Versus Office Sovereignty

Projects are portable across offices.

Platform service instances are portable across offices.

Offices are portable across controller relationships.

These are different operations and must remain distinct:

- project migration moves `host(p)`
- service migration moves `service_host(s)`
- office rebinding moves `parent(o)`

Conflating them would create avoidable ambiguity.

## 12. Platform Auth Versus App Auth

The model requires strict separation:

- **platform auth** governs mutation of offices and projects
- **app auth** governs end-user behavior inside a project

Platform auth should not be required for end-user runtime service.

App auth should not be assumed to share platform UUIDs or platform identity semantics.

## 13. Current Implementation Mapping

Current code approximates this model as follows:

- explicit process roles currently exist as `standalone`, `master`, and `worker`
- product-facing terminology is moving toward `controller` and `office`
- the current clustered slice already has office registration, project placement, and materialized
  runtime sync
- runtime execution can already happen directly on an office without controller participation in the
  request hot path
- marketplace is still being formalized as a platform service instance rather
  than a project-hosted authority

Known mismatch with the target model:

- Project Studio authoring is still controller-first in the current slice
- the stronger office-first authoring model is still to be completed
- marketplace persistence and placement are not yet fully modeled as
  office-hosted service state
- `transfer_project_owner` (10.6) is not yet implemented — after controller dissolution,
  projects owned by shadow users cannot be reassigned to local users
- `detach_to_self` (10.1) is modeled but the full sovereignty recovery workflow (detach +
  enumerate shadow-owned projects + transfer ownership) has no UI or CLI path yet

So the formal model in this document is the architectural target and compatibility contract.

## 14. Normative Summary

If one sentence must survive, it is this:

**An office serves locally, remains bound durably, and changes sovereignty only by explicit
machine-level mutation.**

Expanded:

**An office owns the local embodiment of what it hosts. Federation changes who
may govern that office; it does not make the controller the owner of the
office's project or service data.**
