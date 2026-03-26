# Tailwind in Zeb Templates

The RWE engine processes Tailwind utility classes at compile time. Standard utilities work as-is. This doc covers Zeb-specific patterns you must follow.

---

## Standard Tailwind — works normally

```tsx
<div className="flex items-center gap-4 p-6 rounded-xl border">
  <h1 className="text-2xl font-bold text-slate-900">Title</h1>
  <p className="text-slate-500 text-sm">Subtitle</p>
</div>
```

---

## Semantic Color Tokens

Zeb defines `--color-*` CSS custom properties that resolve to the active theme (dark or light). Use semantic token classes instead of hardcoded palette values for theme-aware UI.

### Theme tokens (per-theme, change with dark/light)

| Class | CSS property | Semantic meaning |
|-------|-------------|-----------------|
| `bg-bg` | background-color | Page background |
| `bg-surface` | background-color | Card / panel background |
| `bg-surface-2` | background-color | Nested panel |
| `bg-surface-3` | background-color | Deeply nested / hover target |
| `text-body` | color | Primary text |
| `text-body-soft` | color | Secondary / muted text |
| `text-body-muted` | color | Placeholder / hint text |
| `text-accent` | color | Brand orange highlight |
| `bg-accent` | background-color | Accent fill |
| `border-border` | border-color | Standard border |
| `border-border-soft` | border-color | Subtle / inner border |
| `border-accent` | border-color | Accent-colored border |
| `border-b-accent` | border-bottom-color | Active tab underline |

### Global UI tokens (used by `components/ui/` system)

| Class | Token |
|-------|-------|
| `bg-ui-bg` | `--color-ui-bg` |
| `bg-ui-bg-subtle` | `--color-ui-bg-subtle` |
| `bg-ui-bg-muted` | `--color-ui-bg-muted` |
| `border-ui-border` | `--color-ui-border` |
| `text-ui-text` | `--color-ui-text` |
| `text-ui-text-soft` | `--color-ui-text-soft` |
| `text-ui-text-muted` | `--color-ui-text-muted` |

### Brand tokens (fixed, same in all themes)

| Class | Value |
|-------|-------|
| `text-brand-orange` | `#ff5c00` |
| `bg-brand-orange` | `#ff5c00` |
| `text-brand-blue` | `#005b9a` |
| `bg-brand-blue` | `#005b9a` |

**How it works:** Any lowercase hyphen-separated class that isn't a standard Tailwind palette name is treated as a semantic token. `bg-surface` → `background-color: var(--color-surface)`. The CSS defines `--color-surface` under `[data-studio-theme="dark"]` and `[data-studio-theme="light"]` selectors — the browser resolves it automatically.

---

## `cx()` — Conditional Class Names

`cx()` is a global. Concatenates truthy class strings:

```tsx
// Simple conditional
<div className={cx("rounded p-4", isActive && "ring-2 ring-accent")}>

// Multi-variant composition
<button className={cx(
  "px-4 py-2 rounded font-medium transition-colors",
  variant === "primary" && "bg-accent text-white hover:bg-accent-strong",
  variant === "ghost"   && "hover:bg-surface-3 text-body",
  variant === "danger"  && "bg-red-600 text-white hover:bg-red-700",
  size === "sm"         && "text-sm px-3 py-1",
  disabled              && "opacity-50 cursor-not-allowed pointer-events-none",
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

**`tw-variants` is only needed** when a class value is built entirely at runtime
from variables with no string literal form in source:

```tsx
// This would NOT be auto-discovered (pure runtime concatenation):
const prefix = userInput;
const cls = prefix + "-500";   // tw-variants needed if cls used as className

// This IS auto-discovered (string literals visible in source):
const cls = condition ? "bg-red-500" : "bg-blue-500";  // ✅ no tw-variants needed
```

---

## `tv()` — Variant Maps

`tailwind-variants` `tv()` is a global. For components with many permutations:

```tsx
const badge = tv({
  base: "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
  variants: {
    color: {
      default: "bg-surface-3 text-body",
      success: "bg-green-900/40 text-green-300",
      warning: "bg-amber-900/40 text-amber-300",
      danger:  "bg-red-900/40 text-red-300",
      accent:  "bg-accent text-white",
    },
    size: {
      sm: "text-[10px] px-1.5 py-0",
      md: "text-xs px-2 py-0.5",
      lg: "text-sm px-3 py-1",
    },
  },
  defaultVariants: { color: "default", size: "md" },
});

// All variant strings are discovered automatically by the OXC source scanner
// No ghost span needed — the string literals in the tv() map are found statically

// Usage
<span className={badge({ color: "success", size: "sm" })}>Active</span>
<span className={badge({ color: "danger" })}>Error</span>
```

---

## Font tokens

```tsx
<h1 className="font-display text-2xl">  // "Pathway Extreme" display font
<p className="font-sans">               // "Roboto" body font
<code className="font-mono">            // "Roboto Mono"
```

---

## Rules

| Rule | Detail |
|------|--------|
| Never `style=` | Use utility classes. Inline styles are a design system smell. |
| Never `[var(--studio-*)]` | Those old names are gone. Use semantic token classes: `bg-surface`, `text-body`, etc. |
| Never `[var(--zf-*)]` | Old prefix, gone. |
| `tw-variants` for pure runtime strings | Only needed when a class is assembled from user input or external data with no literal form in source. Auto-discovery handles all normal cases. |
| Prefer semantic tokens | `bg-surface` over `bg-[#111827]` — it adapts to dark/light theme automatically. |
| Arbitrary values OK when needed | `bg-[#ff5c00]`, `w-[320px]`, `mt-[3px]` are fine for one-offs. |
