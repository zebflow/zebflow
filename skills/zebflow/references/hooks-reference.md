# Zeb Hooks — Quick Reference

All hooks and utilities are imported from `"zeb"`. Required in every file that uses them.

```tsx
import { useState, useEffect, useRef, useMemo, usePageState, useNavigate, Link, cx, tv } from "zeb";
```

---

## useState

```tsx
const [value, setValue] = useState(initialValue);
```

Local component state. Use for UI toggles, form fields, dropdown open/close.

---

## useEffect

```tsx
useEffect(() => {
  // runs on client only, after mount
  const handler = (e) => { /* ... */ };
  window.addEventListener("resize", handler);
  return () => window.removeEventListener("resize", handler);
}, [dep1, dep2]);
```

Never runs during SSR. `[]` = run once. Omit deps = run every render.

---

## useRef

```tsx
const ref = useRef(null);
return <div ref={ref} />;
```

Access DOM node via `ref.current`. Also for stable mutable values that don't trigger re-render.

---

## useMemo

```tsx
const expensive = useMemo(() => computeValue(data), [data]);
```

Recomputes only when deps change.

---

## usePageState

### Object form

```tsx
const state = usePageState(input.state ?? { count: 0, items: [] });

// Direct mutation on client → DOM update
state.count++;
state.items = [...state.items, newItem];
```

### Keyed form

```tsx
const [count, setCount] = usePageState("count", 0);
const [title, setTitle] = usePageState("title", "Hello");

setCount(count + 1);
setTitle("New Title");
```

### Rules

- Use `usePageState` for **page-specific** components that reflect server data
- Use `useState` for **generic UI** components (buttons, dialogs, dropdowns)
- Anything in `components/ui/` should use only `useState`
- `usePageState` belongs in page components and page-specific sub-components

---

## useNavigate

```tsx
const navigate = useNavigate();

async function handleSubmit(e) {
  e.preventDefault();
  await fetch("/api/posts", { method: "POST", body: JSON.stringify(data) });
  navigate("/posts");
}
```

Client-only. No-op during SSR.

---

## Link

```tsx
<Link href="/posts/1" className="underline hover:text-accent">Read post</Link>
```

Renders `<a>` during SSR (SEO). Activates SPA routing on hydration.

---

## cx

```tsx
<div className={cx(
  "rounded p-4",
  isActive && "ring-2 ring-accent",
  variant === "primary" && "bg-accent text-white",
  disabled && "opacity-50 pointer-events-none",
)} />
```

Concatenates truthy class strings. Falsy values are skipped.

---

## tv

```tsx
const button = tv({
  base: "px-4 py-2 rounded font-medium transition",
  variants: {
    variant: {
      primary: "bg-accent text-white hover:bg-accent-strong",
      ghost: "bg-transparent text-body hover:bg-surface-3",
    },
    size: {
      sm: "text-sm px-3 py-1",
      md: "text-base px-4 py-2",
    },
  },
  defaultVariants: { variant: "primary", size: "md" },
});

// REQUIRED: register all variant strings with the Tailwind compiler
<span hidden tw-variants="bg-accent text-white hover:bg-accent-strong bg-transparent text-body hover:bg-surface-3 text-sm px-3 py-1 text-base px-4 py-2" />

// Usage
<button className={button({ variant: "ghost", size: "sm" })}>Click</button>
```

The `tw-variants` hidden span is required — without it, the compiler won't generate CSS for dynamically computed classes.
