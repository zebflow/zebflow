---
name: zebflow
description: Work on Zebflow projects, runtime code, pages, pipelines, Hub packages, storage, maps, database surfaces, and project operations with repository-native conventions.
---

# Zebflow

Use this skill when working inside a Zebflow repository or project workspace.

## Stable Knowledge

- Read `src/platform/help/` for how a project is built (the help tree is the canonical "how to use Zebflow"; `docs/usage/` keeps only install, deploy and node authoring).
- Read `docs/developer/README.md` for Zebflow platform work.
- Read `docs/contracts/README.md` for stable rules shared by both.
- Read `docs/reference/README.md` for exact names and formats.

## First Read

Open only the reference that matches the task:

- `references/orientation.md` for the product model and vocabulary.
- `references/project-surfaces.md` for project folders, runtime data, files, pages, and pipelines.
- `references/editing-rules.md` for code and documentation changes.
- `references/runtime-workflows.md` for running, testing, reinstalling, and checking local instances.
- `references/mcp-workflows.md` for project-scoped MCP work.
- `references/pipeline-authoring.md` for creating, editing, activating, and debugging pipelines.
- `references/web-authoring.md` for TSX pages, RWE, Zeb React, and Zeb Tailwind.
- `references/data-files-maps.md` for Sekejap, SQL connections, files, FileRef, table, geo, and mapserver work.
- `references/hub-packages.md` for Hub package review, publishing, and adding.
- `references/node-authoring.md` for native, composite, and WASM node work.
- `references/platform-internals.md` for changing platform services, routes, runtime storage, policy, auth, git, and settings.
- `references/distribution.md` for npm, pip, Docker, the embedded `blessed/` libraries, and deployable artifacts.
- `references/quality-checks.md` for verification before reporting completion.

## Working Rule

Keep changes scoped to the requested surface. Prefer the existing Zebflow patterns over new abstractions. When a behavior is shared by several pages, nodes, or services, fix the shared implementation instead of patching one screen.

## UI Rule

Use Zeb React and Zeb Tailwind for product UI. Do not add `data-*` behavior hooks when a Zeb/RWE event or component pattern can express the behavior.

## Data Rule

Do not pass large data through normal JSON payloads when a file, reference, table operation, or storage-backed path is the correct shape.
