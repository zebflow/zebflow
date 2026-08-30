# Federated Offices

Federated Offices is the multi-server operating model for Zebflow.

The point is not only scaling compute. The point is coordinating:

- runtimes
- placement
- office roles
- distributed execution surfaces

## What this means in practice

- one controller can understand multiple offices
- projects can be placed or pinned intentionally
- operations can expand beyond a single local runtime

This becomes important when Zebflow is used as an application platform across multiple servers rather than one local box.

## Joining: one token per office

An office joins by presenting a token the controller minted **for that office**.
There is no shared cluster secret: one secret held by every office would mean
revoking one office rotates all of them.

A token is shaped so it says what it is:

```
zfjoin1:<office_id>:<secret>
```

`zfjoin1` names the format, `office_id` names the holder, and `secret` is 32
random bytes. Anything else is refused, with a message naming the mint.

### Mint one (controller, superadmin)

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  -X POST http://controller:10610/api/cluster/join-tokens \
  -H "Content-Type: application/json" \
  -d '{"office_id":"office-a","label":"Office A","note":"sg-1 rack 4"}'
```

Minting creates the office record first, then the token, so the controller
knows who holds what before anything is presented. The plaintext token is in
the response **once** — the controller stores only its SHA-256 digest and can
never show it again. Re-minting for an office that already has one refuses
unless you pass `"rotate": true`, which invalidates the token that office is
using now.

List what has been issued (digests and status, never secrets):

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  http://controller:10610/api/cluster/join-tokens
```

### Give it to the office

```bash
ZEBFLOW_CLUSTER_MASTER_URL=http://controller:10610 \
ZEBFLOW_CLUSTER_ADVERTISE_URL=http://office-a:10610 \
ZEBFLOW_CLUSTER_JOIN_TOKEN=zfjoin1:office-a:… \
  zeb office
```

The office writes the token to `<data-root>/platform/office-join-token`
(mode 0600) and reads it from there on every later start. The variable is only
needed for the first join. If the variable and the stored file disagree, the
office **refuses to start** and names both offices rather than overwriting
either — two different tokens on one office means somebody is wrong about which
office this is.

An office that was not given a node id takes the one inside its token. If you
set `ZEBFLOW_CLUSTER_NODE_ID` to something else, the controller refuses the
registration: a token minted for one office presented by another is exactly
what the office id inside the token exists to catch.

### Both sides verify

The office sends a fresh random nonce with each registration, and the
controller answers with an HMAC over it keyed by that office's own secret
digest. A host that does not hold the token cannot produce it, so an office
refuses a registration response that does not verify and does not begin
heartbeating. The controller proves itself the same way on every internal call
it makes to an office.

### Revoke one office

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  -X POST http://controller:10610/api/cluster/join-tokens/office-a/revoke
```

That office's next register or heartbeat is refused with
`CLUSTER_JOIN_TOKEN_REVOKED`; every other office is untouched. Nothing already
running on the revoked office is killed — an office keeps executing its own
projects whether or not it has a controller — it simply stops being a member,
and stops appearing fresh in the directory. The record is kept rather than
deleted, because it is the record of who held what.
