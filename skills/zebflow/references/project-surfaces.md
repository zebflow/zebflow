# Project Surfaces

Important source and runtime areas:

- `src/` contains the Zebflow application, platform, pipeline, node, UI, and runtime code.
- `libraries/` contains reusable Zeb/RWE library material.
- `npm/` and `pip/` contain package distribution surfaces.
- `docs/` contains documentation intended to be committed.
- `skills/` contains repository-local operating guidance.
- `.ignored/` contains local scratch, archived material, generated previews, and non-committed working notes.
- `data/` is runtime data and should be handled carefully.

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
