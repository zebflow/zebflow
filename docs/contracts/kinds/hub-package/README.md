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
| Writer, publish | `publish_asset` | builds a manifest, writes an artifact file, records a version row |
| Writer, encode | `encode_hub_artifact` | wraps a spec in the envelope |
| Reader, bytes | `parse_hub_artifact_bytes` | Hub store and remote pack |
| Reader, value | `parse_hub_artifact_value` | direct payload, including local node bundle install |
| Reader, remote publish | `hub.rs:4587` | validates an inbound published document |
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

### Republishing silently overwrites

`publish_asset` builds a `HubAssetVersion` and calls `put_hub_asset_version`
without checking whether that `package_id` and `version` already exist. A second
publish of the same version replaces the row, the artifact path, and the digest.

Any `zeb.lock` entry pinning that version then fails its digest check, and the
project reports a tampered dependency for what was in fact a republish. That is
the failure immutability exists to prevent, and it is reachable today.

### `created_at` is set on every publish

The version row records `created_at: now` unconditionally, so a republish also
rewrites when the version was created. With immutability enforced this becomes
correct by construction; until then it hides that a republish happened.

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
- staged install and rollback across a manifest plus N artifacts
