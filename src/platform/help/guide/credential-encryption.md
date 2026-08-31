# Credential encryption

Every credential a project stores is encrypted at rest. This page is what an
operator needs: where the key is, what happens when it is gone, and the two
commands that change keys without downtime.

## What is protected, and what is not

Encryption here defends **a copied database** — a backup, a support bundle, a
snapshot, a misplaced volume. That is the ordinary way secrets escape.

It does **not** defend against filesystem access, process memory, or code
running inside Zebflow, because the key has to be readable by the process that
reads the data. A node you attach a credential to can print that credential.
That is a trust boundary, not a defect.

## Where the key is

```
<data-root>/platform/credential-key      mode 0600, generated on first boot
```

One file. It is **not** in the database, and it is not compiled into the binary.
Back it up separately from the database, and keep the two apart — a backup that
holds both is a backup with the lock taped to the door.

`ZEBFLOW_CREDENTIAL_KEY` supplies the same value from the environment instead,
for deployments that inject secrets rather than mount files. If the variable and
the file both name a key and they disagree, the instance **refuses to start**
and overwrites neither: one of them opens your credentials and the other does
not, and guessing which is not something a program should do on your behalf.

## Losing the key

There is no recovery. Nobody offers one — this is the single largest support
burden in self-hosted platforms, and every project's answer is the same: back
the key up separately.

What Zebflow does instead is **fail early and loudly**. An instance that has
key generations recorded and no key file refuses to start:

```
this instance has 1 credential encryption key generation(s) recorded in its
catalog and no instance key at '…/platform/credential-key'. Restore the key file
from your backup, or set ZEBFLOW_CREDENTIAL_KEY to the key it held. A credential
cannot be regenerated, so this instance will not start a new key over the old
one — the credentials it holds would become permanently unreadable.
```

A key that is present but wrong gets the same treatment, because the ciphertext
authenticates: a wrong key fails, rather than producing garbage that fails much
later somewhere confusing.

**A genuinely new instance starts normally.** The evidence a key ever existed is
the keyring in the catalog, not the credentials — so a fresh install generates
one, and a database restored without its key file refuses even while it holds
nothing to decrypt, which is the moment you can still go and find the file.

## What is stored

```
"zfc1:2:<nonce ‖ ciphertext ‖ tag>"
```

`zfc1` names the format and `2` names the key generation that produced it. Both
are there so a later format can change the cipher while today's bytes stay
readable.

The **whole** secret half is encrypted, whatever a credential type declared
about any individual field. A type author who marks a password as public costs
themselves display safety and never storage safety.

## Rotating keys

Two operations, both online, both superadmin, neither requiring a restart.

**Rotate** installs a new data key. New credentials are written under it;
everything already stored stays readable under the key that wrote it. Instant,
and it re-encrypts nothing.

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  -X POST http://localhost:10610/api/admin/credentials/rotate
```

**Re-encrypt** is the separate sweep that rewrites every stored credential under
the current generation. Run it when you want an older generation to stop being
needed — rotation alone never does that.

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  -X POST http://localhost:10610/api/admin/credentials/reencrypt
```

**Rekey** replaces the instance key file. The data keys are re-wrapped under the
new one and no credential is touched, so it is cheap however many you hold.
Back up the new file afterwards: the old one no longer opens anything.

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  -X POST http://localhost:10610/api/admin/credentials/rekey
```

Rekey is refused when the key comes from `ZEBFLOW_CREDENTIAL_KEY`, because the
instance does not own the file it would rewrite.

**Where the keyring stands** — never key material, only which generations exist:

```bash
curl -H "Cookie: zebflow_session=superadmin" \
  http://localhost:10610/api/admin/credentials/keyring
```

## Credentials written before this release

They were stored in the clear. They are read as they are, and rewritten as
ciphertext the next time anything changes them — a credential cannot be
regenerated, so refusing one would destroy something you cannot replace. The
re-encrypt sweep converts the rest in one call, and is the way to be sure none
are left.
