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

## `tw-variants` — Register Dynamically Built Classes

The RWE engine scans `className` attribute strings at compile time. If you build class names dynamically from variables, those strings never appear literally in the source and won't make it into the generated CSS.

Fix: add a hidden `<span tw-variants="...">` with all the class strings you need:

```tsx
// Declare all dynamically-used classes here — engine scans this
<span hidden tw-variants="bg-sky-900 border-accent text-red-400 bg-surface-2 opacity-50 bg-accent text-white" />

// Now safe to compose dynamically
const bgClass = color === "sky" ? "bg-sky-900" : "bg-surface-2";
<div className={cx("rounded p-4", bgClass)}>
```

**Always add `tw-variants` when you concatenate class names from variables or ternaries.**

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

// Register all variant strings (required for engine to include them)
<span hidden tw-variants="bg-surface-3 text-body bg-green-900/40 text-green-300 bg-amber-900/40 text-amber-300 bg-red-900/40 text-red-300 bg-accent text-white text-[10px] px-1.5 py-0 text-xs px-2 py-0.5 text-sm px-3 py-1" />

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
| Always `tw-variants` for dynamic classes | If a class string is composed from variables, register it with `tw-variants`. |
| Prefer semantic tokens | `bg-surface` over `bg-[#111827]` — it adapts to dark/light theme automatically. |
| Arbitrary values OK when needed | `bg-[#ff5c00]`, `w-[320px]`, `mt-[3px]` are fine for one-offs. |
