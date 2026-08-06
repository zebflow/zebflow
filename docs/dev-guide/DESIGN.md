# Design Rules

## Purpose

This file defines the strict platform UI design rules for Zebflow development.

If a UI implementation conflicts with this file, this file wins.

## Core Rule

There must be **one true source** for design tokens.

For platform UI:

- `main.css` stores global brand tokens and global variables.
- Components consume proper shared utilities and component APIs.
- Do not redefine utility classes manually in `main.css`.

## Canonical Theme

Zebflow uses the Claude design-system overhaul direction as the platform
baseline. The visual language is a Melbourne gray-blue operating system with a
signal-orange action color.

Canonical public/light tokens:

- background: `#E9EDF3`
- soft background: `#F4F6F9`
- ink: `#1B1F24`
- soft ink: `#525D68`
- muted ink: `#8E9CAD`
- border: `#DCE3EC`
- strong border: `#CDD6E1`
- grid line: `#DDE3EB`
- brand blue: `#1E66D6`
- brand orange: `#EA5A0C`

Canonical Studio/dark tokens:

- dark base: `#14171B`
- dark chrome: `#16191E`
- dark panel: `#1B2026`
- dark border: `#232932`
- strong dark border: `#2A313C`
- dark text: `#E9EDF3`
- dark soft text: `#8E9CAD`
- dark muted text: `#6B7785`
- action accent: `#EA5A0C`

All of these must be expressed as CSS variables in `main.css` and consumed by
components through Zeb Tailwind utilities or shared UI APIs. Do not paste these
hex values into new components unless the utility/token pipeline cannot express
the value yet; if that happens, fix the token pipeline.

## Product Surface Split

Zebflow has two deliberate UI moods:

- Public/home/hub/login/content pages use the light gray-blue system.
- Project Studio, graph editing, logs, DB tools, and operational workspaces use
  the dark cockpit system.

Do not mix the two styles inside one surface. A public page should not become a
dark cockpit unless it is showing an embedded tool. A Studio panel should not
use public-page card styling unless it is a preview of public content.

## Typography Scale

Canonical fonts:

- display/UI: `Space Grotesk`
- readable body/supporting copy: `Hanken Grotesk`
- code, IDs, versions, paths, coordinates: `JetBrains Mono`

Default rules:

- app chrome and controls use `font-sans`
- code/path/version/meta labels use `font-mono`
- headings use `font-display`
- ordinary button/input text is 14-15.5px
- compact meta text is 10-12px mono
- page headings are 24-40px depending on surface density
- do not use oversized hero type inside Studio panels

Weights:

- 400 for body and secondary UI
- 500 for controls and medium emphasis
- 600 for section headings and primary actions
- 700 only for brand wordmark or major page headings

Letter spacing:

- normal by default
- tight tracking only for display/brand headings
- uppercase mono labels may use tracking around `0.12em`

## Spacing And Radius

Use an 8px spacing rhythm.

Common spacing:

- dense row padding: `px-3 py-2`
- normal field padding: `px-4 py-3` or `px-4 py-3.5`
- compact toolbar gap: `gap-2`
- form vertical gap: `mt-3`
- primary action gap: `mt-4`
- section/card internal padding: `p-5` to `p-6`

Common radius:

- `6px` compact buttons and small controls
- `8px` operational buttons/cards
- `10px` login and public form controls
- `12px` public cards and image containers
- `14px` larger operational panels
- `9999px` only for true pills, badges, and circular/icon actions

Cards and panels should generally use borders, not heavy shadows. Shadows are
reserved for overlays, focus moments, or deliberate preview depth.

## What Belongs In `main.css`

Only global source-of-truth values and truly global styling concerns:

- brand colors
- font tokens
- radius tokens
- spacing tokens if they are globally standardized
- shared root variables
- global element defaults
- global selection styles
- global backdrop defaults
- keyframes

Examples:

- `--font-display`
- `--font-sans`
- `--font-mono`
- `--color-brand-orange`
- `--radius-panel`

## What Must Not Be Put In `main.css`

Do not put manual utility-class implementations there.

Forbidden examples:

- `.font-display { ... }`
- `.text-body { ... }`
- `.px-4 { ... }`
- any Tailwind-like utility redefinition
- component-specific fixes
- one-off dialog patches
- page-specific hacks

`main.css` is not a place to impersonate the utility system.

## Component Rule

Components should use the proper shared class/API shape directly.

Examples:

- `className="font-display"`
- `className="text-body"`
- shared dialog/header/footer components

If a utility like `font-display` is wrong, fix the actual utility/token pipeline.
Do not patch around it in component CSS or `main.css`.

## Dialog Rule

Dialogs must use the shared dialog component family.

Use:

- `Dialog`
- `DialogContent`
- `DialogHeader`
- `DialogTitle`
- `DialogFooter`

Use standard internal spacing consistently.

Default baseline:

- outer structure through shared dialog components
- body padding should be standardized, e.g. `px-4 py-4` unless there is a stronger shared pattern

## Zebflow UI Rule

For Zebflow platform UI:

- use Zeb React
- use Zeb Tailwind
- use shared UI components
- do not use DOM hacks
- do not use `window.*` or `document.*` when Zeb React can own the behavior

If Zeb/RWE behaves strangely:

- stop
- report the anomaly as foundational
- fix the foundation
- do not patch around it locally

## Typography Rule

Typography tokens must be driven from the global source of truth.

That means:

- define font tokens in `main.css`
- ensure the actual utility system resolves from those tokens
- keep component usage simple

Correct:

- define `--font-display` in `main.css`
- use `className="font-display"` in components

Incorrect:

- `main.css` manually defining `.font-display`
- component-level arbitrary fallback just to bypass the broken token path

## Review Standard

When editing UI, always ask:

1. Is this token defined in the right global place?
2. Is this utility coming from the real system?
3. Am I patching the source of truth, or faking it locally?

If the answer is “faking it locally,” do not proceed with that approach.
