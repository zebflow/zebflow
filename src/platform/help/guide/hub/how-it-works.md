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
schedules, large files, seed data, warnings, and risk level. Built-in UI
components and Hub packages already use this review gate; Git, folder, and zip
sources should stage their contents and reuse the same gate.

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

## Storage

Hub operational state lives under the platform data root:

```text
services/hub-default/hub.db
services/hub-default/packages/{package_id}/versions/{version}/artifact.json
```

The package metadata is in `hub.db`; the installable package payload is in
`artifact.json`.
