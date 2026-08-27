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

The three verbs are not synonyms and this document uses them strictly. The
table — scope, landing area, editability, removal, both install scopes — is
[`distribution.md` §1a's](../../distribution.md#1a-the-verbs) and is not
duplicated here.

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
| `studio.example` | the publishing instance, exposing a Public Hub |
| `acme` | a publisher registered on that Hub |
| `dana` | someone on a second instance who wants the package |
| `mirror.example` | a third instance holding a copy, which nobody has audited |

---

## 1. A publisher ships their first release

**Runs today.**

> **2026-08-27.** The local store is now sealed immutable — seeded by the
> release, no publisher tokens (`distribution.md` §1b) — and publisher publish
> is re-scoped to the Public Hub. The run below is dated evidence of the
> publish core, which the seed and the Public Hub both use. Wherever a run in
> this document lands bytes in `services/hub-default/`, that is the pre-split
> store; the contract now names `services/hub-local/` (blessed shelf) and
> `services/hub-public/` (the one publish target). Code catch-up owed.

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
`artifact`. Every file here is a few kilobytes, and a publish carries anything up
to 1 MiB. A file over that is referenced by digest instead — see
[§7](#7-a-package-with-a-referenced-artifact).

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

Seven spec fields, no eighth, and this document declares six of them: there is
no cover, no long description, no publisher name, no "which project did this
come from". A release carries what installing it requires. `title` and the
one-line `description` are the kept exception, so this file handed over on a USB
stick still says what it is.

The seventh, `layout`, is absent here and is optional everywhere. It records the
repository directories the publisher's `rel_path` values were relative to, which
an installer needs to place them into a project that keeps its own files
somewhere else. A node bundle's paths are internal to the bundle, so this one
has nothing to record. Absent means the platform default, which is why a package
published before the field existed still installs.

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

An empty body means "use the default folder", which is `hub/{package_id}` inside
the receiving project's source root — `pipelines/hub/{package_id}` for a project
that declares no layout, and `src/hub/{package_id}` for one declaring
`source: src`.

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
    "pipelines_registered": ["hub/acme.invoice-tools/api/invoice.zf.json"],
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

The install paths are not the export paths. Each entry is translated from the
layout the release records to the layout the receiving project declares:
`HubInstallPlacement` strips the *publisher's* source root and joins the
remainder under the install root, so the exported
`pipelines/api/invoice.zf.json` lands at
`pipelines/hub/acme.invoice-tools/api/invoice.zf.json` rather than doubling the
segment — and lands at `src/hub/acme.invoice-tools/api/invoice.zf.json` in a
project that keeps its source in `src`. Assets and docs go to the receiver's
asset and docs directories, under the same folder name; anything else stays
inside the package's folder.

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
    "install_root": "pipelines/billing",
    "files_added": [
      "pipelines/billing/api/invoice.zf.json",
      "pipelines/billing/components/invoice-line.tsx",
      "pipelines/billing/pages/invoice.tsx"
    ],
    "pipelines_registered": ["billing/api/invoice.zf.json"],
    "nodes_used": ["n.pg.query", "n.trigger.webhook", "n.web.response"],
    "credentials_required": ["billing_pg"],
    "external_urls": ["https://fonts.example/inter.css"],
    "database_effects": ["n.pg.query"],
    "public_endpoints": ["/invoice", "webhook trigger"],
    "warnings": [],
    "violations": [],
    "installable": true,
    "risk_level": "high"
  }
}
```

**Same package, same bytes, same findings.** That was not always true, and the
way it failed is worth keeping:

> **Fixed.** A target folder used to be pulled inside the source root only when
> the receiver typed a leading slash, so `"/billing"` installed at
> `pipelines/billing` and `"billing"` installed at `billing`. Nothing under
> `billing` is a pipeline by the discovery rule, so the review parsed none of
> them and reported `risk_level: "low"` with every finding list empty, while the
> install registered nothing and the pipelines silently never ran. Typing a
> slash is not consent to be reviewed. Placement follows the project's declared
> layout now, so all three spellings — `""`, `"billing"`, `"/billing"` — review
> and register identically.

Note that `pipelines_registered` reports the pipeline's **identity** — its path
inside the source root — rather than the repository path the install writes, so
a review can be compared against the result it predicts.

What Dana should read off this before accepting:

| Field | What it means for Dana |
| --- | --- |
| `credentials_required: ["billing_pg"]` | this package will not work until Dana creates a credential of that kind. The package did not bring one, and cannot. |
| `public_endpoints` | installing this puts a route on Dana's project that the internet can reach. |
| `database_effects` | it runs SQL against a connection Dana supplies. |
| `files_overwritten: []` | nothing Dana already has is being replaced. A non-empty list here adds 2 to the risk score, and is the one finding that is about Dana's project rather than the package. |
| `installable: true` | nothing refuses. `false` would mean the Add button must not be offered at all. |

### 2.1b The same package, into a project laid out differently

**Runs today.**

Acme keeps its source in `pipelines/`. Dana's `ops` project declares
`spec.layout.source: src`. The release records the layout its paths were
produced by, so the install translates rather than guesses:

```json
{
  "review": {
    "target_folder": "",
    "install_root": "src/hub/acme.invoice-tools",
    "files_added": [
      "src/hub/acme.invoice-tools/api/invoice.zf.json",
      "src/hub/acme.invoice-tools/components/invoice-line.tsx",
      "src/hub/acme.invoice-tools/pages/invoice.tsx"
    ],
    "pipelines_registered": ["hub/acme.invoice-tools/api/invoice.zf.json"]
  }
}
```

The publisher's `pipelines/` is removed rather than carried along, and the
pipeline's identity is the same string in both projects, because identity is the
path inside the source root and the root lives only in `zebflow.yaml`.

### 2.1c Adding a whole project as a folder

**Runs today.**

`spatial-blogging` is a complete project: pipelines, pages, styles, docs, seeds,
and images. At platform scope it installs as a *new project*. From inside `ops`,
adding it makes it a folder of Dana's own source.

```bash
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" -d '{}' \
  http://studio.example/api/projects/dana/ops/hub/assets/labs.spatial-blogging/1.0.0/add
```

Its parts do not all land in one directory, because they cannot: a pipeline is
only discovered inside the source root, and an image is only served from the
asset directory. What it occupies is one *name*, in each area it reaches.

```text
users/dana/ops/repo/
  src/hub/labs.spatial-blogging/
    blog/feed.zf.json          registered as a pipeline definition
    pages/feed.tsx
    zebflow.yaml               the publisher's, kept inert beside its own files
    schemas/sekejap/schema.json
  src/assets/hub/labs.spatial-blogging/
    logo.svg                   served at /assets/dana/ops/hub/labs.spatial-blogging/logo.svg
  docs/hub/labs.spatial-blogging/
    README.md
```

Dana's own `zebflow.yaml` and `schemas/` are untouched: one project holds one of
each, so the package's copies are kept readable rather than written over Dana's.
The review lists every destination above, and removal is deleting those
directories — three, not one, which is the price of the parts actually working.

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
    "pipelines_registered": ["hub/acme.invoice-tools/api/invoice.zf.json"]
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

The add route does not call `review_artifact_payload`, and does not need to.
`refuse_prepared_install_violations` runs inside `install_artifact_payload_from`,
after every entry's bytes are resolved and before the first write, so a violation
refuses a Hub add exactly as it refuses a locally supplied node bundle. The two
are separate calls reached on separate paths; what makes them agree is that both
build the review's entries through `PackagePolicyEntry::from_bytes`, which is the
only thing that decides what a reviewer can see.

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
users/dana/ops/data/hub/nodes/acme.pdfkit/
  definition.json
  icon.svg
  functions/render.zf.json
  wasm/pdfkit.wasm
```

and `repo/zeb.lock` gained a declaration. The lock shown is the 2026-08-27
**target contract** — sealed source vocabulary and Public Hub store scoping
(`../dependency-lock/README.md`); code catch-up is owed:

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "DependencyLock",
  "metadata": { "name": "ops" },
  "spec": {
    "rwe": { "libraries": {} },
    "nodes": {
      "bundles": {
        "acme.pdfkit": {
          "version": "2.0.0",
          "source": "hub.public",
          "source_id": "acme.pdfkit@2.0.0",
          "entry": "nodes/acme.pdfkit/definition.json",
          "integrity": "sha256:…64 hex, illustrative…",
          "definitions": ["n.x.acme.pdf.render"]
        }
      }
    }
  }
}
```

That is the ownership model working: the declaration is declared in `repo/` —
machine-written, per instance-directory Rule 1 — and travels with the git
history; the bytes are machine output in `data/` and are rebuildable from the
declaration. Uninstall
(`DELETE .../nodes/uninstall/{kind}`) removes both.

**Read `integrity` carefully.** It is
`directory_tree_sha256(data/hub/nodes/acme.pdfkit)` — a digest of the installed
tree, computed after the install. It is **not** the sha256 of the `HubPackage`
release document. So the lock detects local tampering with installed bytes, and
does not by itself let you prove the release you have is the release that was
published. That proof is §5, and the fact that the lock does not carry it is
[§10](#10-what-these-scenarios-exposed).

---

## 4. A package that cannot be installed by anyone

**Runs.** The tier, the refusal, and the first detector exist, and every channel
that writes into a project passes through a gate that honours it.

A warning asks "do you accept this effect?". A violation says "this will not be
installed, whoever approves it". They are different questions and the review
answers them in different fields.

### 4.1 The first detector: a pipeline the review cannot read

The contract already closed the crude version of this hole: an entry may declare
`encoding` only as `text`, `base64`, or absent, because an entry labelled `utf8`
used to be unreadable to the scanner and still written verbatim by the installer.

The subtler version survives the contract. An entry landing at a reviewed
pipeline path can be **referenced** rather than carried, and its bytes then come
from the channel. The review now resolves references itself — `entry_review_bytes`
takes the same `HubArtifactChannel` the install would use — so a reference the
channel *can* satisfy is read and scanned exactly like carried bytes. The
violation fires when it cannot: an unresolvable channel, a digest or size
mismatch, base64 that does not decode, or bytes that are not UTF-8.

The distinction that makes this a violation rather than a clean result: an empty
`content` means an empty file, whereas an unreadable entry means the reviewer was
handed nothing while the install would still write something. Those must never
produce the same verdict, so `PackagePolicyEntry` carries an `unreadable` reason
and a non-empty reason at a pipeline path is a violation.

The reason is recorded in one place. `PackagePolicyEntry`'s fields are private,
`from_bytes` is the only thing that turns bytes into readable text or a recorded
reason they are not, and `unresolved` is the only thing that records bytes that
never arrived. Bytes that are not text are unreadable wherever they came from —
which refuses nothing on its own, because the scan escalates `unreadable` only
where those bytes were going to be read as a pipeline. A package carrying an icon
or a font installs.

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

**Where each half of this is reachable.** The refusal above is the review's: it
runs before anything is fetched for the install, so it is the one that answers a
reference the channel cannot satisfy. `refuse_prepared_install_violations` sits
further down, after `prepare_hub_install_entries` has resolved every entry, so an
artifact that could not be obtained has already failed there and this gate only
ever sees bytes. What it judges about them is whether they are text at a path the
scan reads as a pipeline — and even that is refused earlier for a repository
pipeline path, where `validate_prepared_pipeline_sources` rejects bytes that will
not decode as a pipeline graph, with `HUB_INSTALL`. The gate's own
`HUB_INSTALL_REFUSED` is what stands behind a pipeline the *layout* does not
recognise: a node bundle's function pipeline, which is a pipeline because the
bundle's manifest names it and for no other reason.

> **Not built: the local node bundle refusal returns HTTP 200.**
> `api_install_local_node_bundle` answers `{"ok": false, "error": "CODE:
> message"}` with a 200 status and a flat string, unlike every other Hub route,
> which returns a status code and `{"error": {"code", "message"}}`. A client
> checking the status code will read a refusal as a success. The two install
> refusal codes are unmapped in `hub_api_error` for a related reason, and answer
> 500.

### 4.2 What is deliberately not a violation

Everything else. Reaching an external URL, requiring a credential, exposing a
webhook, running SQL, shipping 40 MB of seed data — all warnings and effect
lists, all acceptable, all Dana's decision. The tier is reserved for findings
precise enough to refuse without a false positive, and there are three so far:
this one, a file type no project accepts, and a bundle declaring a node kind this
build already provides. Growing it is deliberate slowness, not neglect: a
violation that misfires
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

**Runs.** Publishing produces one, three channels resolve one, and the review
reads its real bytes on every one of them.

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
> channel, and outside the test module nothing calls them — see
> [§7.4](#74-what-is-still-not-built).

### 7.2 Publishing one, and installing it from this instance's Hub

> **2026-08-27.** Same re-scope as [§1](#1-a-publisher-ships-their-first-release):
> publish now targets the Public Hub store `services/hub-public/`; the run
> below is dated evidence against the pre-split local store (code catch-up
> owed).

**Runs today.** A publish carries a file up to 1 MiB and references anything
larger. The bytes go into `<data_root>/services/hub-default/artifacts/<sha256>`
through `store_artifact`, and the release records only the digest:

```bash
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d '{"source_type":"pipeline_with_dependencies",
       "source_ref":"pipelines/api/big-hello.zf.json",
       "package_id":"big-hello","version":"1.0.2",
       "visibility":"public","publisher_token":"zfmt_..."}' \
  http://studio.example/api/projects/acme/billing/hub/assets/publish
```

The release that comes out is 1 028 bytes and names a 1 120 873-byte pipeline:

```json
{ "rel_path": "pipelines/api/big-hello.zf.json", "kind": "json",
  "size_bytes": 1120873, "reason": "primary pipeline",
  "artifact": { "sha256": "e245c30a…f101", "media_type": "application/octet-stream" } }
```

Installing it locally resolves `artifacts/<sha256>` from that store. Two
packages naming one runtime read one file.

The decision is made **after** the publish review and **before** the store is
written. Referencing changes how bytes are supplied and never which bytes they
are, so the review reads the same content either way; and a publish refused
after the review leaves the artifact store as it found it.

### 7.3 Installing over HTTP

**Runs today.** The hub that serves a release also serves its artifacts:

```text
GET /api/hub/remote/assets/{package_id}/{version}/artifacts/{sha256}
```

The route is namespaced by the release rather than being a bare content address,
so the bytes inherit that release's visibility and its retraction, and a digest
the release does not name is `404` even when the store holds it for another
package.

A remote install fetches every reference before it reviews, so the review reads
the real bytes:

```json
{ "ok": true, "review": {
    "nodes_used": ["n.trigger.webhook", "n.web.response"],
    "external_urls": ["https://art.example.com/feed.json"],
    "public_endpoints": ["/big-hello", "webhook trigger"],
    "violations": [], "installable": true, "risk_level": "high" } }
```

Every one of those findings came out of the referenced artifact. A review that
scanned a placeholder would report none of them, which is exactly the bypass
found on 2026-08-19.

**What the channel refuses.** Redirects are not followed; the declared
`size_bytes` bounds the read, so a `Content-Length` that disagrees is refused
before the body and a body that keeps arriving is cut off; the digest is
verified as the bytes stream, and a file appears at the content address only
after it matches. A hub that answered is distinguishable from a request that
failed:

| Code | When |
| --- | --- |
| `HUB_ARTIFACT_MISSING` | the hub answered 404 or 410 |
| `HUB_ARTIFACT_FORBIDDEN` | the hub answered 401 or 403 |
| `HUB_ARTIFACT_REDIRECTED` | the hub answered 3xx |
| `HUB_ARTIFACT_FETCH_FAILED` | the request did not complete |
| `HUB_ARTIFACT_SIZE_MISMATCH` | more or fewer bytes than the release declares |
| `HUB_ARTIFACT_DIGEST_MISMATCH` | the bytes are not the bytes the release names |

Verified bytes are kept in this instance's own artifact store, so reviewing a
package and then installing it costs one fetch. A cached file whose bytes do not
hash to its own name is treated as absent, so a corrupted cache heals rather
than refusing that release forever.

### 7.4 What is still not built

> **Pushing a referenced release outward.** `import_remote_asset` reviews an
> inbound document against the receiving hub's own store, and the remote publish
> route moves one document and nothing beside it. A publisher pushing to someone
> else's hub therefore still has to keep every file carried.

> **A document supplied as a request body.** `POST .../nodes/install` takes the
> document as JSON (`LocalNodeBundleRequest.artifact`) and so uses
> `HubArtifactChannel::unresolvable`. A referenced entry through that route
> fails with `HUB_ARTIFACT_UNRESOLVED` and is told to install from its file
> instead. Nothing outside the test module calls the local file channel, so
> there is still no command or endpoint that does.

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
is a transport, not a format, and answers three questions: `list()`,
`fetch(id, version)`, and `artifact(id, version, digest)`.

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
POST /api/users/dana/hub/install/review   what that install would do, doing none of it
POST /api/users/dana/hub/install          platform scope: creates a project
```

The platform-scope install returns a project card, because it made a project,
alongside an `install` report of what it did:

```json
{
  "ok": true,
  "project": { "owner": "dana", "project": "invoice-tools", "title": "Invoice Tools", "…": "…" },
  "install": { "files_written": ["…"], "pipelines_registered": ["…"], "pipelines_activated": ["…"], "…": "…" }
}
```

Those `install` fields are the same fields the review answered with, read out of
the same plan, so the two can be compared line for line.

That act is irreversible in the §0 sense: there is no uninstall for it. The
result is Dana's project, editable as source, which is the consumption-mode
promise — you can run it, and you can open the same installation and change it.

### 9.2 What does not

> **Built: one repository interface.** `HubRepositoryChannel` now fronts the
> five package channels — `distribution.md` §2, "One repository interface";
> transfer archives and git remotes move projects outside it.

> **Built: static repositories.** Live-proven — `kinds/hub-repository-index/README.md`.

> **Partly built: the CLI.** `zeb install` exists and is the client for the
> platform-scope install below: it reviews, prints, asks, and then calls the
> same HTTP route. `zeb remove`, `zeb list`, and `zeb run` exist too. The
> project-scope verbs — `zeb node install`, `zeb hub add`, `zeb hub publish` —
> remain a proposal in `distribution.md` §0a, which is why every command in this
> document is `curl`.

### 9.3 What travels when Acme publishes to someone else's hub

> **2026-08-27.** Same re-scope as [§1](#1-a-publisher-ships-their-first-release):
> every publish now lands in a Public Hub store `services/hub-public/` — this
> instance's or another's — so "publishing locally" below is dated evidence of
> the pre-split local store (code catch-up owed). What travels on the wire is
> unchanged.

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

**1. The safety review's coverage depended on a folder name the receiver types —
fixed.** The review matched on the *installed* path, and a target folder was
pulled inside the source root only when the receiver typed a leading slash. So
`"/billing"` was scanned and `"billing"` was not: a package with a public
endpoint, a credential requirement, and a database effect reviewed as
`risk_level: "low"` with every finding list empty, and registered nothing.
Placement follows the project's declared layout now, and every spelling of a
folder reviews and registers the same. (§2.1a)

**2. The Hub add route never ran the review — fixed.** `install_asset` verified
the artifact digest and installed, without consulting `installable`.
`refuse_prepared_install_violations` is now the gate every channel passes
through, and it reviews the *prepared* entries — bytes already resolved, paths
already the destinations that will be written — so the review reads exactly what
the install is about to place. It read those bytes differently from every other
gate until they were given one reader; see §4.1. (§2.2, §4)

**2a. A package's paths meant nothing without the publisher's layout — fixed.**
A `rel_path` is relative to the publishing project's directories, and the
installer stripped the *receiver's* source root off it, which is right only when
the two layouts agree. A package from a `pipelines/` project installed into a
`source: src` project kept `pipelines/` as a dead segment, and its images landed
outside the directory the asset route serves. `spec.layout` records the
publisher's layout and `HubInstallPlacement` translates against it.

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

**6. Referenced artifacts had no producer and no reachable consumer — fixed.**
The format accepted them and `HubArtifactChannel::local` verified them, but
`publish_asset` carried everything inline and HTTP fetching did not exist, so
the half of the rule where a review bypass had been found ran only in unit
tests. A publish now references anything over 1 MiB, a hub serves
`remote/assets/{id}/{version}/artifacts/{sha256}`, and a remote install fetches
and verifies every reference before it reviews — so the review reads the real
bytes and the install writes nothing until all of them pass. What remains is a
document handed over as a request body, which names no origin to fetch from, and
pushing a referenced release outward to someone else's hub. (§7)

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
`{"error": {"code", "message"}}`. `HUB_INSTALL_REFUSED` and
`HUB_REMOTE_INSTALL_REFUSED` are unmapped in `hub_api_error` and answer 500, so
a refusal on the other install paths reads as a server fault. The strictest
check has the least legible failure. (§4.1)

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
