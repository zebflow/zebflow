# HubPackage

Status: **Review**

This contract defines one published package and its content manifest. It is the
document every distribution channel carries: a Hub asset, a remote pack, a file
supplied directly, and a static repository entry all move a `HubPackage`.

Because `NodeBundle` is frozen and its install path decodes one of these, this
contract is load-bearing for a frozen kind and its shape cannot drift.

See [`../../distribution.md`](../../distribution.md) for the scope, verbs,
channels, and trust model this contract sits inside. Those are settled and are
not re-litigated here.

## Decisions taken

### Releases are immutable

A published `package@version` is fixed. Correcting anything — content, title,
description — requires a new version.

Every package registry converged on this: npm, Cargo, Maven Central, NuGet, and
Go modules all forbid republishing a version, because a mutable release makes a
lockfile a lie. Zebflow already assumes it: `zeb.lock` records a digest, and a
digest only means something if the thing it names cannot change underneath it.

The cost is accepted: a typo in a description costs a version bump.

### A file is carried or referenced, never both

Each entry in `spec.files` supplies exactly one of:

| Field | Meaning |
| --- | --- |
| `content` | the bytes, inline in the document |
| `artifact` | a digest the bytes are fetched and verified against |

```json
{ "rel_path": "definition.json", "kind": "node_definition",
  "size_bytes": 5120, "reason": "package",
  "content": "{ … }" }

{ "rel_path": "wasm/core.wasm", "kind": "asset",
  "size_bytes": 33554432, "reason": "package",
  "artifact": { "sha256": "9f2b…c14a", "media_type": "application/wasm" } }
```

Neither field, or both, is rejected. This is the same either-or rule the
`NodeBundle` run binding uses.

**Why not one of the two established shapes.** Comparable systems split into two
camps. NuGet, VSIX, Debian, RPM, and Helm ship one self-contained archive,
optimising for a single thing to move, sign, and trust. OCI, npm, Maven, Cargo,
and Go split a small manifest from blobs addressed by digest, optimising for
size and reuse. Notably, **none of them encodes binaries into a text manifest**:
the archive camp uses binary containers, and the reference camp uses references.

Zebflow's packages are bimodal in a way that fits neither camp cleanly. A
composite bundle is JSON, SVG, and `.zf.json` — all text, all small, and worth
keeping readable, diffable, and committable to a static repository as text. A
WASM module is neither. Helm reached the same split from the same shape: a small
text chart that references heavy images externally.

Base64 in JSON costs 33%, which is why the 25 MB remote artifact cap is really
an 18 MB content limit, and why a 32 MB module cannot be distributed today at
all.

### An artifact reference carries a digest, not a URL

The location comes from the channel the package is being installed through:

```text
Hub asset          <hub>/artifacts/<sha256>
Static repository  <repo-base>/artifacts/<sha256>
Local file         ./artifacts/<sha256> beside the document
```

A package copied to a different repository still resolves, and two packages
shipping the same runtime fetch it once. A URL would bind a package to where it
was first published.

Install fetches every referenced artifact, verifies each digest, and writes
nothing until all of them pass, so a failed fetch leaves the previous state
intact.

## Boundary table

| Role | Where | What it does |
| --- | --- | --- |
| Contract adapter | `src/contracts/kinds/hub_package.rs` | `type Spec = HubPackageSpec`; typed, `deny_unknown_fields`, bounded, and enforces the carried-or-referenced rule |
| Artifact location | `HubArtifactChannel` | the channel's answer to "where are the referenced bytes": a local base directory, or a refusal that says why |
| Reader, artifact | `HubArtifactChannel::resolve` | reads `<base>/artifacts/<sha256>`, checks the declared size, and verifies the digest |
| Writer, artifact | `HubService::store_artifact` | puts bytes into this instance's Hub store, content-addressed, and returns the digest |
| Writer, publish | `publish_asset` | builds a manifest, writes an artifact file, records a version row; refuses a version that already exists |
| Writer, encode | `encode_hub_artifact` | wraps a spec in the envelope |
| Reader, bytes | `parse_hub_artifact_bytes` | Hub store and remote pack |
| Reader, value | `parse_hub_artifact_value` | direct payload, including local node bundle install |
| Reader, remote publish | `import_remote_asset` | validates an inbound published document |
| Reader, API | `web/mod.rs:7488` | serves the document |
| Durable store | `data_root/<artifact_rel_path>` plus a `HubAssetVersion` row | one file per version, digest recorded as `artifact_sha256` |

## Findings

### The contract was empty — fixed

`HubPackageContract` declared `type Spec = Value` and its validator only checked
that the spec was a JSON object. Every rule about what a package contains lived
in `HubArtifact`, a private struct in the hub service that the contract never
reached, so the registered kind provided no guarantee at all.

This was the same defect the `NodeBundle` review found in reverse: there, the
rules existed and were scattered; here, the kind existed and the rules did not.
Giving `HubPackage` a typed spec was the substance of this review.

The shape now lives in the contract as `HubPackageSpec`, `HubArtifact` is gone,
and the hub service decodes into the contract type. One rule found on the way
out: an entry could declare any `encoding` it liked. The package safety review
reads a file through `entry_text`, which returns nothing for an encoding it does
not recognise, while the installer writes the entry's bytes verbatim. Labelling
a file `utf8` therefore skipped the whole review and still landed on disk. The
accepted set is now closed to `text`, `base64`, and absent.

### A referenced artifact could not be installed — fixed for local channels

The format accepted a reference and the installer refused one, so the honest
half of the decision was in place and the useful half was not. Installation now
resolves references, and the location comes from the channel rather than the
document: `HubArtifactChannel` is either a base directory holding
`artifacts/<sha256>` or an explicit refusal carrying the reason.

| Channel | Base it resolves against |
| --- | --- |
| Hub asset | `data_root/services/<instance>/artifacts/<sha256>` |
| Local file | `artifacts/<sha256>` beside the document |
| Remote pack, project bundle over HTTP | refused: `HUB_ARTIFACT_UNRESOLVED` |
| A document supplied as a request body | refused: it names no location |

Every reference is fetched, sized, and hashed in
`prepare_hub_install_entries`, which already ran to completion before the first
write existed. Nothing is staged, copied, or rolled back for a bad artifact,
because nothing was written: a mismatch or a missing file returns before the
install touches the project at all. The remote paths that cannot fetch check
their entries before clearing anything, so a refusal there does not empty a
worktree it was about to fill.

Refusal is deliberate rather than incidental. HTTP fetching of artifacts has no
endpoint, no cache, and no size streaming yet, and a channel that silently wrote
an empty file would be worse than one that says it cannot.

### Republishing silently overwrote — fixed

`publish_asset` built a `HubAssetVersion` and called `put_hub_asset_version`
without checking whether that `package_id` and `version` already existed. A
second publish of the same version replaced the row, the artifact path, and the
digest. Any `zeb.lock` entry pinning that version then failed its digest check,
and the project reported a tampered dependency for what was in fact a
republish — the failure immutability exists to prevent.

`enforce_release_immutability` now refuses the publish with
`HUB_VERSION_EXISTS`, and both writers call it: `publish_asset` before it reads
the source, and `import_remote_asset` — behind the remote publish route — before
it decodes the inbound document. Nothing durable is written on the refused path,
so the stored artifact, the version row, and its digest are exactly as the first
publish left them. The route layer answers 409, because a republish conflicts
with what is already published rather than being malformed.

### `created_at` was set on every publish — fixed

The version row recorded `created_at: now` unconditionally, so a republish also
rewrote when the version was created. With immutability enforced it is now
correct by construction: the row is written once and never again. The store no
longer moves the column either — `put_hub_asset_version` leaves `created_at` out
of its conflict update, so a rewrite that somehow reaches the adapter still
cannot change when a release was created.

### Delete removes a whole package, and frees its versions to be republished

`delete_asset_package` is the only delete path: it takes a `package_id`, drops
every version row and every artifact for it, and returns the count. There is no
per-version delete, so a publisher cannot retract `1.0.1` alone.

After a package is deleted, republishing any version it held is allowed again,
because immutability is enforced against the rows that exist. Registries treat
this as a separate decision from immutability — npm allows unpublish only inside
a 72-hour window and then blocks the name and version forever, Cargo never
deletes and only yanks. Zebflow has taken no such decision yet, so the behaviour
is left as found and recorded here.

## Still to review

- publisher identity: what it asserts, and what a static repository asserts
  having none
- media and gallery: content inside the envelope, or metadata beside it
- generalising `ProjectHubRepository`, which is hardwired to `base_url`,
  `remote_owner`, `remote_project`, and `read_token`, into the `list()` and
  `fetch()` interface described in `distribution.md`
- size and count limits are now declared in the contract: 25 MB per document
  (the existing remote cap), 18 MB per carried file, 512 MB per referenced file.
  The per-file numbers still need review against real packages
- fetching a referenced artifact over HTTP, which is the one channel still
  refusing: it needs an artifact endpoint, a size-bounded streaming read, and a
  decision about caching bytes two packages share
- how a publisher puts an artifact into the store. `store_artifact` exists as
  the writer, but `publish_asset` still carries every file inline, so nothing
  produces a referenced package yet
- whether a deleted package's versions may be republished, or whether delete
  should tombstone the coordinates the way npm and Cargo do
