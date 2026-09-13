# Hub Packages

Hub distributes inspectable source packages, not backups.

Package kinds (`src/platform/services/hub.rs`):

- `pipeline_bundle`, `template_bundle`, `folder_bundle`, `project_bundle` — cloned into the repo as ordinary source, not tracked afterwards
- `node_bundle`, `rwe_library` — installed and recorded in `zeb.lock` (materialized under `data/hub/`)
- `skill` — one `skills/<name>/` folder, cloned to `skills/<name>/` in the receiver's source root; blessed skills come from `blessed/skills/` in the binary (`src/platform/skills/`)

Review before add should show:

- package kind
- publisher
- version
- files to create or replace
- external URLs
- credential requirements
- initialization steps
- database schema or seed steps
- executable scripts
- policy warnings

Naming:

- Use `Hub` for the product surface.
- Use `package` for published shareable items.
- Use `node bundle` for installable node packages.

Do not add silent execution during package review. Any initialization step must be visible before the user confirms.
