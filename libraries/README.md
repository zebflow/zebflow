# Zeb Libraries

This folder contains platform-managed web libraries that are curated by Zebflow.

Current rule:

1. root-level discovery for maintainers and GitHub readers
2. project installs pin concrete versions in project state
3. these root packages are the source catalog, not the project's durable state

## Runtime Installation Model

Library installation is project-triggered and uses the office's shared library
root. `{data_root}` is already the mounted persistence root; there is no nested
`mounted/` directory in the storage contract.

1. Resolve dependency list + pinned versions from Zeb library spec.
2. Download package archives into the shared library root:
   - `{data_root}/libraries/downloads/external/js-registry/...`
3. Extract package bodies into the shared library root:
   - `{data_root}/libraries/installed/external/js-registry/{package}/{version}/package`
4. Build declaration/export indexes for autocomplete:
   - `{data_root}/libraries/indexes/external/js-registry/{package}/{version}/exports.json`
5. Link dependencies into each project workspace:
   - `{project}/repo/node_modules/...` (symlinked to shared library root)
6. Persist project lock state:
   - `{project}/repo/libraries.lock.json`
7. Bundle/minify project assets into:
   - `{project}/data/runtime/web-assets/rwe/chunks/...`

Template save triggers project-level library detection + asset preparation for used libraries.
