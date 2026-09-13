# Project Surfaces

Important source and runtime areas:

- `src/` contains the Zebflow application, platform, pipeline, node, UI, and runtime code.
- `blessed/` contains what the binary embeds: `rwe-libraries/` (zeb/* runtime bundles), `source-libraries/ui/` (zeb/ui), blessed node bundles.
- `src/platform/help/` is the help tree — the one "how to use Zebflow" text, served over MCP and in Studio.
- `npm/` and `pip/` contain package distribution surfaces.
- `docs/` contains documentation intended to be committed.
- `skills/` contains repository-local operating guidance.
- `.ignored/` contains local scratch, archived material, generated previews, and non-committed working notes.
- `.zebflow-platform-data*/` is a dev instance's data root (`dev.sh`); `data/` at the top level is untracked runtime data. Both hold an embedded database — never share one between two processes.

Project source usually includes:

- pages and templates
- pipelines
- scripts
- files
- Hub package metadata
- database setup material

Project data usually includes:

- uploaded files
- runtime files
- Sekejap stores
- SQLite stores
- invocation history
- generated artifacts

Rule:

Do not mix source packages with runtime data unless the feature explicitly exports a portable package.
