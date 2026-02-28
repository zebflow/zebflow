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

## Why Centralized Docs Exist

The goal of `docs/` is to keep the active contract in one place so the
framework, platform, and editor layers do not drift.
