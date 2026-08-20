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
| `asset_kind`, `files`, `layout`, `active_pipelines`, `project_initialization` | `description_md`, `gallery`, `media`, `image_url` |
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

### A release records the layout its paths were produced by

Every `rel_path` in `spec.files` is repository-relative to the **publisher's**
project, and the receiver's project need not keep its source, docs, or assets in
the same directories. Nothing recorded which was which, so the installer guessed:
it stripped the *receiver's* own source root off each entry, which is right only
when the two layouts happen to agree.

`spec.layout` records the publisher's resolved layout — `source`, `assets`,
`docs`, `schema`, `sqlite_schema`, `node_interfaces` — so a receiver can tell
which portion of a path was structure and which was content.

**Why it is a seventh field rather than something beside the release.** The
split above is "a release carries install; presentation lives beside it", and
this decides *where files land*, which is install and nothing else. The place
beside a release is the mutable package row, and putting it there would make a
digest-pinned document's destinations mutable and would leave a package handed
over as a file — which has no row beside it — with nothing to translate against.

**Why one project-wide fact rather than a per-entry role.** Tagging each entry
with its area would let one manifest claim two source roots, which is a state no
project can produce. One record cannot disagree with itself.

**Why not derive it from the carried `zebflow.yaml`.** Only a `project_bundle`
carries one. A pipeline, template, or folder bundle carries none, and those are
exactly the packages whose paths need translating.

`initial_data` is deliberately absent: `project_initialization.initial_data`
already names each seed prefix with the engine that replays it, and one fact
recorded twice is a fact that can disagree with itself.

`allowed_extensions` is absent for a different reason. This record says what the
publisher's paths *meant*; the extension set says what a receiver *accepts*, and
a package does not get to declare that about the project it is installing into.
The install gate reads the receiving project's own set, so a package cannot
carry permission to write a file type that project refuses.

Every entry is optional, and an absent entry resolves through the same rule a
project that declares nothing resolves through. **A package published before this
field existed therefore carries no layout, resolves to the platform default —
which is the layout every project had when it was published — and installs.**

### Install maps into the receiving project's layout

The destination of one entry is decided per area, from the publisher's layout to
the receiver's:

| Publisher area | Where it lands |
| --- | --- |
| `source` | the target folder inside the receiver's `source`, default `hub/{package_id}` |
| `assets` | the receiver's `assets`, under the same folder name |
| `docs` | the receiver's `docs`, under the same folder name |
| anything else | inside the package's own folder, verbatim |

The last row is the decision, not a leftover. The schema exports and the node
interface directory hold **one** document for the whole project: a second copy
cannot merge, and writing it at the canonical path would overwrite the
receiver's own. Keeping it inside the package's folder is readable and inert,
which is the safe half of that pair.

A node bundle is exempt: its paths name files inside the bundle rather than
inside anyone's project, so there is nothing to translate.

The review and the install build the same placement from the same inputs and
resolve every destination through it, so a destination the review shows is the
destination the install writes. They cannot be two answers.

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
| Publisher layout | `HubPackageLayout` | the directories this manifest's paths are relative to; validated by `ProjectConfiguration`'s own `validate_layout_dir` |
| Placement | `HubInstallPlacement` | built once from the package and the target project, and asked for every destination by both the review and the install |
| Artifact location | `HubArtifactChannel` | the channel's answer to "where are the referenced bytes": a local base directory, or a refusal that says why |
| Reader, artifact | `HubArtifactChannel::resolve` | reads `<base>/artifacts/<sha256>`, checks the declared size, and verifies the digest |
| Writer, artifact | `HubService::store_artifact` | puts bytes into this instance's Hub store, content-addressed, and returns the digest |
| Writer, publish | `publish_asset` | builds a manifest, writes an artifact file, records a version row; refuses a version that already exists or was retracted, refuses a release the safety review reports violations for, and carries forward presentation it was not given |
| Gate, publish | `refuse_publish_violations` | refuses a publish and a remote publish alike, naming every violation; called before the first durable write on both |
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

### A manifest's paths meant nothing without the publisher's layout — fixed

`install_entry_rel_inside_folder` stripped the **receiver's** source root from
each entry and then re-rooted the remainder. A package published from a
`pipelines/` project into a project declaring `source: src` therefore landed at
`src/hub/{id}/pipelines/blog/feed.zf.json`, carrying the publisher's root along
as a dead segment; its images landed outside the directory the asset route
serves, so every `/assets/...` URL in its pages 404'd while the review reported
nothing wrong. That is the same shape as the target-folder defect: it installs,
it reviews clean, and it does not work.

`spec.layout` and `HubInstallPlacement` replace the guess with the recorded
fact.

### A whole project added at project scope landed outside the source root — fixed

`install_asset` accepts a `project_bundle`, and `default_install_target_folder`
gave every kind that was not a pipeline or template bundle a root at `hub/{id}`
— outside the source root. Its pipelines were not pipelines by the discovery
rule, so the review scanned none of them and the install registered none of
them. Every kind that is not a `node_bundle` now roots inside `source`, which is
the same rule and the same reason as the target-folder fix.

### A published pipeline bundle lost the page it renders — fixed

`preview_pipeline` read the stored pipeline as raw JSON and looked for a
top-level `nodes` array. A stored pipeline is a `Pipeline` contract document
whose nodes live under `spec`, so the lookup found none, no `n.web.response`
node was ever seen, and the bundle was published without the template it
renders. It decodes through `decode_pipeline_graph` now, like every other reader
of a stored pipeline.

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

### Publish computed violations and stored the release anyway — fixed

`HubPublishReview` carried a `violations` field, reported it, and `publish_asset`
never read it. Both install gates refuse on violations, so a hub would accept and
store a release that no instance — its own included — would ever install.

That is worse than untidy, because a release is immutable. `enforce_release_immutability`
refuses the coordinate a second time and retraction keeps it reserved, so a
publisher who shipped a violation had burned that version number permanently.
Refusing at publish costs a retry; accepting cost a version.

`refuse_publish_violations` now runs in `publish_asset` and in `import_remote_asset`,
in both cases before the first durable write — in `publish_asset` that meant
reading the cover bytes before storing them, since `store_artifact` was the first
write and ran before any review. The error names every violation.

`import_remote_asset` refuses too, and the reason is that it is not a hub judging
a peer's catalogue. It is the remote publish route: the bearer token authenticates
a publisher registered on *this* hub, pushing their own package to it. The
coordinate it would burn is burned here, where the sending instance cannot retract
it, and every installer fetching from here would refuse what they were served.

**The publish review read the wrong extension set — fixed with it.** The
extension allowlist says what a *receiver* accepts: `recorded_publisher_layout`
deliberately omits it and `publisher_layout` resolves it to the platform set. The
publish review, alone, read the source project's own resolved layout, which
carries whatever that project narrowed itself to. That was harmless while publish
only reported; as a refusal it would have blocked a publisher from shipping a file
type every receiver accepts, with no override anywhere. `publish_review_layout`
keeps the publisher's directories — those decide which entries are pipelines — and
restores the extension set to the platform floor.

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

- publish and install do not read a package through identical eyes, and two
  gaps remain. `refuse_prepared_install_violations` hands the review
  `unreadable: ""` for every entry, so "a pipeline the safety review cannot
  read" is unreachable on that path: a non-UTF-8 `.zf.json` is refused at
  publish and passes there. And a package referencing an artifact resolves
  against `artifact_store()` at publish and at local install, but every remote
  channel is `Unresolvable`, so the same release publishes and installs locally
  while refusing remotely. Nothing produces a referenced package yet, so the
  second is latent
- `HUB_INSTALL_REFUSED` and `HUB_REMOTE_INSTALL_REFUSED` are unmapped in
  `hub_api_error` and answer 500. The publish refusals were mapped to 400 when
  they were added; the install pair still reads as a server fault
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
- uninstall for an add. Added content has no update path by design, and removal
  is deleting the files. A package now occupies one folder *name* in up to three
  areas rather than one directory, so "delete the folder" is three deletions the
  user has to know about. The review lists every destination, and nothing
  records them afterwards
- a hand-authored `project_bundle` whose `spec.layout` disagrees with the
  `zebflow.yaml` it carries. Platform-scope install reviews against the recorded
  layout and creates the project from the carried configuration; nothing checks
  that the two say the same thing. No writer produces such a document

## Freeze judgement — 2026-08-20

Status: **Review**, close to Candidate. Not Frozen.

### What the evidence covers

The format is settled and proven. Encoding is byte-reproducible and
digest-stable; a reordered document normalises to the same bytes and an unknown
key is refused rather than dropped, at every level of the tree. The canonical
form is reproducible from outside Rust — `json.dumps(doc, indent=2,
ensure_ascii=False)` returns the golden fixture byte for byte — so verification
is arithmetic a third party can perform without running Zebflow.

Proven against a running server on this date, against the binary as built:

| Step | Result |
| --- | --- |
| publish | `contract-lab.freeze-evidence@1.0.0` stored |
| republish the same coordinate | **409**, `HUB_VERSION_EXISTS` |
| install review, three target folders | identical verdicts, all rooted in source |
| install | file written, pipeline registered `hub/…/hubtest/danger.zf.json`, trigger inferred |
| public access to a private package | hidden from listing, `401` on detail |

The target-folder asymmetry recorded earlier is gone. Installing to `billing`
reviewed `low` with every finding list empty and registered nothing; it now
reviews `high` with two public endpoints and registers one pipeline, matching
both the default folder and `/billing`.

### Why it is not Frozen

Three reasons, in order of weight.

**`spec.layout` is one day old.** It was added on 2026-08-20 so a package
published under one project layout can install into a project declaring another.
Freezing a field with a single day's exercise is how a contract acquires a
permanent mistake. This kind's sibling was marked a freeze candidate on evidence
that predated a change and the claim had to be retracted; the lesson is cheap to
apply and expensive to skip.

**No package with a referenced artifact exists.** `publish_asset` carries every
entry inline, so the `artifact` half of the carried-or-referenced rule has never
run outside unit tests. That half is exactly where a review bypass was found on
2026-08-19: a referenced pipeline reviewed as empty content while the install
wrote the real bytes. A rule proven only by unit test is not proven the way the
carried half is.

**Two enforcement points disagree about the same document.**
`refuse_prepared_install_violations` sets `unreadable` empty unconditionally, so
it can never produce the "a pipeline the safety review cannot read" violation. A
non-UTF-8 `.zf.json` is refused at publish and at project-bundle install, and
reviews as an empty pipeline on the prepared-install path. A contract whose gates
answer differently about the same bytes is not settled, whatever its format does.

### What would close it

1. Produce a package carrying a referenced artifact and run it through publish,
   review, install and run — the same live pass the carried form has had.
2. Reconcile the two install gates so every gate reaches the same verdict on the
   same document.
3. Give `spec.layout` time and a cross-layout install that was not written the
   same week as the field.

Per-release retraction, a `.wasm` governed by anything beyond the NodeBundle
contract, and remote pack rows carrying a retraction marker are open, but none
of them move the on-disk format and none block a freeze.
