# ProjectAccess

Status: **review** — spec settled 2026-09-08, code caught up 2026-09-08.

Who may open a project, and what they may do inside it. One subject, one role,
resolved to capabilities the routes compare against.

## Identity

| | |
| --- | --- |
| Stored in | platform metadata: `project_members`, `project_invites`, `project_policies`, `project_policy_bindings` |
| Adapter | `src/platform/services/access/` — `roles.rs`, `membership.rs`, `invite.rs` |
| Gate | `require_project_api_capability` in `src/platform/web/mod.rs` |

## The four layers

```
Capability   30 atoms          what a route compares against
   ↓ bundled into
Policy       a named set       stored per project, `managed` if preset
   ↓ named as
Role         5 presets         what a person picks
   ↓ attached by
Binding      subject → policy  what resolution walks
```

A capability is the only thing enforced. A role is a policy with a familiar
name. Nothing else is consulted at a route.

## Roles

Cumulative: each rung is the rung below plus its own additions, so a capability
is written once and inherited upward.

| Role | Adds | Total |
| --- | --- | ---: |
| `guest` | read the project, templates, pipelines, files, libraries, members | 6 |
| `reporter` | read data and settings | 8 |
| `developer` | write templates, pipelines and files; **run pipelines** | 20 |
| `maintainer` | members, credentials, **write data**, libraries, settings, MCP sessions | 29 |
| `owner` | delete the project | 30 |

Two lines are deliberate:

- **`developer` stops short of `tables.write`.** Someone can write a pipeline
  that changes data without being able to edit rows by hand.
- **`owner` is `project.delete` and nothing else.** Before it existed the two
  top rungs resolved to the same set, because deleting a project was guarded by
  comparing the session's name to the namespace owner — a check outside the
  policy system that no binding could grant and no role could describe.

`role_capabilities` is the only place a rung's contents are written. Four tests
hold the shape: the ladder never loses a capability going up, every capability
is reachable by some role, each rung grants strictly more than the one below,
and no role can grant above itself.

## Joining

Invited and accepted, never added outright. A member's git identity ends up on
commits made in the project, so joining is something a person agrees to.

```
                 invite                accept
   maintainer ───────────▶ pending ───────────▶ member
                              │
                              ├── decline ────▶ revoked
                              ├── revoke  ────▶ revoked
                              └── lapse   ────▶ expired
```

An invite names an existing user. There is no email delivery, so a person must
have an account before they can be invited into a project.

### Accepting is two steps, in this order

The member row is written **first**, then the invite is marked accepted. A
member without a recorded answer is recoverable — the invite still reads
pending and accepting is idempotent. An accepted invite without a member is
not: the invitation is spent and the person can neither join nor try again.

### Nobody grants above themselves

`can_grant(actor, granted)` refuses a role above the actor's own rung, on both
`upsert_member` and `create_invite`. Without it, anyone holding `members.write`
— which is `maintainer` — could mint an owner and then be made one.

## Seeing what you joined

`list_projects(owner)` answers *which projects can this person open*, not
*which does this person own*. It unions the namespace's own projects with every
`project_members` row for that user. Ownership alone left an accepted
invitation reachable by URL and absent from the only page that lists projects.

## Surface

```
GET    /api/projects/{o}/{p}/members             members.read
POST   /api/projects/{o}/{p}/members             members.write   upsert, role change
DELETE /api/projects/{o}/{p}/members/{user}      members.write
GET    /api/projects/{o}/{p}/invites             members.read
POST   /api/projects/{o}/{p}/invites             members.write
DELETE /api/projects/{o}/{p}/invites/{id}        members.write

GET    /api/invites                              session-scoped
POST   /api/invites/{o}/{p}/{id}/accept          the invitee only
POST   /api/invites/{o}/{p}/{id}/decline         the invitee only
```

Answering is session-scoped rather than project-scoped: the invitee is not yet
a member, so a project capability check would refuse the very person the
invitation is for. An invite addressed to someone else answers `404`, not
`403` — it is not that person's to know about.

## Deliberately absent

- **No deny rule.** Permissions are additive, as in Kubernetes RBAC. AWS's
  explicit deny across five policy types is why that model needs a simulator to
  answer "what can this user do".
- **No per-resource scope.** A capability holds across the whole project. n8n
  reached the same conclusion and answers "which workflow" with *use another
  project*. A `scope` field can be added to a binding later without changing
  anything here.
- **No per-capability editing.** The 30 atoms are the mechanism; the 5 roles are
  the product. `ProjectPolicy.managed` already separates a preset from a
  hand-built policy, so that screen can arrive with no migration.
