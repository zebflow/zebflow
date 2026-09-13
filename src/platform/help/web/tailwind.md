# Tailwind in Zeb Templates

The compiler turns the Tailwind classes in a page's bundle into CSS at
compile time — no Node, no npm, no `tailwind.config.js`. It implements the
Tailwind 3.4 utility vocabulary plus the handful of v4 spellings that
shadcn/ui components use (`size-*`, `rounded-xs`, `shadow-xs`,
`outline-hidden`, `ring-[3px]`).

Compatibility is tested by CSS meaning, not only by whether a class produces a
rule. A utility the compiler does not know emits nothing — silently — so a
class that has no visible effect is the first thing to suspect.

## Compatibility Boundary

Built in:

- the standard v3 spacing, sizing, layout, flexbox, grid, typography, color, border, effect, table, transform, filter, interaction, SVG, and accessibility families used by common application themes
- composable transforms, filters, gradients, shadows, and rings
- responsive, state, structural, `group-*`, `peer-*`, `has-*`, `aria-*`, `data-*`, `supports-*`, direction, orientation, print, and pseudo-element variants
- arbitrary values such as `w-[320px]` and explicit CSS variables such as `bg-[var(--color-product)]`
- OXC source discovery for conditional classes in TSX and JavaScript string literals

Not part of the current contract:

- loading `tailwind.config.js`
- executing third-party Tailwind plugins
- automatically importing npm packages
- the v4 `@theme` / CSS-first configuration

Use project CSS for plugin-specific class systems. There is no `prose`,
`form-*` or other plugin class; a name that starts with a familiar prefix is
not evidence that it exists.

---

## Standard utilities

```tsx
<div className="flex items-center gap-4 rounded-xl border border-border bg-card p-6">
  <h1 className="text-2xl font-bold text-foreground">Title</h1>
  <p className="text-sm text-muted-foreground">Subtitle</p>
</div>
```

---

## Theme Tokens

Colour utilities name a **role**, never a colour. The names are shadcn/ui's,
verbatim; the project's `globals.css` (scaffolded with every project, imported
by each page with `import "@/globals.css"`) gives each a value under `:root`
(light) and `.dark`. `bg-card` compiles to `background-color: var(--card)`.
Every token takes an alpha: `bg-accent/40`, `border-destructive/30`. Put
`dark` on any ancestor to switch. A theme exported from ui.shadcn.com or
tweakcn replaces the two blocks — every token needs a complete colour value,
and the three extra pairs `success`, `warning`, `info` stay. See the tokens
live at `/dev/design-system` and the components on them at
`/dev/design-system/ui`; contract in `docs/contracts/kinds/ui-theme`.

| Token | Utilities | Role |
|-------|-----------|------|
| `background` / `foreground` | `bg-background text-foreground` | the page |
| `card` / `card-foreground` | `bg-card text-card-foreground` | a raised panel |
| `popover` / `popover-foreground` | `bg-popover` | menus, dialogs, an input's own background |
| `primary` / `primary-foreground` | `bg-primary text-primary-foreground` | the one action colour (brand orange) |
| `secondary` / `secondary-foreground` | `bg-secondary` | a quieter button |
| `muted` / `muted-foreground` | `bg-muted text-muted-foreground` | subdued surface; secondary text |
| `accent` / `accent-foreground` | `hover:bg-accent` | hover / selected-row background |
| `destructive` / `destructive-foreground` | `text-destructive bg-destructive/10` | delete, error |
| `success` / `success-foreground` | `text-success` | live, passed, saved |
| `warning` / `warning-foreground` | `text-warning` | caution, pending |
| `info` / `info-foreground` | `text-info` | neutral notice, links (brand blue) |
| `border` | `border-border divide-border` | every hairline |
| `input` | `border-input` | a control's border |
| `ring` | `ring-ring/40 focus:border-ring` | focus outline |
| `chart-1` … `chart-5` | `bg-chart-1` | series colours, in order |
| `sidebar*` | `bg-sidebar border-sidebar-border` | the navigation rail: `sidebar`, `-foreground`, `-primary`, `-primary-foreground`, `-accent`, `-accent-foreground`, `-border`, `-ring` |

Status notices are tinted by default and solid when filled:

```tsx
<div className="border border-warning/30 bg-warning/10 text-warning">tinted</div>
<span className="bg-success text-success-foreground">solid</span>
```

A raw palette class (`bg-red-500`, `text-gray-400`, `text-white`) is a colour
that does not move when the theme does. `zeb/ui` refuses them by test; pages
should not add new ones. The tokens are a closed list: `bg-accent-strong` or
`text-brand` are not tokens and compile to nothing.

Arbitrary semantic names are deliberately not inferred because names such as `bg-fixed`, `border-collapse`, and `outline-offset-2` are real Tailwind utilities. For a project-specific variable, use an explicit arbitrary value:

```tsx
<div className="bg-[var(--color-product)] text-[var(--color-product-foreground)]" />
```

---

## `cx()` — conditional class names

`import { cx } from "zeb/react"` — like every other name, imported in the
file that uses it. It joins truthy parts:

```tsx
import { cx } from "zeb/react";

<div className={cx("rounded p-4", isActive && "ring-2 ring-ring")}>

<button className={cx(
  "rounded-md px-4 py-2 font-medium transition-colors",
  variant === "primary" && "bg-primary text-primary-foreground hover:bg-primary/90",
  variant === "ghost"   && "text-foreground hover:bg-accent hover:text-accent-foreground",
  variant === "danger"  && "bg-destructive text-destructive-foreground hover:bg-destructive/90",
  size === "sm"         && "px-3 py-1 text-sm",
  disabled              && "pointer-events-none cursor-not-allowed opacity-50",
)}>
```

---

## Dynamic Class Discovery (OXC Source Scanner)

The Tailwind compiler scans **all string and template literals** in the bundled page source
before generating CSS. This means classes in:

- `cx("a", condition ? "b" : "c")` — both branches discovered
- `const cls = "flex items-center"` — scanned at declaration
- `{_isOpen ? "block" : "hidden"}` — both string literals found
- Components that return `null` in SSR — their class strings still found

**`tw-variants` is only needed** when a class is assembled at runtime with no
literal form in the source — `` `bg-${tone}` `` produces no CSS on its own.
Declare the possibilities on any element, as an attribute the compiler reads
and strips:

```tsx
// Not discovered: the literal "bg-success" never appears in source.
<div className={`rounded px-3 py-2 bg-${tone}`} />

// Discovered: list the forms the value can take.
<span hidden tw-variants="bg-success bg-warning bg-info bg-destructive" />

// Discovered without help: both literals are visible.
const cls = condition ? "bg-success" : "bg-warning";
```

`tw-variants="text-[*]"` admits any arbitrary value for that utility.

---

## Variant maps

There is no `cva` and no `tv()`. A component with many permutations keeps a
plain object of literal class strings and joins with `cx` — every string is
found by the compile-time scan, and the shape is the one `zeb/ui` uses:

```tsx
import { cx } from "zeb/react";

const COLORS = {
  default: "bg-muted text-muted-foreground",
  success: "bg-success/15 text-success",
  warning: "bg-warning/15 text-warning",
  danger:  "bg-destructive/15 text-destructive",
};
const SIZES = { sm: "text-[10px] px-1.5 py-0", md: "text-xs px-2 py-0.5", lg: "text-sm px-3 py-1" };

export function Badge({ color = "default", size = "md", className, children }) {
  return (
    <span className={cx("inline-flex items-center rounded-full font-medium", COLORS[color] ?? COLORS.default, SIZES[size] ?? SIZES.md, className)}>
      {children}
    </span>
  );
}
```

## Fonts

`font-<name>` compiles to `font-family: var(--font-<name>, <fallback>)`, so a
font is a variable in `globals.css`, not a class. A new project defines only
`--font-sans` (the system stack). To use a web font, load it with
`page.head.links` and set the variable:

```css
/* globals.css */
:root { --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif; --font-display: "Fraunces", serif; }
```

```tsx
<h1 className="font-display text-3xl">…</h1>   /* var(--font-display, …) */
<code className="font-mono">…</code>           /* var(--font-mono, ui-monospace …) */
```

---

## Rules

| Rule | Detail |
|------|--------|
| Avoid `style=` | Use utility classes; inline styles are for values that are truly data (a computed width, a colour from a dataset). |
| Never `[var(--studio-*)]`, `text-body`, `bg-surface`, `text-ui-text` | Those names are gone. Use the theme tokens: `bg-card`, `text-foreground`, `text-muted-foreground`. |
| Never `[var(--zf-*)]` | Old prefix, gone. |
| `tw-variants` for pure runtime strings | Only needed when a class is assembled from user input or external data with no literal form in source. Auto-discovery handles all normal cases. |
| Prefer theme tokens | `bg-card` over `bg-[#111827]` — it adapts to dark/light theme automatically. |
| Arbitrary values OK when needed | `bg-[#ff5c00]`, `w-[320px]`, `mt-[3px]` are fine for one-offs. |
