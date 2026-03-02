# Zeb Libraries

This folder contains platform-managed web libraries that are curated by Zebflow.

Current rule:

1. root-level discovery for maintainers and GitHub readers
2. project installs should still vendor concrete versions into `app/libraries/`
3. these root packages are the source catalog, not the project's durable state

## Runtime Installation Model

Library installation is project-triggered and uses a shared mounted npm store:

1. Resolve dependency list + pinned versions from Zeb library spec.
2. Download/extract npm packages into global mounted store:
   - `{data_root}/mounted/npm-store/packages/...`
3. Build declaration/export indexes for autocomplete:
   - `{data_root}/mounted/npm-store/indexes/...exports.json`
4. Link dependencies into each project workspace:
   - `{project}/app/node_modules/...` (symlinked to mounted store)
5. Persist project lock state:
   - `{project}/app/libraries.lock.json`
6. Bundle/minify project assets into:
   - `{project}/data/runtime/web-assets/rwe/chunks/...`

Template save triggers project-level library detection + asset preparation for used libraries.
