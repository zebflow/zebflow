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
- control contracts between offices

This stance avoids the conceptual weakness of `master/worker`, because offices are not merely
subordinate executors. An office hosts runtime, state, draft source, and project-local mutation.

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

This metaphor is useful for reasoning about authority transfer, detachment, and delegated local
administration. The formal contract, however, is the office/parent/root model defined in the rest
of this document.

## 3. Core Sets And Functions

Let:

- `O` be the set of offices
- `P` be the set of projects

Define:

- `host : P -> O`
  - `host(p)` is the office that currently hosts project `p`
- `parent : O -> O`
  - `parent(o)` is the office that currently governs office `o`
- `root : O -> O`
  - `root(o)` is the terminal office reached by repeated application of `parent`

`root(o)` is defined only if the office graph is acyclic. Acyclicity is therefore a required
invariant, not an optional optimization.

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

This distinction is foundational.

Runtime must not require live controller availability in steady-state operation.

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

## 11. Project Mobility Versus Office Sovereignty

Projects are portable across offices.

Offices are portable across controller relationships.

These are different operations and must remain distinct:

- project migration moves `host(p)`
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

Known mismatch with the target model:

- Project Studio authoring is still controller-first in the current slice
- the stronger office-first authoring model is still to be completed

So the formal model in this document is the architectural target and compatibility contract.

## 14. Normative Summary

If one sentence must survive, it is this:

**An office serves locally, remains bound durably, and changes sovereignty only by explicit
machine-level mutation.**
