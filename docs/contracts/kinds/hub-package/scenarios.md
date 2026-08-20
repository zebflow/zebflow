# HubPackage scenarios

Status: **Review**

This is the companion to [`README.md`](./README.md). The README says what the
contract decides and why. This document walks the decisions through real use:
what a publisher types, what bytes exist afterwards, what the receiver is shown
before accepting, and what refuses.

It exists for two readers. Someone deciding whether to trust and use Hub
packages needs to see the actual shapes rather than a description of them.
Zebflow's own team needs the design exercised end to end, because writing out
real usage is how gaps surface, and the gaps this exercise found are collected
in [§10](#10-what-these-scenarios-exposed).

## 0. How to read this

**Every scenario is marked for what actually runs today.** Distribution is
partly designed and partly built, and a document that describes behaviour the
code does not perform is worse than no document. So each scenario carries one of:

| Marker | Meaning |
| --- | --- |
| **Runs today** | every request, response, and file in it was traced to code that performs it |
| **Partly runs** | the scenario is real but one named step is not built; the step is called out inline |
| **Not built** | the design is decided and nothing performs it; the scenario shows the target and says so |

Inline, an unbuilt step looks like this:

> **Not built.** What is missing, where it would live, and what happens instead
> today.

JSON bodies are the real serialized shapes of the named Rust types. Digests are
real where a real file produced them, and are marked illustrative where the
content is invented.

### Vocabulary, in one table

The three verbs are not synonyms and this document uses them strictly. See
[`../../distribution.md`](../../distribution.md#1a-the-verbs).

| Verb | Scope | Lands in | Receiver may edit | Removal |
| --- | --- | --- | --- | --- |
| **install** (project scope) | one project gains a managed dependency | `data/` | no | uninstall |
| **install** (platform scope) | a whole project is materialised | a new project | yes, it is theirs | none; irreversible |
| **add** | content is copied in as the receiver's own source | `repo/` | yes | delete the files |
| **import** | whole project areas replaced or merged | `repo/`, `data/`, `files/` | yes | none; destructive |

One route name is a trap worth stating once: the Hub project-scope endpoint is
`POST .../hub/assets/{id}/{version}/add`, and for a `node_bundle` it performs an
**install** in the table's sense — the bytes land in `data/`, a `zeb.lock` entry
appears, and uninstall removes it. The route is named for the common case
(pipelines and templates, which really are `add`) and the asset kind decides
which act happens. Scenarios 2 and 3 are the same route doing the two different
things.

### The cast

| | |
| --- | --- |
| `studio.example` | the publishing instance, running a local Hub |
| `acme` | a publisher registered on that Hub |
| `dana` | someone on a second instance who wants the package |
| `mirror.example` | a third instance holding a copy, which nobody has audited |

---

## 1. A publisher ships their first release

**Runs today.**

Acme has a pipeline in the project `acme/billing` that turns a webhook into an
invoice PDF, plus the TSX page it renders. They want it on their instance's Hub
so the rest of the company can pick it up.

### 1.1 Get a publisher token

A publisher record is created by an operator; a project then mints a token
against it.

```bash
curl -s -b /tmp/zf.txt -X POST \
  -H "Content-Type: application/json" \
  -d '{"publisher_id":"acme","title":"billing release token","scopes":["hub:read","hub:publish"]}' \
  http://studio.example/api/projects/acme/billing/hub/tokens
```

```json
{
  "ok": true,
  "token": { "token_id": "mkt_4f1c9a2b", "publisher_id": "acme", "scopes": ["hub:read", "hub:publish"] },
  "secret": "zfmt_mkt_4f1c9a2b_9d3e...(24 bytes hex)"
}
```

The secret is shown once; only its SHA-256 is stored. Scopes are checked twice:
the token cannot hold a scope the publisher record does not grant
(`validate_publisher_scopes`), and publish re-checks `hub:publish` at use.

### 1.2 See what publishing would carry, before publishing

`publish-review` is not a formality. It is the same scanner the receiver runs,
pointed at the publisher's own source, so the publisher finds out what their
package looks like from outside before anyone else does.

```bash
curl -s -b /tmp/zf.txt -X POST \
  -H "Content-Type: application/json" \
  -d '{
        "source_type": "pipeline_with_dependencies",
        "source_ref": "pipelines/api/invoice.zf.json",
        "package_id": "invoice-tools",
        "version": "1.0.0",
        "title": "Invoice Tools",
        "description": "Webhook to invoice PDF, with the rendering page.",
        "image_file_path": "media/invoice-cover.png",
        "visibility": "public",
        "tags": ["billing", "pdf"],
        "publisher_token": "zfmt_mkt_4f1c9a2b_..."
      }' \
  http://studio.example/api/projects/acme/billing/hub/assets/publish-review
```

```json
{
  "ok": true,
  "review": {
    "package_id": "acme.invoice-tools",
    "version": "1.0.0",
    "asset_kind": "pipeline_bundle",
    "source_type": "pipeline_with_dependencies",
    "source_ref": "pipelines/api/invoice.zf.json",
    "title": "Invoice Tools",
    "description": "Webhook to invoice PDF, with the rendering page.",
    "visibility": "public",
    "tags": ["billing", "pdf"],
    "total_files": 3,
    "total_bytes": 18244,
    "files": [
      { "rel_path": "pipelines/api/invoice.zf.json", "kind": "json", "size_bytes": 6120,
        "reason": "primary pipeline", "encoding": "text", "content": "…" },
      { "rel_path": "pipelines/components/invoice-line.tsx", "kind": "tsx", "size_bytes": 2814,
        "reason": "imported from pipelines/pages/invoice.tsx", "encoding": "text", "content": "…" },
      { "rel_path": "pipelines/pages/invoice.tsx", "kind": "tsx", "size_bytes": 9310,
        "reason": "web response template", "encoding": "text", "content": "…" }
    ],
    "media": [
      { "name": "cover.webp", "role": "cover", "content_type": "image/webp", "size_bytes": 41220 }
    ],
    "nodes_used": ["n.trigger.webhook", "n.pg.query", "n.web.response"],
    "credentials_required": ["billing_pg"],
    "external_urls": ["https://fonts.example/inter.css"],
    "database_effects": ["n.pg.query"],
    "filesystem_effects": [],
    "public_endpoints": ["/invoice", "webhook trigger"],
    "schedules": [],
    "large_files": [],
    "seed_data": [],
    "project_initialization": {
      "include_sekejap_schema": false,
      "include_sqlite_schema": false,
      "libraries": [],
      "initial_data": []
    },
    "warnings": [],
    "violations": [],
    "risk_level": "high"
  }
}
```

Three things in that output are worth a publisher's attention, and all three are
computed rather than declared:

- `credentials_required: ["billing_pg"]` — the export carries the *name* of a
  credential the pipeline reads. It never carries the value. Credential values
  are the one row in `distribution.md` §1 whose channel column reads **none**.
- `external_urls` — every `http(s)` string found anywhere in the carried text,
  including a font URL inside the page component. A publisher who did not know
  their package reaches out to a third party now knows.
- `risk_level: "high"` — a database effect plus a public endpoint plus a
  credential already scores 4. High is normal for anything useful; it is a
  description, not an accusation.

Note what `files[]` shows: `content` inline, `encoding: "text"`, and no
`artifact`. **Everything `publish_asset` produces today is carried.** The
referenced form is accepted by the format and resolvable by the installer, but
no publish path emits one — see [§7](#7-a-package-with-a-referenced-artifact).

### 1.3 Publish

Same body, different route.

```bash
curl -s -b /tmp/zf.txt -X POST \
  -H "Content-Type: application/json" -d @publish.json \
  http://studio.example/api/projects/acme/billing/hub/assets/publish
```

```json
{
  "ok": true,
  "package": {
    "package_id": "acme.invoice-tools",
    "publisher_id": "acme",
    "publisher_display_name": "Acme Corp",
    "asset_kind": "pipeline_bundle",
    "title": "Invoice Tools",
    "description": "Webhook to invoice PDF, with the rendering page.",
    "summary": "",
    "description_md": "",
    "image_url": "/api/hub/remote/assets/acme.invoice-tools/media/cover.webp",
    "media": [
      { "name": "cover.webp", "role": "cover", "content_type": "image/webp",
        "size_bytes": 41220,
        "artifact_sha256": "…64 hex, illustrative…" }
    ],
    "visibility": "public",
    "tags": ["billing", "pdf"]
  },
  "version": {
    "package_id": "acme.invoice-tools",
    "version": "1.0.0",
    "source_owner": "acme",
    "source_project": "billing",
    "source_kind": "pipeline_with_dependencies",
    "source_ref": "pipelines/api/invoice.zf.json",
    "artifact_rel_path": "services/hub-default/packages/acme.invoice-tools/versions/1.0.0/artifact.json",
    "artifact_sha256": "…64 hex, illustrative…",
    "created_at": 1755561600
  }
}
```

The package id was rewritten: the request said `invoice-tools` and the stored id
is `acme.invoice-tools`. `canonical_hub_package_id` forces the publisher prefix,
and sending `someoneelse.invoice-tools` is refused with
`HUB_PACKAGE_ID_INVALID: package id must use the selected publisher prefix`.

### 1.4 What now exists on disk

```text
<data_root>/
  services/hub-default/
    packages/acme.invoice-tools/versions/1.0.0/artifact.json   the release, immutable
    artifacts/<sha256-of-cover-webp>                            the cover, content-addressed
```

Plus two database rows. The split is the whole design in one picture:

| Row | Holds | Mutable |
| --- | --- | --- |
| `HubAssetVersion` for `1.0.0` | `artifact_rel_path`, `artifact_sha256`, provenance | no, by `enforce_release_immutability` |
| `HubAssetPackage` for `acme.invoice-tools` | title, description, `summary`, `description_md`, `image_url`, `media`, `gallery`, tags, visibility, publisher identity | yes, in principle |

"In principle" is doing real work in that last cell. See
[§6.2](#62-fixing-a-typo-without-a-version-bump).

The cover is *not* in `artifact.json`. It went through `store_artifact` into
`artifacts/<sha256>` and only its digest is on the package row, which is why
`get_latest_asset_media` can serve the image without opening a release document.

### 1.5 The release document itself

The file at `artifact.json` is a plain `HubPackage` envelope. Rather than invent
one, here is the repository's own golden fixture, byte for byte
(`tests/fixtures/contracts/hub-package/v1-complete.json`), because its digest is
then something you can check yourself:

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "HubPackage",
  "metadata": {
    "name": "acme.demo-tools",
    "version": "1.0.0"
  },
  "spec": {
    "asset_kind": "node_bundle",
    "title": "Demo Tools",
    "description": "One carried definition, one carried binary, one referenced module.",
    "active_pipelines": [
      "pipelines/api/hello.zf.json"
    ],
    "project_initialization": {
      "include_sekejap_schema": true,
      "include_sqlite_schema": false,
      "libraries": [
        "zeb/prosemirror"
      ],
      "initial_data": [
        {
          "engine": "sekejap",
          "path": "data/sekejap/seed.sql",
          "statement_count": 2,
          "size_bytes": 64
        }
      ]
    },
    "files": [
      {
        "rel_path": "definition.json",
        "kind": "node_definition",
        "size_bytes": 30,
        "reason": "package",
        "encoding": "text",
        "content": "{\n  \"kind\": \"n.x.demo.load\"\n}\n"
      },
      {
        "rel_path": "icon.svg",
        "kind": "asset",
        "size_bytes": 41,
        "reason": "package",
        "encoding": "base64",
        "content": "PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciLz4="
      },
      {
        "rel_path": "wasm/core.wasm",
        "kind": "asset",
        "size_bytes": 33554432,
        "reason": "package",
        "artifact": {
          "sha256": "9f2bc14a3c6e0b8a9c15224a8228b9a98ca1531d3c6e0b8a9c15224a8228b9a9",
          "media_type": "application/wasm"
        }
      }
    ]
  }
}
```

Six spec fields, no seventh. There is no cover, no long description, no
publisher name, no "which project did this come from". A release carries what
installing it requires. `title` and the one-line `description` are the kept
exception, so this file handed over on a USB stick still says what it is.

Note the third entry: `artifact` with a digest, no `content`, no `encoding`, and
no URL anywhere. That entry is 32 MB of WASM that will never fit inside a 25 MB
document, and its location will come from whichever channel installs it.

---

## 2. Someone installs it and reads the safety review first

**Runs today**, with one gap called out at the end.

Dana wants the invoice tools in `dana/ops`. `pipeline_bundle` is an **add**: the
files become Dana's own source in `repo/`, which means Dana can edit them and
also that there is no update path afterwards.

### 2.1 Review before accepting

```bash
curl -s -b /tmp/zf.txt -X POST \
  -H "Content-Type: application/json" -d '{}' \
  http://studio.example/api/projects/dana/ops/hub/assets/acme.invoice-tools/1.0.0/review
```

An empty body means "use the default folder", which for a pipeline bundle is
`pipelines/hub/{package_id}`.

```json
{
  "ok": true,
  "review": {
    "package_id": "acme.invoice-tools",
    "version": "1.0.0",
    "target_folder": "",
    "install_root": "pipelines/hub/acme.invoice-tools",
    "asset_kind": "pipeline_bundle",
    "files_added": [
      "pipelines/hub/acme.invoice-tools/api/invoice.zf.json",
      "pipelines/hub/acme.invoice-tools/components/invoice-line.tsx",
      "pipelines/hub/acme.invoice-tools/pages/invoice.tsx"
    ],
    "files_overwritten": [],
    "pipelines_registered": ["pipelines/hub/acme.invoice-tools/api/invoice.zf.json"],
    "nodes_used": ["n.pg.query", "n.trigger.webhook", "n.web.response"],
    "credentials_required": ["billing_pg"],
    "external_urls": ["https://fonts.example/inter.css"],
    "database_effects": ["n.pg.query"],
    "filesystem_effects": [],
    "public_endpoints": ["/invoice", "webhook trigger"],
    "schedules": [],
    "large_files": [],
    "seed_data": [],
    "project_initialization": {
      "include_sekejap_schema": false,
      "include_sqlite_schema": false,
      "libraries": [],
      "initial_data": []
    },
    "warnings": [],
    "violations": [],
    "installable": true,
    "risk_level": "high"
  }
}
```

The install paths are not the export paths. `install_entry_rel_inside_folder`
strips a leading `pipelines/` or `templates/` from each entry before joining it
under the install root, so the exported `pipelines/api/invoice.zf.json` lands at
`pipelines/hub/acme.invoice-tools/api/invoice.zf.json` rather than doubling the
segment.

### 2.1a The same package, one word different

Now Dana types a destination instead of taking the default.

```bash
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d '{"target_folder": "billing"}' \
  http://studio.example/api/projects/dana/ops/hub/assets/acme.invoice-tools/1.0.0/review
```

```json
{
  "review": {
    "target_folder": "billing",
    "install_root": "billing",
    "files_added": [
      "billing/api/invoice.zf.json",
      "billing/components/invoice-line.tsx",
      "billing/pages/invoice.tsx"
    ],
    "pipelines_registered": [],
    "nodes_used": [],
    "credentials_required": [],
    "external_urls": [],
    "database_effects": [],
    "public_endpoints": [],
    "warnings": [],
    "violations": [],
    "installable": true,
    "risk_level": "low"
  }
}
```

**Same package, same bytes, and every finding is gone.** The credential, the
database access, and the public endpoint have not changed; the review simply
stopped looking.

`is_reviewed_pipeline_path` tests the path the install *writes*: it must start
with `pipelines/` and end with `.zf.json`. Under `billing` the pipeline lands at
`billing/api/invoice.zf.json`, which fails the first half, so it is never parsed.
Testing the installed path rather than the source path is deliberate and correct
in intent — the review and the install must never disagree about which entries
are pipelines — but it makes the review's *reach* a function of a folder name the
receiver types.

The sharpest form of it: `normalize_install_target_folder` prefixes `pipelines/`
when the typed folder begins with a slash. So `"/billing"` is scanned and
`"billing"` is not.

> **Design gap, not an unbuilt feature.** A receiver who chooses their own
> destination can silently turn the safety review off for exactly the files that
> carry the behaviour, and is shown `risk_level: "low"` when they do. Recorded in
> [§10](#10-what-these-scenarios-exposed). Read every receiver-side review in
> this document as "what the scanner saw", not "what the package does".

Taking the default folder, the review reports the full picture, and that is the
version to read the table below against.

What Dana should read off this before accepting:

| Field | What it means for Dana |
| --- | --- |
| `credentials_required: ["billing_pg"]` | this package will not work until Dana creates a credential of that kind. The package did not bring one, and cannot. |
| `public_endpoints` | installing this puts a route on Dana's project that the internet can reach. |
| `database_effects` | it runs SQL against a connection Dana supplies. |
| `files_overwritten: []` | nothing Dana already has is being replaced. A non-empty list here adds 2 to the risk score, and is the one finding that is about Dana's project rather than the package. |
| `installable: true` | nothing refuses. `false` would mean the Add button must not be offered at all. |

### 2.2 Accept

```bash
curl -s -b /tmp/zf.txt -X POST \
  -H "Content-Type: application/json" -d '{}' \
  http://studio.example/api/projects/dana/ops/hub/assets/acme.invoice-tools/1.0.0/add
```

```json
{
  "ok": true,
  "target_folder": "",
  "result": {
    "package_id": "acme.invoice-tools",
    "version": "1.0.0",
    "asset_kind": "pipeline_bundle",
    "install_root": "pipelines/hub/acme.invoice-tools",
    "files_written": 3,
    "pipelines_registered": ["pipelines/hub/acme.invoice-tools/api/invoice.zf.json"]
  }
}
```

Afterwards, in `users/dana/ops/repo/`:

```text
pipelines/hub/acme.invoice-tools/
  api/invoice.zf.json              also registered as a pipeline definition
  components/invoice-line.tsx
  pages/invoice.tsx
```

These are Dana's files now. Nothing was written to `zeb.lock`, because add is not
a dependency. Removal is `rm`. **There is no update path**: if Acme ships 1.1.0,
Dana adds it into a different folder and merges by hand, because re-adding over
edited files would destroy Dana's work.

The write is all-or-nothing. Every entry's bytes are produced and verified by
`prepare_hub_install_entries` before the first `atomic_write`, and a failure part
way through goes to `recover_failed_install`, which restores what was there.

> **Not built: the add route never consults the review.**
> `install_asset` reads the version row, verifies the artifact digest, and
> installs. It does not call `review_artifact_payload` and does not check
> `installable`. A violation therefore blocks a *locally supplied* node bundle
> (`install_node_bundle_payload` does check, and refuses with
> `NODE_BUNDLE_INSTALL_REFUSED`) but does not block a Hub add of the same
> package. The UI runs `/review` first and can decline to offer the button, but
> the API does not enforce it, so the non-overridable tier is only non-overridable
> on one of the two paths. This is the single most important thing in this
> document.

---

## 3. The same route, doing an install instead

**Runs today.**

Acme also publishes a node bundle, `acme.pdfkit@2.0.0`, `asset_kind:
"node_bundle"`. Dana adds it through the same endpoint and gets a completely
different kind of act, because the asset kind decides the destination.

```bash
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" -d '{}' \
  http://studio.example/api/projects/dana/ops/hub/assets/acme.pdfkit/2.0.0/add
```

```json
{
  "ok": true,
  "target_folder": "",
  "result": {
    "package_id": "acme.pdfkit",
    "version": "2.0.0",
    "asset_kind": "node_bundle",
    "install_root": "nodes/acme.pdfkit",
    "files_written": 4,
    "pipelines_registered": []
  }
}
```

The bytes landed under `data/`, not `repo/`:

```text
users/dana/ops/data/nodes/acme.pdfkit/
  definition.json
  icon.svg
  functions/render.zf.json
  wasm/pdfkit.wasm
```

and `repo/zeb.lock` gained a declaration:

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "DependencyLock",
  "metadata": { "name": "ops" },
  "spec": {
    "rwe": { "libraries": {} },
    "nodes": {
      "bundles": {
        "hub/local/acme.pdfkit": {
          "version": "2.0.0",
          "source": "hub",
          "source_id": "local/acme.pdfkit",
          "entry": "nodes/acme.pdfkit/definition.json",
          "integrity": "sha256:…64 hex, illustrative…",
          "definitions": ["n.x.acme.pdf.render"]
        }
      }
    }
  }
}
```

That is the ownership model working: the declaration is authored state in
`repo/` and travels with the git history; the bytes are machine output in
`data/` and are rebuildable from the declaration. Uninstall
(`DELETE .../nodes/uninstall/{kind}`) removes both.

**Read `integrity` carefully.** It is
`directory_tree_sha256(data/nodes/acme.pdfkit)` — a digest of the installed
tree, computed after the install. It is **not** the sha256 of the `HubPackage`
release document. So the lock detects local tampering with installed bytes, and
does not by itself let you prove the release you have is the release that was
published. That proof is §5, and the fact that the lock does not carry it is
[§10](#10-what-these-scenarios-exposed).

---

## 4. A package that cannot be installed by anyone

**Partly runs.** The tier, the refusal, and the first detector exist; the wiring
that makes every channel honour it does not.

A warning asks "do you accept this effect?". A violation says "this will not be
installed, whoever approves it". They are different questions and the review
answers them in different fields.

### 4.1 The first detector: a pipeline the review cannot read

The contract already closed the crude version of this hole: an entry may declare
`encoding` only as `text`, `base64`, or absent, because an entry labelled `utf8`
used to be unreadable to the scanner and still written verbatim by the installer.

The subtler version survives the contract. An entry landing at a reviewed
pipeline path can be **referenced** rather than carried, and its bytes then come
from the channel. The review now resolves references itself — `entry_review_text`
takes the same `HubArtifactChannel` the install would use — so a reference the
channel *can* satisfy is read and scanned exactly like carried bytes. The
violation fires when it cannot: an unresolvable channel, a digest or size
mismatch, base64 that does not decode, or bytes that are not UTF-8.

The distinction that makes this a violation rather than a clean result: an empty
`content` means an empty file, whereas an unreadable entry means the reviewer was
handed nothing while the install would still write something. Those must never
produce the same verdict, so `PackagePolicyEntry` carries an `unreadable` reason
and a non-empty reason at a pipeline path is a violation.

Acme hand-authors `1.2.0` with the pipeline referenced rather than carried, and
publishes it to a hub whose store does not hold that artifact. Dana reviews:

```json
{
  "ok": true,
  "review": {
    "package_id": "acme.invoice-tools",
    "version": "1.2.0",
    "install_root": "pipelines/hub/acme.invoice-tools",
    "files_added": ["pipelines/hub/acme.invoice-tools/api/invoice.zf.json"],
    "nodes_used": [],
    "credentials_required": [],
    "warnings": [],
    "violations": [
      "pipelines/hub/acme.invoice-tools/api/invoice.zf.json: a pipeline the safety review cannot read (HUB_ARTIFACT_MISSING: file 'pipelines/api/invoice.zf.json' references artifact 4c1a…, which this channel does not have)"
    ],
    "installable": false,
    "risk_level": "blocked"
  }
}
```

`blocked` is not a fourth score. `install_risk_level` short-circuits on any
violation and `review_package_entries` does the same, because no arrangement of
accepted effects makes an unreviewable pipeline safe, and overwrites cannot
re-score a package that is refused outright.

The refusal, on the local node bundle path:

```json
{
  "ok": false,
  "error": "NODE_BUNDLE_INSTALL_REFUSED: package cannot be installed: pipelines/… : a pipeline the safety review cannot read (…)"
}
```

> **Not built (two parts).**
>
> 1. **The Hub add route does not check.** As in §2.2, `install_asset` verifies
>    the release digest and calls `install_artifact_payload_from` directly. It
>    never calls `review_artifact_payload` and never reads `installable`. So this
>    violation blocks `POST .../nodes/install` and does not block
>    `POST .../hub/assets/{id}/{v}/add` for the same package. Until that is
>    wired, "never overridable" is true of one channel and enforced by the UI on
>    the other.
> 2. **The local node bundle refusal returns HTTP 200.**
>    `api_install_local_node_bundle` answers `{"ok": false, "error": "CODE:
>    message"}` with a 200 status and a flat string, unlike every other Hub route,
>    which returns a status code and `{"error": {"code", "message"}}`. A client
>    checking the status code will read a refusal as a success — on the one route
>    that actually enforces violations.

### 4.2 What is deliberately not a violation

Everything else. Reaching an external URL, requiring a credential, exposing a
webhook, running SQL, shipping 40 MB of seed data — all warnings and effect
lists, all acceptable, all Dana's decision. The tier is reserved for findings
precise enough to refuse without a false positive, and there is exactly one so
far. Growing it is deliberate slowness, not neglect: a violation that misfires
makes a package uninstallable with no override, which is a worse failure than a
warning nobody reads.

---

## 5. Verifying a package pulled from an untrusted mirror

**Runs today for the arithmetic. Partly runs for where the expected digest comes
from.**

Dana finds `acme.invoice-tools@1.0.0` on `mirror.example`, which nobody at Acme
runs. The question is not "is the mirror nice" but "are these the bytes Acme
published".

### 5.1 What the mirror serves

```bash
curl -s https://mirror.example/api/hub/remote/assets/acme.invoice-tools/1.0.0/artifact
```

```json
{
  "ok": true,
  "version": {
    "package_id": "acme.invoice-tools",
    "version": "1.0.0",
    "publisher_id": "acme",
    "publisher_display_name": "Acme Corp",
    "asset_kind": "pipeline_bundle",
    "artifact_sha256": "57aa4c19194e223612731dad9864931f54d57cccbd2eea0aa5d4eca36b1d2353",
    "source_kind": "pipeline_with_dependencies",
    "created_at": 1755561600
  },
  "artifact_sha256": "57aa4c19194e223612731dad9864931f54d57cccbd2eea0aa5d4eca36b1d2353",
  "artifact_size_bytes": 1521,
  "artifact": { "apiVersion": "zebflow.com/v1", "kind": "HubPackage", "…": "…" }
}
```

### 5.2 The check Zebflow performs

`verify_remote_artifact_hash` does not hash the JSON it received. It **decodes
the document through the contract and re-encodes it**, then hashes that:

```text
decode_contract_value::<HubPackageContract>(artifact)   strict envelope, typed spec, full validation
encode_hub_package(metadata, spec)                      canonical bytes
sha256_hex(bytes) == expected                           or HUB_REMOTE_HASH_MISMATCH
```

Re-encoding rather than re-hashing is the point. The digest names the *document*,
not one serialization of it, so a mirror that reindents the JSON, reorders the
root keys, or adds `"encoding": ""` where the canonical form omits it still
produces the same digest — and a mirror that changes one byte of a pipeline does
not.

The digest is deliberately not inside the document. A field naming the hash of
the thing it sits in cannot be computed without excluding itself, and every
format that tried has an exclusion rule people get wrong.

### 5.3 Doing it yourself, offline

The canonical encoding is `serde_json::to_vec_pretty` plus a trailing newline:
two-space indent, struct field order, no escaping of non-ASCII, `None` and empty
optionals omitted. That is reproducible without Zebflow. Against the fixture in
§1.5:

```bash
python3 - <<'PY'
import json, hashlib
raw = open('tests/fixtures/contracts/hub-package/v1-complete.json','rb').read()
canon = (json.dumps(json.loads(raw), indent=2, ensure_ascii=False) + '\n').encode()
print('byte-identical:', canon == raw)
print(hashlib.sha256(canon).hexdigest())
PY
```

```text
byte-identical: True
57aa4c19194e223612731dad9864931f54d57cccbd2eea0aa5d4eca36b1d2353
```

That digest is real: it is the sha256 of that file as committed, and the
contract's own `golden_package_roundtrips_without_schema_drift` test asserts
that `encode_hub_package` reproduces those exact bytes.

**The caveat that makes this honest.** A generic JSON re-dump only agrees with
the contract encoder for a document already in canonical form. Hand it a
document carrying `"encoding": ""`, or with `spec` before `metadata`, and the
re-dump differs while `decode` then `encode` normalises. So the Python recipe is
a *spot check you can run without Rust*, not the definition. The definition is
`decode_hub_package` followed by `encode_hub_package`.

### 5.4 Where the expected digest should come from

Everything above compares the document against a digest **the mirror supplied in
the same response**. That catches corruption and a careless proxy. It catches
nothing about a mirror that intends to lie, because a liar edits both halves.

The trustworthy comparison is against a digest recorded somewhere the mirror does
not control: a `zeb.lock` entry from a previous resolution, a repository index
fetched from elsewhere, or a digest Acme published out of band.

> **Not built.** Nothing records a `HubPackage` release digest in `zeb.lock`.
> The node bundle lock entry's `integrity` is a digest of the installed
> directory tree (§3), so it detects local modification of installed bytes and
> cannot answer "is this the release Acme published". `distribution.md` §4 lists
> integrity as supplied by the lock; for `HubPackage` that is currently
> aspirational. Until it lands, cross-mirror verification means comparing the
> `artifact_sha256` two independent sources report, by hand.

---

## 6. Shipping v2, and fixing a typo

### 6.1 Republishing 1.0.0 is refused

**Runs today.**

Acme fixes a bug and publishes with the version field unchanged.

```bash
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d @publish-1.0.0-again.json \
  http://studio.example/api/projects/acme/billing/hub/assets/publish
```

```text
HTTP/1.1 409 Conflict
```

```json
{
  "ok": false,
  "error": {
    "code": "HUB_VERSION_EXISTS",
    "message": "acme.invoice-tools@1.0.0 is already published and releases are immutable; bump the version and publish again"
  }
}
```

409 rather than 400: the request is well formed and conflicts with what is
already there.

`enforce_release_immutability` runs *before* the source project is read, before
the cover is converted and stored, and before any file is written, so a refused
republish leaves the artifact file, the version row, its digest, and its
`created_at` exactly as the first publish left them. The remote publish route
(`import_remote_asset`) calls the same check before decoding the inbound
document, so a second instance cannot be used as a back door.

This is the failure the check exists to prevent, stated plainly: if 1.0.0 could
be replaced, every project whose `zeb.lock` pinned it would report a *tampered
dependency* the next time it verified. A republish and an attack would be
indistinguishable.

Bump and publish:

```json
{ "package_id": "invoice-tools", "version": "1.1.0", "…": "…" }
```

```json
{
  "ok": true,
  "version": {
    "package_id": "acme.invoice-tools",
    "version": "1.1.0",
    "artifact_rel_path": "services/hub-default/packages/acme.invoice-tools/versions/1.1.0/artifact.json",
    "artifact_sha256": "…different bytes, different digest…",
    "created_at": 1755648000
  }
}
```

Two release documents now exist side by side. 1.0.0 is untouched and Dana's
copy still verifies.

### 6.2 Fixing a typo without a version bump

**Runs today.** This is the decision the presentation split was made *for*.

A comma in the long description is presentation, presentation lives on the
mutable `HubAssetPackage` row, so correcting it is a write to that row and
touches no release and no digest.

```bash
curl -s -b /tmp/zf.txt -X PATCH -H "Content-Type: application/json" \
  -d '{"summary":"Webhook to invoice PDF.","publisher_token":"zfmt_mkt_4f1c9a2b_..."}' \
  http://studio.example/api/projects/acme/billing/hub/assets/invoice-tools/presentation
```

```json
{
  "ok": true,
  "presentation": {
    "summary": "Webhook to invoice PDF.",
    "description_md": "…unchanged…",
    "image_url": "/api/hub/remote/assets/acme.invoice-tools/media/cover.webp",
    "gallery": { "cover": { "kind": "image", "media_name": "cover.webp", "alt": "" } },
    "media": [ { "name": "cover.webp", "role": "cover", "…": "…" } ]
  }
}
```

`update_asset_presentation` is authorised by the same publisher token as
publish, and every field of the body is optional: an omitted field is left
alone. Nothing in it reads or writes a `HubAssetVersion` or an artifact under
`packages/`, which is the property the split exists for.

The publish path follows the same rule. `publish_asset` used to set
`summary: String::new()` and `description_md: String::new()` unconditionally and
rebuild `image_url`, `media`, and `gallery` from `image_file_path` alone, so
publishing 1.1.0 erased the long description and dropped the cover unless it was
re-supplied. It now reads the existing package row first and carries forward
what the publish does not carry; a supplied cover replaces the cover entry and
leaves other media and gallery items alone.

---

## 7. A package with a referenced artifact

**Partly runs.** The format, the resolver, and one channel work. The producer
and two channels do not.

The third entry of the §1.5 fixture references 32 MB of WASM by digest. Where
those bytes come from is the channel's answer, never the document's.

### 7.1 Installing from a file, with the artifact beside it

**Runs today** in the service layer.

```text
/tmp/pkg/
  package.json                 the HubPackage document
  artifacts/
    9f2bc14a…b9a9              the 32 MB module, named by its own digest
```

`HubArtifactChannel::local("/tmp/pkg")` resolves
`<base>/artifacts/<sha256>`, checks the file is a regular file, checks its length
against the entry's `size_bytes`, reads it, and hashes it. Only if all four pass
does the byte reach `prepare_hub_install_entries`, and only if *every* entry
passes does the first write happen.

The failure messages are specific on purpose:

| Code | Message |
| --- | --- |
| `HUB_ARTIFACT_MISSING` | `file 'wasm/core.wasm' references artifact 9f2b…, which this channel does not have` |
| `HUB_ARTIFACT_SIZE_MISMATCH` | `file 'wasm/core.wasm' declares 33554432 bytes but artifact 9f2b… is 118 bytes` |
| `HUB_ARTIFACT_DIGEST_MISMATCH` | `file 'wasm/core.wasm' expects artifact 9f2b…, but the stored bytes hash to 4c1a…` |
| `HUB_ARTIFACT_TOO_LARGE` | `file 'wasm/core.wasm' declares … bytes, over the 536870912 byte limit for a referenced artifact` |

Copy that directory to a different repository and it still resolves, because
nothing in the document names where it was published from.

> **Not built: no route reaches this.** `install_local_node_bundle_document` and
> `review_local_node_bundle_document` are the only callers of the local file
> channel, and outside the test module nothing calls them. The API route
> `POST .../nodes/install` takes the document as a JSON *body*
> (`LocalNodeBundleRequest.artifact`) and therefore uses
> `HubArtifactChannel::unresolvable`, so a referenced entry through that route
> fails with:
>
> ```json
> { "ok": false, "error": "HUB_ARTIFACT_UNRESOLVED: file 'wasm/core.wasm' references artifact 9f2b… and a bundle supplied as a document body says nothing about where its artifacts live; install it from its file instead" }
> ```
>
> The message tells you to install it from its file. There is currently no
> command or endpoint that does.

### 7.2 Installing from this instance's Hub

**Runs today, if the artifact is already in the store.**
`HubService::artifact_store()` points at
`<data_root>/services/hub-default/artifacts/`, so a Hub install resolves
`artifacts/<sha256>` from there. Two packages naming one runtime read one file.

> **Not built: nothing puts a *content* artifact in that store.**
> `store_artifact` has exactly one caller, and it stores cover images.
> `publish_asset` carries every entry of `spec.files` inline, so no publish
> produces a referenced package. Referenced packages exist today only if
> hand-authored.

### 7.3 Installing over HTTP

> **Not built, and honestly so.** A remote pack and a project bundle over HTTP
> get `HubArtifactChannel::unresolvable`, and every referenced entry through them
> is refused with `HUB_ARTIFACT_UNRESOLVED`. There is no artifact endpoint, no
> size-bounded streaming read, and no cache for bytes two packages share. A
> channel that quietly wrote an empty file in place of a 32 MB module would be
> far worse than one that says it cannot, so the refusal is the deliberate
> current state and not an oversight.

The net effect across §7: **referenced artifacts are a working format and a
working local resolver with no producer and no reachable consumer.** The 32 MB
WASM module that motivated the design still cannot be distributed end to end.

---

## 8. Retraction

**Partly runs.** Retraction is per package; per-release retraction is not built.

The act: a publisher withdraws the bytes while the coordinates survive.

| Survives | Removed |
| --- | --- |
| `acme.invoice-tools` and `1.1.0` as a listed, explorable coordinate | the release document |
| title, description, publisher, presentation | referenced artifacts only this release used |
| the retracted marker, its timestamp, and its reason | |

```bash
curl -s -X DELETE -H "Authorization: Bearer zfmt_…" \
  -H "Content-Type: application/json" -d '{"reason":"leaked an API key"}' \
  http://studio.example/api/hub/remote/assets/acme.invoice-tools
```

```json
{ "ok": true, "retracted": true, "retracted_versions": 2 }
```

The method is still `DELETE`, because that is what a publisher means, but
`retract_asset_package` performs a retraction: it marks every version row
retracted with the moment and the reason, marks the package row, and only then
removes the artifact files. The markers land before the bytes go, so an
interrupted retraction leaves rows that still point at readable artifacts rather
than releases that read as live and cannot be served.

`acme.invoice-tools@1.0.0` can now never be reused. `enforce_release_immutability`
checks the rows that exist, and the rows still exist, so a republish of a
retracted coordinate is refused:

```json
{
  "ok": false,
  "error": {
    "code": "HUB_VERSION_RETRACTED",
    "message": "acme.invoice-tools@1.0.0 was retracted and its coordinates can never be reused; publish a new version"
  }
}
```

Freeing the coordinates would have been the hole in immutability — a `zeb.lock`
pinning the old digest would report a tampered dependency for content that was
merely republished, the exact failure §6.1 prevents. npm reaches the same place
by blocking a name and version forever after unpublish; Cargo by never deleting
and only yanking.

A retracted release stays explorable. `GET .../assets/{id}/{version}` answers
`200` with the version described, the presentation intact, `artifact: null`, and
the retracted marker saying why; `.../artifact` and the install paths answer
`410 Gone` with `HUB_VERSION_RETRACTED` and the publisher's reason. Listings keep
the package and report `latest_version: ""` when nothing installable is left.

Publishing a *new* version of a retracted package is allowed and clears the
package-level marker: the package has live content again. The retracted version
rows keep their own markers, and `retract_hub_asset_version` is the only writer
of them and never clears one.

> **Not built: per-release retraction.** Retraction takes a `package_id`, so
> 1.1.0 cannot be withdrawn while 1.0.0 stays.
>
> **Not built: reference counting for presentation.** The release artifacts are
> removed but a cover in the content-addressed store is not, because the store is
> shared and nothing counts references. This is deliberate here — a retracted
> package stays explorable, and its cover is part of what stays.

---

## 9. Adding a second hub

**Partly runs.**

The premise is that hubs are pluggable the way apt repositories are:
`hub.telkomsel.com`, `hub.mit.edu`, `github.com/google/zebflow-hub`. A repository
is a transport, not a format, and answers two questions: `list()` and
`fetch(id, version)`.

### 9.1 What runs today

A project or platform repository row pointing at another Zebflow instance:

```bash
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d '{"title":"Acme Hub","base_url":"https://studio.example","remote_owner":"acme","remote_project":"billing","read_token":"zfmt_…"}' \
  http://localhost:10610/api/projects/dana/ops/hub/repositories
```

Then list and install from it:

```text
GET  /api/projects/dana/ops/hub/remote/assets
POST /api/users/dana/hub/install          platform scope: creates a project
```

The platform-scope install returns a project card, not a file list, because it
made a project:

```json
{ "ok": true, "project": { "owner": "dana", "project": "invoice-tools", "title": "Invoice Tools", "…": "…" } }
```

That act is irreversible in the §0 sense: there is no uninstall for it. The
result is Dana's project, editable as source, which is the consumption-mode
promise — you can run it, and you can open the same installation and change it.

### 9.2 What does not

> **Not built: one repository interface.** `ProjectHubRepository` carries
> `base_url`, `remote_owner`, `remote_project`, and `read_token` — four fields a
> static repository and a local file do not have. Every remote path builds a URL
> with `remote_hub_url` and expects another Zebflow instance's API on the other
> end. "A new source is a fetcher, not a new installer" is the design; the code
> has one hardwired fetcher. `distribution.md` §2 says this generalisation should
> happen *before* a second remote source is added rather than after, and it has
> not happened.

> **Not built: static repositories.** No `zebflow-repository.json` index is read
> anywhere. A plain HTTPS location serving an index plus package documents is
> designed and absent.

> **Not built: the CLI.** `zeb install`, `zeb node install`, `zeb add`,
> `zeb publish` are a proposal in `distribution.md` §0a. The only distribution
> verb the binary has is `zebflow run`, which materialises a project or Hub asset
> and serves it. Every command in this document is `curl` for that reason.

### 9.3 What travels when Acme publishes to someone else's hub

Publishing outward is a different act from publishing locally, with a different
consent requirement, and it is the only path that carries presentation:

```json
{
  "package_id": "invoice-tools",
  "version": "1.1.0",
  "title": "Invoice Tools",
  "description": "Webhook to invoice PDF, with the rendering page.",
  "summary": "Invoice generation for Zebflow projects",
  "description_md": "## Invoice Tools\n\nInstall, point it at a Postgres connection…",
  "media": [
    { "name": "cover.webp", "role": "cover", "content_type": "image/webp",
      "encoding": "base64", "size_bytes": 41220, "sha256": "…", "content": "UklGR…" }
  ],
  "gallery": { "cover": { "kind": "image", "media_name": "cover.webp", "alt": "" }, "items": [] },
  "visibility": "public",
  "tags": ["billing", "pdf"],
  "source_owner": "acme",
  "source_project": "billing",
  "source_kind": "pipeline_with_dependencies",
  "source_ref": "pipelines/api/invoice.zf.json",
  "artifact": { "apiVersion": "zebflow.com/v1", "kind": "HubPackage", "…": "…" }
}
```

Presentation rides *beside* `artifact`, never inside it, because `artifact` is
the digest-pinned release and a listing detail must not be able to change it.
The receiving instance validates the images, stores them content-addressed, and
records only digests.

**One honest correction to "provenance is never published."** The four `source_*`
fields are on the wire. They tell the receiving hub which project on the
publishing instance the package came from, and the receiving hub keeps them in
its own version row. What is true is narrower and still worth having:

- provenance is never in the **release document**, so it does not travel with a
  package copied to a third place;
- the public read API exposes `source_kind` (what type of export it was) and
  does **not** expose `source_owner`, `source_project`, or `source_ref`.

So the accurate statement is "provenance never enters the release and is not
served publicly", not "provenance never leaves the instance". Worth restating in
the README, because the current wording is stronger than the code.

---

## 10. What these scenarios exposed

Writing the usage out surfaced these. They are ordered by how much they change
what a user actually gets.

**1. The safety review's coverage depends on a folder name the receiver types.**
`is_reviewed_pipeline_path` matches on the *installed* path, and
`install_entry_rel_inside_folder` strips the entry's own `pipelines/` prefix. A
receiver who chooses a target folder that does not start with `pipelines/` gets a
review that scans none of the pipelines and reports `risk_level: "low"` for a
package with a public endpoint, a credential requirement, and a database effect.
`"/billing"` is scanned; `"billing"` is not. The publish-side review, which sees
the source paths, reports everything. Two reviews of one package disagree, and
the one the receiver reads is the weaker one. (§2.1a)

**2. The Hub add route never runs the review.** `install_asset` verifies the
artifact digest and installs. It does not call `review_artifact_payload` and does
not consult `installable`. Only `install_node_bundle_payload`, on the local
document paths, refuses on a violation. `distribution.md` §3 says every channel
that installs into a project runs the same review; one of them does not run it at
all. (§2.2, §4)

**3. Presentation was mutable in the design and immutable in practice — fixed.**
There was no endpoint to edit a package row's presentation, and the only writer,
`publish_asset`, cleared `summary` and `description_md` on every publish and
dropped the cover unless `image_file_path` was re-supplied. `PATCH
.../hub/assets/{package_id}/presentation` now writes the row and no release, and
a publish carries forward the presentation it does not supply. (§6.2)

**4. `zeb.lock` does not record a release digest.** The node bundle entry's
`integrity` is a digest of the installed directory tree. Nothing records the
sha256 of the `HubPackage` document, so a receiver cannot prove offline that the
release they hold is the release that was published; they can only confirm the
document is internally consistent with the digest the same source handed them.
`distribution.md` §4's integrity row is not yet true for this kind. (§3, §5.4)

**5. Delete freed version strings for reuse — fixed.** Whole-package delete
dropped every row, and immutability is enforced against rows that exist, so
after a delete any prior version could be republished with different content —
the same lockfile-invalidating failure §6.1 exists to prevent, reachable in two
steps. Delete is now retraction: the rows survive marked, the bytes go, and a
retracted coordinate is refused with `HUB_VERSION_RETRACTED`. Per-release
retraction is still not built. (§8)

**6. Referenced artifacts have no producer and no reachable consumer.** The
format accepts them, `HubArtifactChannel::local` resolves and verifies them, and
`store_artifact` can store them. But `publish_asset` carries everything inline,
the only route that accepts a local document uses the unresolvable channel, and
HTTP fetching does not exist. The 32 MB WASM module in the golden fixture cannot
be distributed by any path that currently has an entry point. (§7)

**7a. Unrecognised token scopes were dropped silently — fixed.**
`normalize_scopes` lowercased and filtered to the three known scopes, so a token
created with `["read","publish"]` stored `[]` and failed much later, at a
different endpoint, as `HUB_TOKEN_FORBIDDEN / scope missing`. An unknown scope is
now a `HUB_TOKEN_SCOPE_INVALID` request error naming the value and the accepted
set, and a token with no usable scope is refused too.

**7. One route reports refusals as HTTP 200.**
`api_install_local_node_bundle` and `api_review_local_node_bundle` return
`{"ok": false, "error": "CODE: message"}` with a 200 status and a flat string,
where every other Hub route returns a status code and a structured
`{"error": {"code", "message"}}`. That is the route that enforces violations,
so the strictest check has the least legible failure. (§4.1)

**8. "Provenance is never published" is stronger than the code.** The four
`source_*` fields travel on a remote publish and are kept by the receiving hub;
`source_kind` is then served publicly. The defensible claims are that provenance
never enters the release document and that owner, project, and ref are not served
publicly. (§9.3)

**9. Two names for one act.** `add` performs an install for `node_bundle` and
an add for everything else, on the same route. The verbs are carefully
distinguished in `distribution.md` and collapsed at the endpoint. Either the
route grows a sibling or the docs must say plainly that the asset kind picks the
verb — this document chose the latter, in §0.

---

## Sources

Every claim above was read from:

| | |
| --- | --- |
| Contract and limits | `src/contracts/kinds/hub_package.rs` |
| Canonical encoding | `src/contracts/io.rs`, `src/contracts/metadata.rs` |
| Golden document | `tests/fixtures/contracts/hub-package/v1-complete.json` |
| Publish, install, channels, immutability | `src/platform/services/hub.rs` |
| Safety review, warnings and violations | `src/platform/policy/package.rs`, `src/platform/policy/report.rs` |
| Routes, request bodies, status mapping | `src/platform/web/mod.rs` |
| Lock entries | `src/platform/services/dependency_lock.rs` |
| Umbrella contract | `docs/contracts/distribution.md` |
