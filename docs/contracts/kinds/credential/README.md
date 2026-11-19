# Credential

Status: **review** — spec settled 2026-08-29. The at-rest half ships: the `zfc1` envelope names its format and key, secrets are sealed at the storage boundary, and the keyring is versioned. The declaration half — `exposure`, inheritance along the type chain, unresolvable-type redaction, and the registration-time name check — is **deliberately deferred**, not owed by oversight; `exposure` appears nowhere in the code.

One stored credential: the secret-bearing value a project holds so a node can
authenticate. It is **not** the credential *type* definition, which a
[`NodeBundle`](../node-bundle/README.md) declares and the platform registry
merges.

Survey behind these decisions:
`surveys/credential-storage-practice.md` (maintainer notes, outside this repository).

## Identity

| | |
| --- | --- |
| Representation | database record; the secret half is ciphertext |
| Scope | one project |
| Adapter | `ProjectCredential`, `project_credentials` |

## The record

```json
{
  "owner": "joseph",
  "project": "myblog",
  "credential_id": "openai-main",
  "title": "OpenAI (production)",
  "kind": "openai",
  "fields": { "organization": "org-4" },
  "secret": "zfc1:3:base64ciphertext…",
  "notes": "billing account 4",
  "created_at": 1787175600,
  "updated_at": 1787175600
}
```

| Field | Rule |
| --- | --- |
| `credential_id` | stable, unique within the project |
| `kind` | the declared credential type this record fills |
| `fields` | the values the type declares public; stored as they are |
| `secret` | every other value, encrypted as one blob |
| `notes` | the operator's own note, not a secret |
| `managed_by` | the `zebflow_control` fence whose door created it (`app-control.md`); empty when a person made it. A person's edit keeps it |

Every create, change and delete is a line in the change record
(`project.md` § Change record): action `credential create|edit|delete`, path
`credential:<id>`, never a secret.

## Which fields are secret

A credential type declares each field:

```json
{ "kind": "postgres",
  "fields": {
    "host":     { "type": "string", "exposure": "public" },
    "port":     { "type": "number", "exposure": "public" },
    "password": { "type": "string" }
  } }
```

- **A field with no `exposure` is secret.** Declaring nothing is the safe
  direction; a type author must deliberately declassify.
- **Declarations merge along the type's inheritance chain**, so a composite
  type inherits its parents' markers rather than restating them.
- **An unresolvable type means everything is secret.** A bundle that was
  uninstalled, a record whose type is unknown: redact the whole record rather
  than returning what cannot be classified.
- **Names are checked at registration, never at runtime.** Registering a type
  with a field named like a secret (`*password*`, `*token*`, `*secret*`,
  `*key*`) that declares `exposure: public` is refused with the field named.
  The author overrides deliberately or fixes it. Name matching decides nothing
  once the type is registered.

## The `zebflow_control` fence

A `zebflow_control` credential is not a secret. It is the fence a
`zebflow.command.run` door works inside, kept as a credential so that only a
maintainer in Studio makes or changes it, and nothing a door does can widen
it. What each field means and every rule a door follows: `app-control.md`.

```yaml
scope: modules/            # files and pipelines are written only here
routes: /m/                # a webhook it makes answers only under this
tools: [file_write, pipeline_register, pipeline_activate, credential_create]
delegate: [pg_modules]     # credentials handed to the door, by id
nodes_mode: deny           # deny: all but `nodes` · allow: only `nodes`
nodes: []                  # kinds or families; empty in deny = the shared stores
manage_kinds: [postgres]   # kinds the door may create
manage_limit: 50           # optional; empty = no limit
record_as: site-a-app      # how the change record names it
```

Saved only when it draws a real boundary: `scope` a folder that is not the
root, `routes` a prefix that is not the whole site, `tools` from the closed
list, `nodes` naming real kinds or families, `manage_kinds` real kinds and
never `zebflow_control`, `credential_create` only with `manage_kinds`,
`manage_limit` from 1 to 10000. A refused fence is refused when it is saved.

Evidence: `zebflow::command` unit tests; `tests/platform/zebflow_command.rs`.

## Changing a credential

A save never drops a secret the caller did not name.

- **A read masks every secret** with `__zebflow_secret_unchanged__`; the mask
  sent back in a save keeps the stored value, and a mask with nothing stored
  behind it is refused.
- **A save with no `secret`** changes the title and notes and keeps the
  stored secret.
- **`PATCH /credentials/{id}`** changes only what it names: `secret` is a JSON
  merge patch (RFC 7396) onto the stored secret, where `null` removes a key;
  `{"secret": null}` is refused. A save holds a lock from its read to its
  write, so two at once cannot undo each other.

Evidence: `services/credential.rs` tests `a_save_without_a_secret_keeps_the_stored_one`,
`a_patch_changes_only_what_it_names`; `tests/platform/smoke.rs`
`platform_credential_change_keeps_the_signing_key`.

## At rest

The whole secret half is encrypted regardless of what the type declared. A
declaration mistake then costs display safety, never storage safety.

| Rule | |
| --- | --- |
| Cipher | an AEAD — the ciphertext authenticates, so a wrong key fails loudly instead of yielding garbage |
| Ciphertext | self-describing: it names the format that produced it and the key that made it. Never a bare blob whose algorithm is implied by the reader's version |
| Key | generated at first boot into the data directory, mode 0600, overridable by environment |
| Never | a default key compiled into the binary |
| Never | the key stored in the database beside the ciphertext |
| On a missing or wrong key | refuse to start. Never regenerate, never fall back to plaintext |

The ciphertext being self-describing is what lets the cipher change later
without stranding data: a reader that meets an older format tag knows what it
is holding instead of guessing from its own version.

For example, a format tag, the data key, and the AEAD output:

```
zfc1:3:base64( nonce ‖ ciphertext ‖ tag )
```

where `zfc1` pins both the envelope and the algorithm, so a later `zfc2` can
change cipher while `zfc1` bytes stay readable. XChaCha20-Poly1305 is the
recommended choice for `zfc1`: constant-time in pure software, so it does not
depend on AES hardware a Raspberry Pi may not have, and its large nonce makes
random nonces safe without keeping a counter. AES-256-GCM is a reasonable
alternative where that hardware exists. The contract requires the shape; the
named cipher is a recommendation.

**Key versioning exists from the first release.** The instance key wraps
versioned data keys, and every ciphertext names the key that made it. New
writes use the current key; older keys stay for reads.

This is not a future nicety. Rotation is the one feature nobody has been able
to add afterwards: Supabase hardcoded a key id and rotation became structurally
impossible; n8n's retrofit ships one-way, and disabling it makes data
permanently unreadable; Jenkins has no rotation at all.

Two operations, both online: **rotate** installs a new data key without
re-encrypting anything, and **rekey** re-wraps the data keys under a new
instance key. A separate sweep re-encrypts old ciphertext when an old key is to
be retired — rotation alone never does that.

## What this protects, and what it does not

It protects a **copied database**: a backup, a support bundle, a snapshot, a
misplaced volume. That is the ordinary way secrets escape.

It does not protect against filesystem access, process memory, or code running
inside Zebflow. The key must be readable by the process that reads the data, so
no local scheme can. This is stated here because a contract that implies
protection it cannot give is worse than one that admits the boundary.

**A node can read the credential it was given.** Someone who authors a node and
attaches a credential to it can print that credential. This is a trust boundary,
not a defect. Narrowing it — a type restricting which nodes may use it, a
credential restricting which hosts it may be sent to — is recorded below as
unbuilt.

## What never carries a value

- **Pipelines** store a credential id, never a value.
- **Exports and packages** carry a reference. A package declares the credential
  *kinds* it needs; it never carries a secret, and a project arriving with an
  unresolved reference gets a placeholder to fill, not a published pipeline.
- **Run history** never holds a secret: fields the type declares secret are
  dropped, and values actually injected during the run are masked before the
  log is written.
- **Operator backups** carry ciphertext by default. A decrypted export must say
  so in the flag itself and record that it happened.

## Rejections

An unknown `kind`. A record whose secret half is not ciphertext. A type
registration that declassifies a secret-looking field without an explicit
override. Reading a credential when the key is absent.

## Open

- **Narrowing runtime use.** A type restricting which node kinds may decrypt
  it, and a credential restricting which hosts it may be sent to, are the two
  mitigations the surveyed systems added after the fact. Neither is built.
- **Audit.** Every decryption should be recorded. Nothing records one today.
- **External key managers.** A pluggable backend for the instance key, never a
  requirement: a Raspberry Pi install must work with no external dependency.
- **Masking limits.** Value masking cannot catch a secret the pipeline
  transformed before printing, and short values are indistinguishable from
  ordinary text. The limits belong in the documentation beside the feature.
