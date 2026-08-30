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
zfjoin2:<office_id>:<controller_verify_key>:<secret>
```

`zfjoin2` names the format, `office_id` names the holder, `controller_verify_key`
is the controller's Ed25519 public key, and `secret` is 32 random bytes.
Anything else is refused, with a message naming the mint.

The verification key is in the token because the office has to be able to tell a
real controller from anything that answers the controller's URL, and it has to
be able to do that offline. The controller keeps the private half at
`<data-root>/platform/cluster-signing-key` (mode 0600) and never publishes it:
what an office stores verifies a controller and forges nothing, and what a
controller stores about an office forges nothing either.

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
never show it again. That digest never leaves storage either: it is not in the
mint response and not in the listing. Re-minting for an office that already has one refuses
unless you pass `"rotate": true`, which invalidates the token that office is
using now.

List what has been issued (office, status, and timestamps — never a secret and
never the stored digest):

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  http://controller:10610/api/cluster/join-tokens
```

### Give it to the office

```bash
ZEBFLOW_CLUSTER_MASTER_URL=http://controller:10610 \
ZEBFLOW_CLUSTER_ADVERTISE_URL=http://office-a:10610 \
ZEBFLOW_CLUSTER_JOIN_TOKEN=zfjoin2:office-a:… \
  zeb office
```

The office writes the token to `<data-root>/platform/office-join-token`
(mode 0600) and reads it from there on every later start. The variable is only
needed for the first join — a joined office restarts with it unset, so a reboot
needs no operator. If the variable and the stored file disagree, the
office **refuses to start** and names both offices rather than overwriting
either — two different tokens on one office means somebody is wrong about which
office this is.

An office that was not given a node id takes the one inside its token. If you
set `ZEBFLOW_CLUSTER_NODE_ID` to something else, the controller refuses the
registration: a token minted for one office presented by another is exactly
what the office id inside the token exists to catch.

### Both sides verify

The two directions carry different halves of the credential, deliberately.

The office proves itself by presenting its token; the controller compares
`sha256(secret)` against what it stored, so reading the controller's database
yields a value that cannot be presented back as a secret.

The controller proves itself by **signing**. The office sends a fresh random
nonce with each registration, and the controller answers with an Ed25519
signature over it — over the office id, the office's current token fingerprint,
and the nonce — which the office checks with the public key its token carried. A
host that does not hold the controller's private key cannot produce it, *even if
it has read every byte the controller stores about that office*, so an office
refuses a registration response that does not verify and does not begin
heartbeating. The controller proves itself the same way on every internal call
it makes to an office.

That asymmetry is the point. If the controller proved itself with something it
also stored, anyone who read `office_join_tokens` — or a mint response, or a
proxy log — could mint a vouch naming any identity at any office.

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

## While joined, the local door is closed

An office that holds a join token refuses local password login. That is the one
thing joining costs, and it is the whole of it:

```
$ curl -i -X POST http://office-a:10610/login -d 'identifier=superadmin&password=…'
HTTP/1.1 403 Forbidden

local password login is disabled on this instance. It has joined a controller
as office 'office-a', and while it is joined the controller's identity is the
way in: open this office from the controller's directory, or ask it for a
vouch. Local authority can be re-enabled from this host, with no controller and
no quorum, by stopping this server and running `zeb admin break-glass`;
`zeb admin detach` leaves the controller for good and keeps every project, its
data, its files, and its shelf.
```

The accounts are **disabled, not deleted**. Nothing is removed by joining and
nothing has to be recreated by leaving.

The refusal is the same words for a correct password, a wrong one, and a name
this office has never held, because the check runs before the credential is
read. A caller who cannot get in learns nothing about who is here.

**The state is on disk, not in the command line.** "Joined" means
`<data-root>/platform/office-join-token` exists — the same file membership
itself reads — so restarting a joined office as plain `zeb` does not reopen the
local door. Starting in a different mode is not leaving, and leaving has its own
command.

## Break-glass: getting back in from the host

```bash
# on the office's host, with the office stopped
zeb admin break-glass                # re-enable local authority
zeb admin break-glass superadmin     # and rotate that account's password
```

It needs no controller and no quorum. Filesystem access is the protection, the
way it is for `pg_hba.conf`, which is why this is a command on the host and not
a route: a route would be reachable by exactly the population the closed door
excludes, and would need the credential that is missing.

Two rules worth knowing before you need it:

- **It does not detach.** The office stays joined: the controller may still
  place projects here and still vouches for identities here. Break-glass changes
  who may open the door, not who the office belongs to. `zeb admin detach` is
  what leaves.
- **It does not resurrect controller-created accounts.** An account that exists
  only because a vouch created it has no usable password by construction, and
  re-enabling local authority does not give it one. Naming such an account as
  the argument is refused; break the glass on one of this office's own accounts.

A break-glass is recorded in the office's own catalog and never expires there:

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  http://office-a:10610/api/office/local-authority
```

which also answers the state question — `joined`, `local_login_allowed`, and
which recorded act is currently in force.

On the next successful registration the office reports it to the controller,
where it lands at:

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  http://controller:10610/api/cluster/office-break-glass
```

The office re-sends anything still unacknowledged on every registration cycle,
so an office offline for a month reports on the first reconnect. An office that
never reconnects keeps the record locally and says so — `reported_at` stays `0`.
The local record is the authority; the controller's copy is what it was told.

A break-glass is scoped to the membership it was performed against. Re-issuing
that office's token (`"rotate": true`) gives it a new secret, and the old record
no longer matches, so the new join has its local door closed again with nobody
having to remember to clear anything.

## Leaving: detach

```bash
# on the office's host, with the office stopped
zeb admin detach
```

The office keeps its projects, `data/`, `files/`, its blessed shelf, and its
public surface, and its local accounts go live again. It is a complete instance
the moment it leaves, because it never stopped being one — removing the join
token is the entire act.

Detach is an **office-side** command, and only that. The controller has three
verbs — place, see, vouch — and there is no fourth; and an office whose
controller has been destroyed must still be able to leave, which a controller
verb could not do. What the controller has is revocation, which stops membership
without touching the office's own accounts.

Two things to do after detaching:

- unset `ZEBFLOW_CLUSTER_JOIN_TOKEN`, or the variable joins the instance
  straight back on its next start
- revoke that office's token on the controller, since detaching is the office's
  act and the controller still holds its half of the record

A detach is recorded locally and is never reported: reporting it would need the
credential that detaching gives up. The controller learns the same fact from the
office ceasing to heartbeat.

## Reaching an office: the vouch

An operator authenticated on the controller reaches any of its offices without
knowing a password there. This is the controller's third verb — it vouches for
one identity, and the office accepts it.

From the controller's home page, an office card carries an **Open office**
button. That is the whole flow: click it and you land on that office, signed in.

Behind the button, the controller mints a vouch and redirects to it:

```text
zfjoin2v:<office_id>:<identity>:<expires_at>:<nonce>:<proof>
```

`proof` is the controller's Ed25519 signature over the other four fields **and**
over that office's current token fingerprint, under a third scheme word so
nothing captured in one direction replays in another. Every field is signed, so
the office id, the identity, and the expiry cannot be edited. The office
verifies it with the public key its join token carried and calls nobody, which
is why a vouch still works when the controller is gone.

Because the fingerprint is in the signature, **rotating** an office's token
invalidates every vouch minted under the old one, at that office,
cryptographically. Revoking without rotating stops the controller minting new
ones but leaves an already-minted vouch redeemable for the rest of its 120
seconds; if you need an office shut out now and cannot reach it, rotate.

### Mint one by hand

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  -X POST http://controller:10610/api/cluster/offices/office-a/vouch
```

The response carries the vouch and a `redeem_url` on that office. The vouch
names **your** session's identity; there is no field for naming somebody else.

### Redeem it

```bash
curl -i -X POST http://office-a:10610/api/office/vouch \
  -H "Content-Type: application/json" \
  -d '{"vouch":"zfjoin2v:office-a:…"}'
```

The office answers with an ordinary session cookie — the same one `POST /login`
issues there — so everything downstream is unchanged. The browser path is
`GET /office/vouch?v=…`, which is what the redirect uses.

### What the office checks, and what it does not

The office verifies the vouch entirely with the secret it already holds. **It
does not contact the controller**, and a vouch works with the controller down.
That is deliberate: an office is never locked out of itself by the unavailability
of the party that would repair the relationship.

It refuses a vouch that:

- names a different office (`CLUSTER_VOUCH_OFFICE_MISMATCH`)
- does not verify under its own secret (`CLUSTER_VOUCH_INVALID`)
- has expired (`CLUSTER_VOUCH_EXPIRED`) — a vouch lives **120 seconds**,
  because it is a hand-off and not a session
- has already been spent (`CLUSTER_VOUCH_ALREADY_REDEEMED`)

A vouch opens exactly one session. The spent nonce is recorded in the office's
own catalog and pruned once it passes its own expiry, so the record cannot grow
without bound.

### Revoking an office also stops vouching for it

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  -X POST http://controller:10610/api/cluster/join-tokens/office-a/revoke
```

The next mint for that office is refused with `CLUSTER_JOIN_TOKEN_REVOKED` —
the same record and the same status check that stops a heartbeat. Note that a
vouch minted *just before* the revoke stays redeemable at that office until it
expires, at most 120 seconds later, because the office learns nothing new until
it talks to the controller. Re-minting with `"rotate": true` closes it
immediately and cryptographically: every controller proof is signed over that
office's *current* token fingerprint, so an office running the old token
computes a different one and nothing minted under the new membership verifies
there, and nothing minted under the old one verifies once the office is
re-issued. **If you need an office shut out now and cannot reach it, rotate
rather than revoke.**

### What was written into this office's accounts

If the vouched identity has no local account, the office creates one. That is an
identity write, and every one of them is logged where **this office's** operator
can read it, with the controller uninvolved:

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  http://office-a:10610/api/office/identity-writes
```

Each row records the owner, whether it was `created` or `linked` to an account
that already existed, the role it holds, and the reason. Two rules the log makes
visible:

- an account created by a vouch gets **no usable password**. It is reachable by
  vouch and by nothing else, so it never becomes a local back door.
- an account that already existed keeps the role this office gave it. The
  controller vouches for *who* somebody is; what they may do here stays the
  office's own statement.
