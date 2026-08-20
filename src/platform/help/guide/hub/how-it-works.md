# How Hub Works

Hub is the office-hosted package sharing service for Zebflow work.

## Core ideas

- one Zebflow Hub can publish packages
- another Zebflow project can consume them
- public packages can be browsed without a token
- private access can use scoped Hub tokens
- packages are typed and versioned
- a published version is immutable

## Why this matters

This lets Zebflow share project material natively, without forcing everything
through GitHub repos or plugin packaging first.

## Hub and Add+

Hub and Add+ are related but not identical.

- Hub is a package publishing and sharing authority.
- Add+ is the project-side surface for bringing reusable material into the
  current project.

Add+ may read from several source adapters:

- Built-in Catalog: offline packages or components embedded in the Zebflow
  binary.
- Hub: packages from a Zebflow Hub source.
- Git: packages or source folders from GitHub, GitLab, or another git remote.
- Local: local folders or uploaded package archives.

Zip is local when the archive is selected from the user's machine. If a zip is
downloaded from a URL, the source adapter is the remote system that provided
that URL.

All Add+ sources must be reviewed before they write. The review is intentionally
source-agnostic: it lists added/skipped/overwritten files, executable surfaces,
credentials, external URLs, database/filesystem effects, public endpoints,
schedules, large files, seed data, database initialization, warnings, and risk
level. Built-in UI components and Hub packages already use this review gate;
Git, folder, and zip sources should stage their contents and reuse the same
gate.

## Warnings and violations

Most of what the review reports is a **warning**: an effect the package has, for
you to accept or decline. A public webhook, a credential, an outbound URL, and a
destructive seed statement are all warnings.

A **violation** is different. It is not a stronger warning; it means the package
will not be installed at all, whoever approves it, and the risk level reads
`blocked`. There are two:

- a pipeline whose bytes the review could not read, because bytes the review
  never saw are bytes the install would still write
- a file whose type a project's repository does not accept

The second is a file-extension check, not a guess about content. A package may
write source, documentation, data, and images. It may not write a `.sh`, a
`.dylib`, a `.env`, or any binary type this build has no reader for. The
refusal names the path and the extension.

The accepted set is the platform's, so every project has it without configuring
anything. A project may narrow it in `repo/zebflow.yaml` under
`spec.layout.allowed_extensions`; it cannot widen it. `zebflow.yaml`,
`zeb.lock`, `zebflow.init.json`, `schema.json`, and `.gitkeep` are accepted by
name whatever the list says, because an install cannot proceed without them.

A node bundle is judged by its own contract rather than by a project's list: it
materializes into `data/nodes/` and never writes into `repo/`.

## Database initialization

A package may carry SQL that runs at install, under `initial-data/`, `init/`,
or `seeds/`, in a `sekejap/` or `sqlite/` subfolder. Those are the only two
engines: there is no postgres or mysql seed execution, and both stores are
project-local ones the install creates, so package SQL cannot reach a database
you already had.

The review reports one `database_initialization` row per such file:

- `engine` and `source`: which engine replays the file, and where it came from
- `store` and `existing_data_at_risk`: which store receives it
- `statements`: a count per statement kind, with anything unrecognised counted
  under `OTHER` so nothing is dropped silently
- `tables`: every table the statements name
- `destructive`: the DROP, DELETE, TRUNCATE, and ALTER statements, quoted back

Every install review in the UI renders these rows in full: the Hub page's
project-bundle review, the project Hub page's node-bundle review, and Add+ in
the pipeline registry.

## Installing part of a project bundle

A platform Hub project install accepts three flags, all true when omitted:

- `include_code`: write source entries -- pipelines, pages, docs, everything
  that is not schema or seed SQL
- `include_schema`: write the schema and seed `.sql` files into `repo/`
- `execute_schema`: replay that SQL into the project's own stores

`execute_schema: false` with `include_schema: true` installs the schema as
files and leaves the stores untouched, so the SQL can be reviewed and run by
hand. `execute_schema: true` with `include_schema: false` is rejected rather
than downgraded, and so is a request that includes nothing at all. The install
response names every entry the scope skipped and every seed script that was
written but not run.

`zebflow.yaml`, `zeb.lock`, and `zebflow.init.json` are always written: a
project without its configuration is not a smaller install, it is a broken one.

In the UI these three flags are checkboxes in the Hub page's install review,
each on by default. Turning the schema off turns execution off with it and locks
that checkbox, so the form cannot assemble the combination the API refuses.

## Hub is not backup

Hub is for distribution. Backup is for preservation.

Hub packages are versioned, reusable project material that can move through Zebflow Hub,
GitHub, GitLab, local folders, or zip files. They should exclude private
runtime state, credentials, logs, caches, `.git/`, and full production data.

Backups may include project data, files, database snapshots, selected logs,
runtime state, and credential references or encrypted secrets when explicitly
allowed. Restore belongs to the backup system, not Hub.

## Common flow

1. enable the platform Hub service
2. create a publisher and scoped token
3. publish a package to `/api/hub/remote/assets`
4. register a Hub source at platform or project level
5. grant platform-owned source access to projects when sharing is intended
6. browse packages in Hub or Add+
7. review safety effects
8. add the package into the local project workspace

## Access model

Hub access is explicit:

- Hub service: where the Hub lives
- Hub access source: URL, read token, title, and capability-bearing access
- Hub grant: assignment of a platform-owned source to `all_projects` or one
  selected project

Registering a Hub service does not grant project access. Creating a platform
Hub source also does not grant project access. Projects see platform-owned
sources only through an explicit grant, or through sources they add inside the
project itself.

This supports internal software-house Hubs: a `reusables-producer` project can
publish shared assets, while platform admins grant read access to all client
projects without asking every project user to configure the source manually.

## Package kinds

Current canonical package kinds are:

- `pipeline_bundle`
- `template_bundle`
- `folder_bundle`
- `project_bundle`
- `node_bundle`

Unknown package kinds are rejected. This keeps Hub packages predictable and
prevents arbitrary package categories from appearing in consumers.

## Add and Install

Add+ should feel closer to Unity or Blender asset import than npm package
installation. Hub packages should follow the same safety model when they are
added through Add+.

- Add: canonical Hub action that brings a package into the current project.
- Add to current project: write the package into its normal destination.
- Clone as folder: Add mode that writes a larger package, folder, starter, or
  workflow bundle as a separate folder.
- Install: reserve for app-level runnable packages or full project/app
  templates.

For normal Hub packages, use Add. Imported pipelines should land as draft unless
the user explicitly activates them.

Add is copy/clone based. Hub does not keep ownership of the resulting project
files and does not track an installation state after Add. Users can change the
added source like any other project code.

### Where an added package lands

A package's file paths are relative to the publishing project's layout, and the
receiving project may keep its own files elsewhere. A release records the layout
its paths were produced by, and Add translates each file into the equivalent
directory of the receiving project:

- source (pipelines, pages, styles, shared components) goes to the target folder
  inside this project's source root, defaulting to `hub/{package_id}`
- assets go to this project's asset directory, under the same folder name, so
  they are served at `/assets/{owner}/{project}/...`
- docs go to this project's docs directory, under the same folder name

Anything else the package carries — its own `zebflow.yaml`, exported schema
documents, node interfaces — stays inside the package's own folder. One project
holds one of each of those, so a second copy is kept readable rather than
written over the project's own.

The safety review reports these destinations, and Add writes exactly the
destinations the review reported.

## Safety review

Before Add, Zebflow should show what the package can affect:

- files added or overwritten
- pipelines registered
- nodes used
- credentials required
- external URLs and domains
- database schemas, tables, collections, graphs, or indexes touched
- filesystem paths touched
- schedules, webhooks, and public endpoints created
- secrets required but not included
- large files and optional seed/demo data

The package brings source into the project. It does not bring trust.

## Gallery metadata

Hub cards show package identity and presentation metadata:

- title
- package id
- latest version
- package kind
- tags
- publisher name and publisher URL
- optional cover image

Cover images are Hub media, not arbitrary files from the install payload.
Installable files live in `artifact.files`; gallery media lives in the package
media contract and is served through:

```text
/api/hub/remote/assets/{package_id}/media/{media_name}
```

The target gallery contract also allows optional screenshots and YouTube demo
links, but those are structured metadata, not arbitrary external embeds.

## Release immutability

A published `package@version` is fixed. Correcting anything — content, title,
description — means publishing a new version.

A second publish of a version that already exists is refused with
`HUB_VERSION_EXISTS` (HTTP 409) on both the project publish route and
`/api/hub/remote/assets`. The refusal happens before anything is written, so
the stored artifact, its digest, and its creation time are left untouched.

This is what makes `zeb.lock` meaningful: a lock entry pins a version to a
digest, and a digest only means something when the bytes it names cannot change
underneath it. If a release could be overwritten, every project that pinned it
would report a tampered dependency the next time it checked.

## Presentation

A release carries what installing it requires. Everything a human reads while
choosing — summary, long-form markdown, cover, gallery — lives on the mutable
package row beside it, and is edited without publishing anything:

```text
PATCH /api/projects/{owner}/{project}/hub/assets/{package_id}/presentation
{ "summary": "...", "description_md": "...", "image_file_path": "...", "gallery": { ... } }
```

Authorised by the same publisher token as publish. Every field is optional and
an omitted field is left alone. No release, digest, or creation time moves.

Publishing follows the same rule: a publish that carries no summary, no
description, and no `image_file_path` leaves the existing presentation exactly
as it was.

## Retraction

`DELETE /api/hub/remote/assets/{package_id}` retracts. It does not delete.

The artifact bytes of every release are destroyed. The package and version rows
survive, marked retracted with a timestamp and the publisher's reason, and
`package@version` can never be published again — a second publish of a retracted
coordinate is refused with `HUB_VERSION_RETRACTED`.

Freeing the coordinates would have been the hole in immutability: a lockfile
pinning the old digest would report a tampered dependency for content that was
simply republished. Keeping them means a coordinate always names one thing, or
nothing.

A retracted release stays listed and explorable. Its detail response still
describes it, with `artifact: null` and the retracted marker saying why; the
artifact and install routes answer `410 Gone`.

## Token scopes

A hub token holds `hub:read`, `hub:publish`, `hub:manage`, or a combination.
Any other value is refused at creation with `HUB_TOKEN_SCOPE_INVALID` (HTTP
400), naming the value and the accepted set, and a token with no usable scope is
refused too. A scope the publisher record does not grant is refused with
`HUB_PUBLISHER_SCOPE_DENIED`.

## Storage

Hub operational state lives under the platform data root:

```text
services/hub-default/hub.db
services/hub-default/packages/{package_id}/versions/{version}/artifact.json
```

The package metadata is in `hub.db`; the installable package payload is in
`artifact.json`.
