# Office Federation Contract

> Status: draft, intended to become normative for multi-office `0.2.x`
> behavior.
>
> This document defines how offices bind, coordinate, and recover without
> corrupting project truth.

## 1. Purpose

The Office Federation Contract exists to make Zebflow scale from:

- one self-managed office on a laptop or Raspberry Pi
- one controller office with multiple managed offices
- Kubernetes multi-node deployments
- office-hosted platform services such as marketplace
- future agent-heavy offices with multiple managed execution capabilities

without changing the meaning of:

- office identity
- management authority
- project placement
- platform service placement
- mutation journaling
- failure and recovery

## 2. Normative Language

The words `Must`, `Must Not`, `Should`, and `May` are used normatively.

## 3. Core Principle

Communication failure **Must Not** corrupt project truth.

Loss of management connectivity degrades coordination, not project durability.

This means:

- runtime may continue while management is unavailable
- binding does not disappear because a heartbeat is missed
- retries and resumes must operate on explicit durable records

## 4. Entities

### 4.1 Office

An Office is one Zebflow installation.

Every office has:

- local runtime embodiment
- local project storage
- local project execution
- local platform service storage for services it hosts
- local platform service execution for services it hosts
- a management role that is either sovereign or delegated

An office is therefore not only a place for projects. It is the local Zebflow
embodiment for hosted projects and hosted platform service instances.

### 4.2 Managing Office

A Managing Office is the office that currently governs another office’s
management domain.

### 4.3 Managed Office

A Managed Office is an office whose management domain is governed by another
office.

### 4.4 Self-Managed Office

A Self-Managed Office is an office whose managing office is itself.

### 4.5 Project Host Office

Each project is hosted by exactly one office in the base `0.2.x` model.

That office is the embodiment of the project runtime and local workspace.

### 4.6 Platform Service Instance

A Platform Service Instance is a platform-level service embodied in one office.

Examples include:

- marketplace
- future artifact registry
- future scheduler or queue
- future model or agent runner

A platform service instance is not a project, even if implementation reuses
project/app deployment machinery internally.

Minimum identity fields:

- `service_instance_id`
- `service_kind`
- `display_label`
- `host_office_id`
- `state_office_id`
- `public_base_url`
- `enabled`
- `status`
- `placement_generation`
- `created_at`
- `updated_at`

For the default marketplace service:

```text
service_instance_id: marketplace-default
service_kind: marketplace
host_office_id: {office_id}
state_office_id: {office_id}
public_base_url: https://market.zebflow.com/api
```

`host_office_id` and `state_office_id` **Must** be equal in the base `0.2.x`
model.

### 4.7 Platform Service Host Office

Each platform service instance is hosted by exactly one office in the base
`0.2.x` model.

That office is the embodiment of the service runtime and operational state.

### 4.8 Platform Service Management Binding

The managing office for a platform service instance is the managing office of
the service host office.

Formally:

```text
service_manager(s) = root(service_host(s))
```

This means a controller governs a platform service while it governs the office
that hosts the service. The controller does not own the service state unless it
is also the service host office.

## 5. Identity Fields

Every office **Must** expose stable identity fields:

- `office_id`
- `display_label`
- `advertise_url`
- `version`
- `contract_version`
- `capabilities`

Future versions **May** add more identity fields, but these meanings **Must**
remain stable.

## 6. Binding vs Status

Binding and status are separate.

### 6.1 Binding

Binding is durable governance state.

Binding answers:

- who manages this office?
- which office hosts this project?
- which office hosts this platform service instance?

Binding changes only through explicit mutation.

### 6.2 Status

Status is temporary operational state.

Minimum statuses:

- `online`
- `offline`
- `degraded`
- `compromised`

Status informs operations. It does not redefine sovereignty.

## 7. Binding Rule

Every office **Must** have exactly one managing office binding.

An office is therefore either:

- self-managed
- managed by exactly one other office

Heartbeat loss **Must Not** make an office “free”.

An office remains bound until:

- explicit detach
- explicit reattach
- explicit reassignment
- explicit disaster recovery takeover

## 8. Runtime Independence Rule

If an office remains healthy, its hosted projects **Must** continue serving even
if its managing office is unavailable.

This includes:

- public pages
- APIs
- webhooks
- websocket or SSE
- runtime-served files
- project-local state access
- app-level authentication handled by the project itself
- office-hosted platform service APIs
- office-hosted platform service state access

This does not include:

- new federation-level mutation
- cross-office orchestration
- office reassignment
- service reassignment
- controller-only management UI

## 9. Management Scope Rule

Management authority for a managed office is defined by its managing office.

Local runtime remains local.

This means:

- sovereignty is hierarchical
- runtime embodiment is local
- service embodiment is local
- local operators in a managed office act by delegation, not by intrinsic
  sovereignty

Federation changes management authority. It **Must Not** imply that hosted
project or hosted platform service data is moved into the managing office.

## 10. Actor And Capability Model

The federation must support more than one kind of actor.

Actors **May** include:

- human users
- service principals
- local office daemons
- AI agents
- future research or engineering execution principals

The contract therefore distinguishes:

- actor identity
- capability ceiling
- execution request
- verification and trace outputs

Future AI-heavy offices with multiple agents **Must Not** bypass this model by
implicitly acting as root without capability declaration.

## 11. Capability Declaration

Any management or execution operation crossing office boundaries **Should**
declare:

- actor identity
- project scope
- requested capability
- operation intent

The receiving office **Must** be able to decide:

- whether the sender is recognized
- whether the office version supports the requested capability
- whether the project policy allows it

## 12. Version And Compatibility Handshake

Every managed office **Must** report at least:

- application version
- contract version
- supported capability set

The managing office **Must** evaluate compatibility before dispatching
operations.

Unsupported operations **Must** fail explicitly, not degrade into undefined
behavior.

This is especially important because not every office will auto-update in lock
step.

## 13. Operation Journal

Any controller-to-office mutation that matters operationally **Must** be backed
by a durable operation record.

Minimum record fields:

- `operation_id`
- `operation_kind`
- `owner`
- `project`
- `service_id` if applicable
- `service_kind` if applicable
- `source_office_id`
- `target_office_id` if applicable
- `status`
- `current_step`
- `last_error`
- `retry_count`
- `created_at`
- `updated_at`
- `completed_at`

Minimum statuses:

- `pending`
- `running`
- `failed`
- `completed`

The journal is required for:

- sync
- export
- import
- placement change
- service placement change
- service migration
- remote authoring mutation
- future migration or cutover actions

## 14. Resume And Retry Rule

Failed operations **Must** remain inspectable.

Retries **Should** resume from the last safe checkpoint where possible.

An operation **Must Not** vanish simply because a request path returned an
error.

This rule exists because distributed mutation is not reliable enough to treat
HTTP success/failure as the only system memory.

## 15. Materialization Contract

Runtime-critical project state sent from a managing office to a managed office
**Must** use explicit payloads.

Materialized categories **May** include:

- project runtime profile
- placement metadata
- repo bundle contents
- credential bindings or credential values if explicitly allowed
- DB connection definitions
- activation/bootstrap intent

Runtime **Must Not** depend on per-request calls back to the managing office for
core operation.

Hot-path data must already be local enough for the office to serve correctly.

## 15a. Platform Service Materialization Contract

Runtime-critical platform service state for a service hosted by an office
**Must** be local enough for that office to serve the service without per-request
controller lookups.

For marketplace, materialized state **May** include:

- marketplace service configuration
- publisher records
- token hashes and revocation state
- package metadata
- package version metadata
- artifact blobs
- media assets
- quotas
- audit records needed for local service operation

The managing office **May** hold inventory and replicated summaries, but those
summaries are not the runtime source of truth while the service is hosted by a
managed office.

## 16. Project Placement Rule

Each project has exactly one host office in the base `0.2.x` model.

The host office is authoritative for:

- live workspace
- local mutation execution for that project
- runtime execution
- project-local files and data

The managing office is authoritative for:

- placement policy
- inventory
- governance
- migration orchestration

## 16a. Platform Service Placement Rule

Each platform service instance has exactly one host office in the base `0.2.x`
model.

Formal functions:

```text
service_host : S -> O
state_host : S -> O
service_manager : S -> O
```

Base `0.2.x` invariant:

```text
state_host(s) = service_host(s)
service_manager(s) = root(service_host(s))
```

The host office is authoritative for:

- service runtime execution
- service operational database/storage
- service-served files and assets
- service-local audit required for operation

The managing office is authoritative for:

- service placement policy
- inventory
- governance
- migration orchestration

For marketplace, this means the selected host office owns the marketplace
database and artifact/media storage while it hosts the marketplace service.

### 16b. Platform Service Lifecycle

Platform service lifecycle states:

- `disabled`
- `provisioning`
- `enabled`
- `degraded`
- `migrating`
- `disabled_pending_cleanup`

Lifecycle transitions **Must** be explicit and journaled.

Minimum operations:

- `service.enable`
- `service.disable`
- `service.configure`
- `service.move_host`
- `service.rotate_secrets`
- `service.rebuild_public_projection`

Marketplace-specific operations:

- `marketplace.publisher.create`
- `marketplace.publisher.update`
- `marketplace.publisher.disable`
- `marketplace.token.create`
- `marketplace.token.revoke`
- `marketplace.package.publish`
- `marketplace.package.unpublish`
- `marketplace.package.install`

Each operation record **Must** include:

- `service_instance_id`
- `service_kind`
- `source_office_id`
- `target_office_id` when placement changes
- actor identity
- requested capability
- status
- audit timestamp

### 16c. Platform Service State Ownership

The host office owns the service's operational state while it hosts the service.

For marketplace, operational state includes:

- marketplace configuration
- publisher records
- token hashes and revocation state
- package records
- version records
- artifact blobs
- media assets
- quota counters
- service-local audit log

The managing office may keep:

- inventory
- health/status summaries
- placement generation
- replicated package summaries for management UI

Those replicated records are not the runtime source of truth.

## 17. Office-Truthful Studio Rule

If a project is hosted on a remote office, the visible Studio behavior **Must**
be truthful about that.

That means:

- remote writes must execute on the host office
- remote reads that present live project workspace state should come from the
  host office, not from a stale controller shadow copy

The managing office may remain the navigation lens, but it must not present
stale project truth as if it were authoritative.

## 18. Attach, Detach, And Reassignment

Attach, detach, and reassignment are infrastructure-level governance events.

They **Must Not** be treated as casual end-user web actions.

They **Should** be executed through:

- CLI
- operator workflow
- later controlled disaster-recovery procedures

Required operations:

- attach to manager
- detach to self-managed
- reattach to a new manager
- move project host office
- move platform service host office
- optional future subtree transfer semantics

## 19. Failure Semantics

### 19.1 Managing Office Offline

If the managing office is offline:

- managed offices remain bound
- managed offices may continue serving runtime
- managed offices may continue serving hosted platform services from local state
- federation mutation is suspended

### 19.2 Managed Office Offline

If a managed office is offline:

- manager records the office as unavailable
- binding remains unchanged
- pending mutation may remain queued or fail explicitly
- projects and platform services hosted by that office are unavailable unless
  explicitly migrated or failed over

### 19.3 Office Compromised

If an office is compromised:

- this is not a normal “owner-less” state
- it is an infrastructure compromise
- recovery must proceed through explicit revocation and replacement

## 20. Transport Neutrality

This contract does not require one transport.

It is compatible with:

- HTTP
- gRPC
- mTLS
- internal service mesh transport

But regardless of transport, the meaning of:

- identity
- status
- binding
- journaled mutation
- compatibility handshake

must remain the same.

## 21. Multi-Agent Future Compatibility

The federation contract must remain strong even when offices host:

- multiple AI agents
- research and engineering loops
- Python or C++ experiment runners
- project-specific autonomous automation

To stay adaptable, the contract should remain centered on:

- office identity
- actor capability declaration
- operation journaling
- execution observability

not on any one current execution engine.

## 22. Additive Extension Rule

Within `0.2.x`, new releases **May**:

- add identity fields
- add operation kinds
- add capability names
- add compatibility metadata
- add materialized payload fields

Within `0.2.x`, new releases **Must Not**:

- change binding semantics by timeout
- silently reinterpret status as sovereignty
- silently require hot-path controller lookups for core runtime
- silently make a host office non-authoritative for its own project workspace
- silently make a host office non-authoritative for its own hosted platform
  service state

## 23. Compatibility Examples

### Additive And Safe

- add a `supports_direct_runtime_ingress` capability
- add `contract_patch_version`
- add `verification_ref` to operation records
- add new operation kinds such as `runtime_promote`

### Breaking Unless Explicitly Migrated

- redefining `offline` to mean detached
- allowing two host offices for one project without new contract language
- requiring controller-only source workspace for remote projects
- making retries overwrite prior operation records without trace

## 24. Example Office Record

```json
{
  "office_id": "office-a",
  "display_label": "Office A",
  "advertise_url": "http://office-a.internal:10610",
  "version": "0.2.1",
  "contract_version": "office-federation.v1",
  "capabilities": [
    "project.sync",
    "project.export",
    "project.import",
    "project.authoring.remote"
  ],
  "status": "online"
}
```

## 25. Example Operation Record

```json
{
  "operation_id": "op_01",
  "operation_kind": "project.sync",
  "owner": "superadmin",
  "project": "example",
  "source_office_id": "controller-main",
  "target_office_id": "office-a",
  "status": "failed",
  "current_step": "materialize_credentials",
  "last_error": "remote office unavailable",
  "retry_count": 2
}
```

## 26. Compliance Test

An office federation implementation is compliant with this contract if:

1. binding and status remain separate
2. runtime can remain local when management is unavailable
3. mutations are journaled durably
4. compatibility is checked explicitly
5. host office truth is preserved for hosted projects and hosted platform
   services

That is the minimum stability bar for multi-office Zebflow.
