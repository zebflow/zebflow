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

The cost is accepted for what the release actually carries. What a release
carries is now much less, which is the next decision.

### A release carries install; presentation lives beside it

**An immutable release carries what installing it requires. Everything a human
reads while choosing lives in mutable storage beside it.**

| Stays in the release, digest-pinned | Moves out, mutable |
| --- | --- |
| `asset_kind`, `files`, `active_pipelines`, `project_initialization` | `description_md`, `gallery`, `media`, `image_url` |
| `title` and a one-line `description` | `publisher_id`, `publisher_display_name`, `publisher_url`, `publisher_email` |
| | `source_type`, `source_owner`, `source_project`, `source_ref` |

Two things forced this. Under immutability, correcting a typo in
`description_md` cost a version bump — and a version bump changes a digest, so
every `zeb.lock` pinning it reports a changed dependency for a fixed comma.
And a cover image was base64'd into the very document the installer parses to
reach `files[]`, so a 400 KB screenshot was read, decoded, and validated by
every install that never displayed it.

`title` and `description` stay as the deliberate exception. A package handed
over as a file has no store beside it, so it still has to be able to say what
it is offline.

**Where each half now lives.**

| | Record | Mutable |
| --- | --- | --- |
| Release | `HubAssetVersion` plus its `HubPackage` document | no |
| Presentation | `HubAssetPackage`: `summary`, `description_md`, `image_url`, `media`, `gallery` | yes |
| Publisher identity | `HubAssetPackage`: `publisher_*` | yes |
| Provenance | `HubAssetVersion`: `source_owner`, `source_project`, `source_kind`, `source_ref` | local only |

Publisher identity belongs to the store that serves the package, not to the
package: a document copied into a second repository does not carry an assertion
the second repository never made. Provenance names the publishing instance's own
project structure, so it stays in that instance's version row and is never
published.

### A cover image is an artifact, not a field

`hub_cover_media_from_path` converted the publisher's chosen image to WebP and
base64'd it into the document. The WebP now goes through `store_artifact` into
the same content-addressed store referenced files use, and only the digest is
recorded, on the package row.

This is the first producer of stored artifacts, which closes half of an open
question below: `store_artifact` had a writer and no caller.

The publisher's request did not change. `PublishHubAssetRequest` still takes
`image_file_path` and the same fields; the publisher quota on image size is
still checked against the converted WebP, before anything is stored.

A cover therefore resolves the same way a referenced file does —
`<hub service root>/artifacts/<sha256>`, size-checked and digest-verified — and
`get_latest_asset_media` no longer opens a release document to serve one.

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

### Retraction removes the bytes, never the coordinates

A publisher may retract a release. Retraction deletes the stored artifact and
its referenced bytes; it does not delete the release.

| Survives retraction | Removed |
| --- | --- |
| `package_id` and `version` | the package document |
| title, description, publisher | referenced artifacts owned only by this release |
| the retracted marker and its reason | |

`package@version` can never be reused. A retracted release stays listed and
explorable so a project pinning it can see what happened, and installing it
fails with a clear reason rather than a missing artifact.

This keeps immutability true in the only way that matters: a coordinate always
means one thing, or nothing. npm reaches the same place by blocking a name and
version forever after unpublish; Cargo reaches it by never deleting and only
yanking. The difference here is that the bytes really go, so a publisher can
withdraw content while the record of it remains.

A retracted release is therefore not a hole in immutability. Freeing the
coordinates would have been.

### Publisher identity is attribution, not verification

A publisher name asserts who claims to have published, and nothing more. Zebflow
does not attempt to prove it, for the same reason git does not: anyone may fork a
project, rewrite its history, put their own name on it, and hand it to someone on
a USB stick. That is a property of open source, not a defect to engineer around.

A static repository has no publisher at all, and under this rule it is not
therefore less trustworthy — it simply makes no claim.

**The consequence is what matters.** Because identity is weak by nature, it
cannot be the thing that protects a user. Behaviour review has to be. A package
is judged by what the scan says it does, not by whose name is on it, which is why
`violations` refuses regardless of publisher and why one review runs on every
channel.

Presenting a publisher badge as a safety signal would invert this. It is
attribution for credit and contact, not a trust boundary.

## Boundary table

| Role | Where | What it does |
| --- | --- | --- |
| Contract adapter | `src/contracts/kinds/hub_package.rs` | `type Spec = HubPackageSpec`; typed, `deny_unknown_fields`, bounded, and enforces the carried-or-referenced rule |
| Artifact location | `HubArtifactChannel` | the channel's answer to "where are the referenced bytes": a local base directory, or a refusal that says why |
| Reader, artifact | `HubArtifactChannel::resolve` | reads `<base>/artifacts/<sha256>`, checks the declared size, and verifies the digest |
| Writer, artifact | `HubService::store_artifact` | puts bytes into this instance's Hub store, content-addressed, and returns the digest |
| Writer, publish | `publish_asset` | builds a manifest, writes an artifact file, records a version row; refuses a version that already exists or was retracted, and carries forward presentation it was not given |
| Writer, presentation | `update_asset_presentation` | writes the mutable package row and never a release |
| Writer, retraction | `retract_asset_package` | marks the rows retracted, then destroys the release artifacts |
| Writer, encode | `encode_hub_artifact` | wraps a spec in the envelope |
| Reader, bytes | `parse_hub_artifact_bytes` | Hub store and remote pack |
| Reader, value | `parse_hub_artifact_value` | direct payload, including local node bundle install |
| Reader, remote publish | `import_remote_asset` | validates an inbound published document |
| Reader, API | `public_hub_artifact_json` | serves the release document, withholding only `files` |
| Reader, presentation | `public_hub_presentation_json`, `hub_package_gallery_projection` | serves the mutable half, from the package row and never from a release |
| Writer, cover | `hub_cover_webp_from_path` then `store_artifact` | converts, checks the publisher quota, stores content-addressed |
| Reader, cover | `get_latest_asset_media` | package row names the digest; the artifact store supplies the bytes |
| Durable store | `data_root/<artifact_rel_path>` plus a `HubAssetVersion` row | one file per version, digest recorded as `artifact_sha256` |
| Durable store, presentation | `HubAssetPackage` row plus `<hub service root>/artifacts/<sha256>` | mutable, one stored file per distinct image |

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

### Delete freed a package's versions for reuse — fixed

`delete_asset_package` dropped every version row and every artifact for a
`package_id`. Because immutability is enforced against the rows that exist,
republishing any version the package had held was then allowed again — the same
lockfile-invalidating failure `enforce_release_immutability` exists to prevent,
reached in two steps rather than one.

Deletion is now retraction. `retract_asset_package` marks every version row and
the package row retracted, with the moment and the publisher's reason, and only
then removes the release artifacts; the markers land before the bytes go.
`HUB_VERSION_RETRACTED` refuses a republish of a retracted coordinate, and the
artifact and install paths answer `410 Gone` with the reason. The route is still
`DELETE`, because that is what a publisher means by it.

There is still no per-release retraction: it takes a `package_id`, so a
publisher cannot withdraw `1.0.1` alone.

### Presentation was mutable in the design and immutable in practice — fixed

No endpoint updated a package row's presentation, and the only writer that
existed worked against the decision: `publish_asset` set `summary` and
`description_md` empty on every publish and rebuilt `image_url`, `media`, and
`gallery` from `image_file_path` alone, so publishing a new version erased the
long description and dropped the cover unless it was re-supplied. Correcting a
typo therefore cost a version bump, which is the cost the split was made to
remove.

`PATCH /api/projects/{owner}/{project}/hub/assets/{package_id}/presentation`
(`update_asset_presentation`) now writes the package row and no release,
authorised by the same publisher token as publish, with every field optional and
an omitted field left alone. Publish carries forward the presentation it does not
supply, and a supplied cover replaces the cover entry rather than the whole
gallery.

### Unrecognised token scopes were dropped silently — fixed

`normalize_scopes` filtered to the three known scopes and discarded the rest, so
a token created with `["read","publish"]` succeeded holding `[]` and failed much
later, at a different endpoint, as `HUB_TOKEN_FORBIDDEN / scope missing`. An
unknown scope is now `HUB_TOKEN_SCOPE_INVALID` (HTTP 400) naming the offending
value and the accepted set, and a token with no usable scope is refused rather
than issued.

## Still to review

- publisher identity now lives on the package row rather than in the release.
  What it *asserts*, and what a static repository asserts having none, is still
  open
- generalising `ProjectHubRepository`, which is hardwired to `base_url`,
  `remote_owner`, `remote_project`, and `read_token`, into the `list()` and
  `fetch()` interface described in `distribution.md`
- size and count limits are now declared in the contract: 25 MB per document
  (the existing remote cap), 18 MB per carried file, 512 MB per referenced file.
  The per-file numbers still need review against real packages
- fetching a referenced artifact over HTTP, which is the one channel still
  refusing: it needs an artifact endpoint, a size-bounded streaming read, and a
  decision about caching bytes two packages share
- how a publisher puts a *content* artifact into the store. Covers now go
  through `store_artifact`, but `publish_asset` still carries every entry in
  `spec.files` inline, so nothing yet produces a referenced package
- presentation is not garbage-collected. Retracting a package removes its release
  artifacts; a cover it was the only referent of stays in the content-addressed
  store, because the store is shared and nothing counts references yet. A
  retracted package stays explorable, so keeping the cover is currently the
  wanted behaviour rather than a leak
- per-release retraction. Retraction takes a `package_id` and withdraws every
  release the package holds
