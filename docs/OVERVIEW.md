# Zebflow Overview

Zebflow is built from four layers:

1. `framework`
2. `language`
3. `rwe`
4. `platform`

## System Shape

1. `framework`
   - Rust-first pipeline orchestration
   - pin-based graph execution
   - observable execution traces

2. `language`
   - sandboxed scripting runtime
   - used where a node or page needs executable logic

3. `rwe`
   - Reactive Web Engine
   - compiles and renders TSX templates
   - auto-wraps page documents from the page contract
   - supports selective client hydration
   - stays generic about local modules/assets

4. `platform`
   - Zebflow’s own web application
   - login, home, project shell
   - built using the same RWE contract
   - owns product policy such as Zeb Libraries
   - owns project-level authorization policy for REST, MCP, and assistant access

## Canonical Web Delivery Policy

For Zebflow web delivery, the canonical direction is:

1. SSR first
2. `history` navigation by default
3. selective component hydration
4. `document` navigation as a fallback mode

This is an SSR-first app model with SPA navigation capability.

## Canonical Template Contract

There is one page-root semantic and one ordinary component semantic:

1. page
   - `export const page = { ... }`
   - `export default function Page(...) { return <Page>...</Page>; }`
   - the only kind that should be bound directly to routes

2. component
   - normal imported TSX module
   - never a direct render root

There is no special layout kind in the compiler contract. Layouts are just
components.

## Theme Source Direction

Theme and base CSS are owned by the template tree under `template_root`.

Deterministic default entries:

1. `styles/main.css`

This keeps theme definition:

1. local to the project or platform template surface
2. compile-time visible to RWE
3. easy to extend from managers such as Zebflow platform

## Folder Direction

At project level, the intended git-synced app surface is:

1. `pipelines/`
2. `assets/`
3. `templates/`

At runtime, a manager such as Zebflow platform supplies the per-project
`template_root` to RWE for each compile call.

## Pipeline Runtime Direction

Pipelines should not use duplicated source trees for draft and production.

There is one canonical source surface:

1. `app/pipelines/`

From that one source surface, Zebflow derives two runtime states:

1. draft
   - current working-tree file content
   - used for save-time validation, preview, and editor feedback
2. production
   - activated runtime snapshot
   - used by webhook and schedule execution

The distinction is tracked by revision state, not by cloning the whole project.

Current design direction:

1. `hash`
   - current working-tree content hash
2. `active_hash`
   - activated production hash
3. if `hash != active_hash`
   - the pipeline is draft-diverged from production

To keep runtime stable while the working tree changes, production uses an
activated snapshot, not the mutable working-tree file directly.

## Pipeline Hot Reload Direction

Pipeline hot reload should follow one shared lifecycle:

1. source file changes
   - update draft hash
   - draft compile/validation only
   - production runtime unchanged
2. activate/publish
   - materialize one activated pipeline snapshot
   - update `active_hash`
   - rebuild compiled runtime registry entry
   - refresh derived trigger projections such as schedules
3. deactivate
   - remove compiled runtime registry entry
   - remove derived trigger projections

The runtime registry is the execution truth for production.
The working tree is the authoring truth for draft.

This separation is intentional so the platform can support:

1. safe on-the-fly editing
2. production stability
3. hot reload by atomic registry replacement
4. future schedule re-registration without restart

## Why Centralized Docs Exist

The goal of `docs/` is to keep the active contract in one place so the
framework, platform, and editor layers do not drift.

## Project Authorization Direction

Project access should be resolved through one shared capability model:

1. capability
   - smallest permission unit
2. policy
   - named bundle of capabilities
3. binding
   - assignment of one policy to one subject in one project

This is intentionally platform-level policy, not RWE policy.

It is the foundation for:

1. current REST access control
2. future MCP session scopes
3. future internal assistant profiles
4. future contributor RBAC
