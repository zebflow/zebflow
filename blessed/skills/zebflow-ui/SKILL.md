---
name: zebflow-ui
description: Building a screen in a Zebflow project with the zeb/ui component set — buttons, forms, dialogs, tables, tabs, menus, toasts, the editor. Use when composing any UI, choosing between zeb/ui and a clone, applying theme roles, or laying out an admin shell, a list, a form or a detail page.
license: MIT
metadata:
  version: "1"
---

# Build a screen with zeb/ui

`zeb/ui` is shadcn/ui on the platform's engine: the same names, variants and
sub-parts, imported per component with nothing to install. Facts:
`help(topic="web/ui")`, `web/tailwind`; every component live at
`/dev/design-system/ui`; `list_ui_catalog` for what exists and what the
project has cloned.

## Choose the component before you write markup

1. `list_ui_catalog` once per session. If a component exists, use it; a
   hand-rolled dialog, dropdown or select is a bug you will own.
2. Import each one from its file: `import { Dialog, DialogTrigger, DialogContent } from "zeb/ui/dialog"`.
   Sub-parts compose through context like Radix — `<Tabs value onValueChange>`,
   `<Dialog open onOpenChange>`, `<Select value onValueChange>` — no wiring
   between parent and children.
3. There is no `asChild`: `<Button as="a" href="/x">`. There is no `cva`:
   variants are objects joined with `cx`.
4. Only clone (`install_ui_components names=["dialog"]` → `shared/ui/dialog.tsx`,
   import `@/shared/ui/dialog`) when you need to change the component's
   behaviour. A clone stops receiving fixes; it is yours.

## Colour and type are roles

Every colour is a token the project's `globals.css` defines for light and
dark: `bg-background text-foreground`, `bg-card`, `bg-primary text-primary-foreground`,
`bg-muted text-muted-foreground`, `border-border`, `border-input`, `ring-ring`,
`text-destructive`, `bg-success/10 text-success`, `text-warning`, `text-info`.
A palette class (`bg-blue-600`, `text-gray-500`, `text-white`) is a colour that
does not move when the theme does — do not add one. Fonts are variables
(`--font-sans`, `--font-display`) set in `globals.css`, not classes.

## The four layouts

Almost every screen is one of these; start from the shape, not from a blank page.

**Shell** — a sidebar of destinations and a header, content in the middle.
`aside` with `bg-sidebar text-sidebar-foreground border-sidebar-border`,
a `Link` per item (`bg-sidebar-accent` for the active one), `main` with
`mx-auto max-w-6xl p-6`. Mobile: the sidebar becomes a `Sheet`.

**List** — a title, one primary action (`Button`), filters as controls in a
row, a `Table` (or `Item` rows) with the important column first, `Pagination`
below, and an `Empty` state that carries the action when there are no rows.

**Form** — `Field` per input (`Label`, `Input`/`Textarea`/`Select`/`Checkbox`/
`Switch`, the error under it), grouped in a `Card`, one submit `Button` at the
end, `Toaster` for the result. The form is a real `<form method="post">` to
the POST pipeline (`zebflow-pipeline`); the page re-renders from the redirect.

**Detail** — a header with the title, status `Badge` and actions
(`DropdownMenu` for the rare ones), then `Tabs` or stacked `Card`s of facts;
destructive actions behind `AlertDialog`.

Rich text is `Editor` from `zeb/ui/editor` with `DocumentView` /
`renderDocumentHtml` from `zeb/ui/editor-render` — the JSON document is what
you store (`zebflow-files-editor`).

## Quality floor — check before calling a screen done

- Every interactive element is a `button` or an `a` (navigation), reachable
  by keyboard, with a visible focus ring (`focus-visible:ring-2 ring-ring`).
- Icon-only buttons have `aria-label`. Inputs have a `Label` (`htmlFor`).
- Hit targets ≥ 24 px; touch ≥ 44 px. Mobile input text ≥ 16 px.
- Empty, loading and error states exist and say what to do next.
- Destructive actions confirm (`AlertDialog`) or are undoable.
- Filters, tabs and pagination are reflected in the URL (`useSearchParams`,
  `useRouter().push`) so a refresh keeps them.
- Numbers in columns use `tabular-nums`; dates are formatted for the locale
  (`Tool.time.format`).
- No `transition-all`, no layout shift on hover, no `outline-none` without a
  replacement, no emoji as icons.
- Works at 375 px wide and in the dark theme (put `dark` on an ancestor to
  check) — both are one class away in the gallery.

Then the loop from `zebflow-rwe`: fetch, read the body for
`RWE component error`, open it, click what you built and check the state.

## What the gallery is for

`/dev/design-system/ui` shows every component with every variant, on the
same tokens a project uses. When a component looks wrong there, it is a
library bug — report it with the component and the state — not something to
work around in the page. When it looks right there and wrong in your page,
the difference is in your page: a missing `globals.css` import, a palette
class, or a token that is not a token.
