# Zeb Libraries

Zeb Libraries are reviewed browser libraries shipped with Zebflow under the
`zeb/*` namespace. They provide complex capabilities that should not be
rewritten for every project, such as code editing, maps, charts, rich text, and
3D rendering.

## Ownership

The platform owns:

1. The library catalog and embedded files.
2. Project enable and disable controls.
3. Version and integrity resolution in `repo/zeb.lock`.
4. Local asset serving and policy checks.

RWE owns:

1. Compiling project TSX and TypeScript.
2. Resolving enabled `zeb/*` imports to local assets.
3. Rendering and browser hydration.
4. Enforcing import, script, and network boundaries.

RWE does not run a package manager.

## Durable State

`repo/zebflow.yaml` records the requested libraries. `repo/zeb.lock` records the
resolved version, entry point, source, and digest. The lock file is the frozen
`DependencyLock` contract defined in:

- `src/contracts/kinds/dependency_lock.rs`
- `docs/contracts/kinds/dependency-lock/README.md`

Library files are embedded in the Zebflow executable. They are not copied into
project `node_modules`, and template saves do not download or rebuild them.

## User Paths

Projects have three preferred ways to add browser behavior:

1. Create a focused local TypeScript module.
2. Add a reviewed script or template package from Hub.
3. Enable a bundled Zeb Library.

These paths keep project source readable, reviewable, and reproducible.

## Library Shape

Each bundled library may contain:

```text
libraries/zeb/<name>/
  manifest.json
  README.md
  <version>/
    library.json
    exports.json
    keywords.json
    runtime/
    wrappers/
```

Runtime bundles must work without a CDN or package-manager process. Wrappers
should expose a small Zeb React interface while leaving a lower-level export
available when advanced control is needed.

## Future npm Ingestion

Direct npm ingestion is intentionally deferred. A stable implementation must
resolve and lock the complete dependency graph, reject lifecycle scripts,
bound archive extraction, compile to an approved browser artifact, record
immutable digests, and show capability findings before installation.

Until those rules are implemented and frozen, no project-facing npm installer
or automatic template-save ingestion should exist.
