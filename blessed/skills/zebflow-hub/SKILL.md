---
name: zebflow-hub
description: Adding a package from the Zebflow Hub into a project, or publishing one — pipeline, template, folder and project bundles, node bundles, zeb/* libraries, skills. Use before any Add, install, clone or publish; covers the safety review, what lands where, drafts, zeb.lock, versions and presentation.
license: MIT
metadata:
  version: "1"
---

# The Hub: add, own, publish

The Hub distributes inspectable source, not installs. Adding most packages is
a **copy into the project** that the project then owns; two kinds are real
dependencies. Facts: `help(topic="guide/hub")`, `guide/hub/how-it-works`,
`guide/hub/nodes`; contract `docs/contracts/kinds/hub-package`.

| Kind | Add means | Tracked after? |
|---|---|---|
| `pipeline_bundle`, `template_bundle`, `folder_bundle`, `project_bundle` | files copied into the project (source under `hub/<package>/` unless the review says otherwise; assets under `static/`, docs under `docs/`) | no — it is your source now |
| `skill` | `skills/<name>/` copied into the project; shadows a blessed skill of the same name | no — edit it freely |
| `node_bundle` | node kinds `n.x.<bundle>.<node>` become available; materialized under `data/hub/nodes/` | yes — `zeb.lock` |
| `rwe_library` | a `zeb/*` runtime library the project may load | yes — `zeb.lock` and `zebflow.yaml` |

`zeb/ui` components are not hub packages: `install_ui_components` clones one
from the platform into `shared/ui/`.

## Before you add anything

1. **Review first, add second.** `POST /api/projects/{o}/{p}/hub/assets/{package}/{version}/review`
   (or the Review button) lists every file the package writes, the node
   kinds it needs, credentials it expects, outbound URLs, database effects,
   schedules, public endpoints, large files, seed data, and a risk level.
   Read the whole list. A package that carries a credential, a schedule you
   did not ask for, or an outbound URL you do not recognise is a question for
   the user, not a click.
2. **Destinations.** Add writes exactly the destinations the review
   reported. Check they do not overwrite files the project already has.
3. **Drafts.** Pipelines that arrive land as `draft`. Nothing serves until
   someone activates it — do not bulk-activate a bundle you have not read.
4. **Schema.** A project bundle may include schema and seed SQL
   (`include_schema`) and may replay it into the project's stores
   (`execute_schema`). `execute_schema: false` installs the files and runs
   nothing; use it when the project already has data.
5. **Trust.** Packages from a hub the project's owner registered are as
   trustworthy as that hub. A remote hub's packages are reviewed the same
   way; the review is the trust boundary, not the source.

After adding: `pipeline_list status=all` (what arrived, all `draft`),
`file_list glob="hub/**"`, then read the package's README before activating.
A cloned pipeline that references a credential id from the publisher's
project will fail here until `credential_list` shows an equivalent and the
pipeline is patched.

## Publishing

A release is immutable: `package@version` never changes; correcting anything
is a new version. What a person reads while choosing — summary, description,
cover, gallery — is **presentation**, editable beside the release without a
version bump (`PATCH …/hub/assets/{package}/presentation`).

1. `POST …/hub/publish-preview` (or `publish-sources`) to see what would be
   carried: the files, their layout, the active pipelines, project
   initialization.
2. `POST …/hub/assets/publish-review` — the same safety review a consumer will
   see. Fix what it flags: a credential value in a file, an absolute URL to
   your dev instance, a schedule that should be off, a 40 MB screenshot.
3. Publish with a token whose scope is `hub:publish` (`…/hub/tokens`). A
   `HUB_VERSION_EXISTS` refusal means the version is taken; bump it.
4. Set the presentation: one sentence of summary, a description that says
   what the package assumes (tables, credentials, libraries), a cover.
5. Install it into a fresh project and run its README from the top. That is
   the only acceptance test a package has.

For a **skill**: the folder is `skills/<name>/` with a `SKILL.md` whose
frontmatter has `name` (equal to the folder), `description` (when to use it,
≤ 1024 chars) and `license` (an SPDX id — `MIT`, `Apache-2.0` — or
`proprietary`; a public hub accepts only open licences). Keep the body under
500 lines; references and scripts go beside it. Name the origin when you
derived it: `metadata.derived_from`.

## Never

- add a package without reading its review;
- activate arriving pipelines in bulk;
- publish a package with a credential, a token or a dev URL inside;
- treat a hub as a backup — it holds releases, not your project's data.
