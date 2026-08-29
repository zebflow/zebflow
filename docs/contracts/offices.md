# Offices Contract

Status: **review** — spec settled 2026-08-29, code catch-up owed.

How independent Zebflow instances relate to each other.

`distribution.md` governs how things move *between* instances.
This governs whether two instances are related *at all*.

## 1. The ground state

One instance is complete. Its own data root, catalogue, accounts, blessed
shelf, projects, and public surface. It needs nothing outside itself, and
nothing outside knows it exists. Five instances on two servers are five
ground states; sharing a machine relates them no more than sharing a continent.

Everything below is a response to one pressure and no other: **an operator who
administers several instances wants one identity and one view, without the work
leaving the instance that owns it.**

## 2. What an office is

An office is an instance that has accepted a coordinating authority — the
**capital**. It remains a full instance: its own data root, its own blessed
shelf seeded from its own binary, its own store, its own public surface. What
it accepts is narrow, and named in §4.

The capital has exactly three verbs:

| Verb | Meaning |
| --- | --- |
| **place** | create a project on a named office. The only reason cargo moves |
| **see** | a directory of offices and what each holds |
| **vouch** | one identity the offices accept for administration |

There is no fourth. No scheduling, no load balancing, no shared data, no
proxying.

## 3. The capital is never on the data path

Public traffic — webhooks, websockets, tiles, rendered pages, APIs — reaches
the office that runs the project, at that office's own base URL. The capital
carries none of it.

```
office99.example.com/wh/joseph/myblog/hook       one project
office42.example.com/wh/joseph/myblog/hook       a different project entirely
```

The office is the **host**, never a path segment. Two offices may hold the same
`owner/project` slug and never collide, because they were never reachable at
one address. Routes are unchanged by joining.

Consequences, recorded rather than discovered later:

- The capital's death costs logins, never execution. Every office keeps
  serving.
- A project's public address is its office's base URL, held in the office
  record.
- Moving a project between offices changes its public address. The remedy is a
  DNS name or a proxy in front, which is the operator's, not the platform's.
- The public cannot tell that two offices share a capital. Nothing about the
  relationship appears in a request.

## 4. The treaty

What an office keeps, and what it accepts. These are separable clauses; the
list is the whole agreement.

| Clause | Terms |
| --- | --- |
| Institutions | Kept. Every office seeds its own blessed shelf and data root, joined or not; identical bytes for the same release |
| Projects | Kept. Authored, held, and executed locally; the office is their address |
| Data | Kept. `data/store` and `files/` are the office's, always |
| Execution | Kept. What runs here keeps running when the capital is unreachable |
| Version | Kept. Offices need not agree; skew shows up only when two of them exchange something |
| Login | **Accepted.** Local accounts are disabled for login while joined; the capital's identity is the normal door |
| Placement | **Accepted.** The capital may create a project here |
| Exit | Defined, in §7. Nothing is destroyed by joining |

## 5. Joining

**A fresh office** — the ordinary case — joins by presenting its token. Nothing
is wiped, because there is nothing to wipe. Its local accounts, if any, are
disabled for login; they are not deleted.

**An office that already holds projects** may join, on one condition:

> Every local owner that owns anything must be explicitly mapped to a capital
> identity. No mapping, no join.

The mapping exists because one question has no machine answer: is office99's
`joseph` the same person as office42's `joseph`? A human states it. Both may
map to one capital identity, or to two. Nothing on disk moves: `users/{owner}/`
paths are untouched, and an unmapped owner is a refusal, never a guess.

This is the one rule every surveyed system reached by a different road — none
merges two account databases, because every merge policy is a privilege
escalation in one direction or a silent breakage in the other
(`surveys/multi-instance-join-models.md`).

## 6. Break-glass

An office's local authority is re-enabled by a command run **on its host**.
Filesystem access is the protection, as it is for `pg_hba.conf`.

- It re-enables local authority. Resetting a password is not enough: while
  joined the account is disabled, not merely unknown.
- It requires no quorum and no capital. An office whose capital is gone is
  never locked out of itself.
- Its use is recorded locally and reported to the capital on reconnect.

Without this clause §4's login term recreates the failure every centralised
identity system eventually hit: the credential needed to repair the
relationship is held by the party that is unreachable.

## 7. Leaving

Detaching is an act, not an accident. On detach the office keeps its projects,
data, files, shelf, and public surface, and its local accounts become live
again. It is a complete instance the moment it leaves, because it never stopped
being one.

This clause is why joining is safe to do at all.

## 8. Security terms

| Term | Rule |
| --- | --- |
| Token | Per-office, issued by the capital, revocable for one office alone. It carries the capital's CA digest so the office verifies the capital, and a secret so the capital verifies the office |
| Data path | The capital carries no request. Not a preference — the rule §3 exists to protect |
| Identity writes | Anything the capital pushes into an office's accounts is logged where the office's operator can read it |
| Owner mapping | Stated by a human at the door, never inferred |
| Break-glass | Host access, §6; recorded when used |

A shared environment secret is not a token: possession would be membership,
with no way to revoke one office.

## 9. Open

- **What the capital may write into an office's identity.** Accepted and logged
  today. A co-signature requirement, so a capital alone cannot insert an
  administrator, is the stronger form and is not built.
- **Remote placement.** The capital creating a project on a *remote* office,
  rather than the office it runs on, is unbuilt; so is remote rollback and
  remote provisioning of a public hub store (`stability-matrix.md` row 14).
- **Version skew between capital and office.** Undefined. Offices need not
  agree with each other; whether an office may be newer than its capital has no
  rule yet.
- **Directory freshness.** What the capital shows when an office has not
  reported recently, and whether stale entries are shown or hidden.
