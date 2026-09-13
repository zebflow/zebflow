# UI Theme

Status: **sealed** — 2026-09-11. Names are shadcn/ui's, verbatim. Values are Zebflow's.

The colour vocabulary every platform template speaks. A component names a
*role* (`bg-card`, `text-muted-foreground`); the theme decides what colour
that role is. Light and dark are the same names with different values.

## Identity

| | |
| --- | --- |
| Definition | `src/platform/web/templates/styles/main.css` — `:root` (light) and `.dark` |
| Utilities | `src/rwe/processors/tailwind/compiler.rs` `is_semantic_color_token` — `bg-<token>`, `text-<token>`, `border-<token>`, `ring-<token>`, … with `/<alpha>`; each compiles to `var(--<token>)` |
| Switch | `.dark` on the theme root (`ProjectStudioShell`); no attribute, no media query |
| Guard | `tests/rwe/theme_tokens.rs` — a template may name only these tokens; the primitives in `components/ui/` may name no raw palette colour. `tests/rwe/zeb_ui.rs` — the same for `zeb/ui`, plus every class compiles and every component is on `/dev/design-system/ui` |
| User projects | a new project's `globals.css` carries both blocks (`src/platform/theme/mod.rs` `THEME_CSS`); `zeb/ui` components render on them — `blessed/source-libraries/ui/README.md` |

## Tokens

Pairs: a surface token has a `-foreground` for what sits on it.

| Token | Role |
| --- | --- |
| `background` / `foreground` | the page |
| `card` / `card-foreground` | a raised panel |
| `popover` / `popover-foreground` | menus, dialogs, inputs' own background |
| `primary` / `primary-foreground` | the one action colour (brand orange) |
| `secondary` / `secondary-foreground` | a quieter button |
| `muted` / `muted-foreground` | subdued surface; secondary text |
| `accent` / `accent-foreground` | hover / selected row background |
| `destructive` / `destructive-foreground` | delete, error |
| `success` / `success-foreground` | live, passed, saved |
| `warning` / `warning-foreground` | caution, pending |
| `info` / `info-foreground` | neutral notice, links (brand blue) |
| `border` | every hairline |
| `input` | a control's border |
| `ring` | focus outline |
| `chart-1` … `chart-5` | series colours, in order |
| `sidebar`, `sidebar-foreground`, `sidebar-primary`, `sidebar-primary-foreground`, `sidebar-accent`, `sidebar-accent-foreground`, `sidebar-border`, `sidebar-ring` | the navigation rail, themed apart from content |
| `radius` | base corner radius; `rounded-*` derive from it |

`success`, `warning`, `info` are additions, made the way shadcn's theming doc
says to add a colour: a pair, defined in both blocks, exposed as a utility.

## Shape

One name per token: `--<token>`, shadcn's. A utility compiles straight to it —
`bg-card` → `background-color: var(--card)`. There is no `--color-<token>`
alias: an alias set on `:root` computes once against the light values and is
inherited as that colour into `.dark`.

```css
:root  { --background: #e9edf3; --foreground: #1b1f24; … --radius: 0.5rem; }
.dark  { --background: #14171b; --foreground: #e9edf3; … }
```

A shadcn theme file pastes into these two blocks unchanged.

## Rules

1. A template names roles, never colours. `bg-red-500` in `components/ui/` fails the guard; elsewhere it is a defect to be swept.
2. Every token is defined in both blocks. A token defined once is a light-only bug waiting for the toggle.
3. The palette primitives (`--color-zeb-*`, `--color-brand-*`) are values for the two blocks to point at. They are not utilities and no template names them.
4. Alpha is on the utility (`bg-accent/40`), never a second token.
5. Adding a role = one row here, one pair in each block, one arm in `is_semantic_color_token`, one rendering in `/dev/design-system`.

## Retired

`body`, `body-soft`, `body-muted`, `surface`, `surface-2`, `surface-3`, `bg`, `border-soft`, `accent-*` (as orange), `ui-*`, `dark-*`, `brand-*` as utilities. The guard refuses them.
