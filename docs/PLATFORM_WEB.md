# Zebflow Platform Web

This document describes how Zebflow platform uses RWE today.

## Core Rule

Zebflow platform uses the same RWE contract that user project templates will
use.

That is why the platform now depends on:

1. compile-scoped `template_root`
2. compile-scoped project styles under `templates/styles/`
3. explicit TSX imports
4. `export const page = { ... }`
5. intrinsic `<Page>...</Page>` roots

The platform should not rely on a hidden or product-only template mechanism.

## Zeb Libraries Boundary

Zeb Libraries are managed by platform, not by RWE directly.

Platform responsibilities:

1. official library catalog
2. version pinning
3. install/update/remove flows
4. project vendoring under `app/libraries/`
5. project lock file management (`app/libraries.lock.json`)
6. root catalog source under `libraries/`

RWE responsibilities:

1. compile templates
2. resolve local modules/assets through generic hooks
3. inject trusted local runtime/script/style bundles

This keeps library policy in the product layer and rendering policy in the
engine layer.

The current repository-level source catalog should live under:

`libraries/`

That root is for maintainers and discovery. Project-owned installed state still
belongs under:

`app/libraries/`

## Manager Direction

Platform should expose Zeb Libraries through a dedicated management surface
under `Settings`.

The intended split is:

1. `Node Manager`
   - pipeline/runtime extension packages
   - separate trust and execution concerns
2. `Web Library Manager`
   - template/runtime libraries for web rendering
   - owns `zeb/*` installation and vendoring

For Zeb Libraries specifically, the platform-side flow should be:

1. install library
2. generate project-owned metadata and vendored runtime files
3. notify the editor/autocomplete surface
4. let save-time compile reuse the prepared metadata

The editor should consume metadata such as `keywords.json` and `exports.json`,
not raw upstream package source on every keystroke.

## Platform Template Root

Current platform template root:

`crates/zebflow/src/platform/web/templates`

Platform pages compile from real files under that root. This forces the
platform UI to obey the same import and boundary rules as user project
templates.

## Platform Template Structure

1. `templates/pages/`
   - route entry templates

2. `templates/components/`
   - shared UI and shell components

3. `templates/components/ui/`
   - reusable UI primitives such as `Button`

4. `templates/components/layout/`
   - shared admin shell components such as `AdminWrapper`

5. `templates/styles/`
   - compile-scoped theme and base CSS
   - default entries:
     - `styles/main.css`

Only files under `templates/pages/` should be selected as render roots by the
platform route layer.

## Current Login Page Policy

The login page stays server-first and operationally focused:

1. no client-side explainer toggle
2. no bootstrap-debug UI in the main login interaction
3. primary submit action uses the shared `Button` component

## Current Route Shell Policy

Platform route structure is:

1. `/login`
2. `/home`
3. `/projects/{owner}/{project}/...`

The project area is modeled as an SSR-first shell that can later support richer
SPA navigation inside the project surface.

## Current Project Menu Policy

The project shell is now organized around:

1. `Pipelines`
2. `Build`
   - `Templates`
   - `Assets`
   - `Schema`
3. `Dashboard`
4. `Credentials`
5. `Tables`
6. `Files`
7. `Todo`
8. `Settings`

`Build` is the authoring area. It replaces the old overloaded design slot and
keeps template work, asset management, and structured design artifacts under
one project-facing workspace.

## Current Admin Shell Policy

Project-facing admin pages now share a common wrapper:

1. persistent sidebar navigation
2. sticky page header
3. slot-based page content
4. bottom-right assistant launcher

This shell lives in `templates/components/layout/AdminWrapper` and is consumed
by project pages instead of duplicating layout markup per route.

The assistant launcher is enabled by default in current project pages. If a
specific admin view needs to suppress it, the wrapper can be given
`chatClass="hidden"`.

## Template Selection Rule

When the platform or a future WUI selects a template for rendering:

1. only page templates should be selectable
2. components are not direct render roots

If a component needs preview behavior, that should be a dedicated preview flow,
not the production route-binding contract.
