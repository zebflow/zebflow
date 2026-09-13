# Hub

Hub is the sharing surface for reusable Zebflow project material.

It is not only a plugin registry.
It is also not a backup system.
It is also not the entire Add+ system.

The current main focus is:

- packages
- reusable project material
- frontend library discovery
- future shared nodes and platform-level capabilities

## Important distinction

Add+ is the unified "bring something into this project" surface. Hub is one
source behind Add+.

Add+ sources should be treated explicitly:

- Built-in Catalog: offline first-party material embedded in Zebflow, such as
  shadcn-compatible Zeb React UI components.
- Hub: packages from `hub.zebflow.com`, a private platform Hub, or a project
  Hub source.
- Git: packages or source folders fetched from GitHub, GitLab, or another git
  remote.
- Local: files selected from the current machine, including `.zip` package
  archives and local folders.

Zip belongs to Local when the user uploads/selects the archive from their
machine. A zip fetched from a URL belongs to the remote source that provided it.

Adding something usually means:

- copy into project
- clone into workspace
- edit it as project source

not:

- activate a hidden runtime install

The two package kinds that really are runtime dependencies —
`node_bundle` (materializes under `data/hub/nodes/`) and `rwe_library`
(materializes under `data/hub/rwe-libraries/`) — are recorded as such in
`zeb.lock` (and `zebflow.yaml` for `rwe_library`) rather than left as
untracked copies.

This keeps Hub aligned with how Zebflow projects actually work.

Use these words consistently:

- Package: canonical Hub item. A package is versioned, typed, and reviewable.
- Package kind: the package category, such as `pipeline_bundle` or
  `project_bundle`.
- Browse: find addable material from Add+ sources. Inside Project Hub, Browse
  means packages visible from configured Hub sources.
- Published: packages this project or publisher context has published.
- Publish: create a new package version.

- Add: canonical Hub action that brings a package into the current project
- Add to current project: write the package into its normal destination
- Clone as folder: Add mode that writes a larger package, folder, starter, or
  workflow bundle as a separate folder
- Install: reserve for runnable apps or full project/app templates

Hub packages should arrive as reviewable project source. Adding a package is a
copy/clone operation, not a managed installation. After Add, users may freely
edit, delete, or restructure the files in the project. Pipelines, schedules,
public endpoints, credentials, schema changes, initial data, and outbound URLs
must be visible before the project accepts or activates them.

## Add+ policy review

Every Add+ source must pass through the same review shape before it writes into
the project:

- source adapter and package/material kind
- files added, skipped, or overwritten
- pipelines and nodes included
- credentials required
- external URLs
- database and filesystem effects
- schedules and public endpoints
- large files and seed/demo data
- warnings and risk level

Current implementation covers Hub packages and the built-in UI component
catalog. Git, local folder, and zip sources should stage their files first and
then use the same review shape before accepting the import.
