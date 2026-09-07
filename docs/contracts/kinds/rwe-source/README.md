# RweSource

Status: **review** — spec settled 2026-08-30; imports enforced in code.

One authored RWE source file: a page, a component, a `.ts` behavior script, or
a stylesheet. It is the content a project **adds** rather than installs, so it
becomes the receiving project's own editable source.

It has no envelope. A `.tsx` file is governed by convention — which imports are
legal, what a page must export, where files may live — rather than by a
serialization format, which is why it is the first kind with a `SourceFile`
representation.

## Identity

| | |
| --- | --- |
| Representation | source file; no `apiVersion`, no `metadata`, no `spec` |
| Lives | under the project's template root in `repo/` |
| Distributed by | `template_bundle`, `folder_bundle`, and the UI catalogue |
| Compiler | `src/rwe/core/compiler.rs` |

## A page

A page has **exactly one default export component**. Anything else is refused:
`template must have one default export component`.

```tsx
import { useState } from "zeb/react";
import Button from "@/components/ui/button";

export default function Page() {
  const [count, setCount] = useState(0);
  return <Button onClick={() => setCount(count + 1)}>{count}</Button>;
}
```

## Imports

Four forms, and only four.

| Form | Meaning |
| --- | --- |
| `"zeb/react"`, `"zeb/…"` | runtime-provided. Never a file, never bundled. Every file that uses a hook imports it here — there are no implicit globals |
| `"@/…"` | a project file, addressed from the template root |
| `"npm:…"`, `"node:…"`, `"jsr:…"`, `"http://…"`, `"https://…"` | left alone; the compiler does not resolve them |
| `"./…"`, `"../…"` | **refused** |
| `"react"`, `"preact"`, and their submodules | **refused**, naming `"zeb/react"` — the runtime here is not React's |

`"zeb/react"` is the built-in core UI API: hooks, elements, context, refs,
portals, `ErrorBoundary`, `useSyncExternalStore` and rendering functions. It supports named imports, aliases, default
and namespace imports. Unsupported named exports are refused at compile time.
It is available independently of optional library enablement. Zebflow helpers
(`usePageState`, `useRouter`, `Link`, `cx`) remain in `"zeb/react"`; existing hook
imports from `"zeb/react"` are still supported.

### Why relative imports are refused

Two reasons, and the second is the one that decides it.

The compiler resolves a relative path but the bundler does not carry what it
finds, so the file compiles and then fails at runtime — the worst possible
shape for an error.

And these files are mostly written by language models. `@/components/ui/button`
is context-free: correct wherever the file sits, and nothing has to be counted.
`../../components/ui/button` is correct only from one directory, and getting the
depth wrong is the most common mistake a model makes. A rule that removes an
entire class of error is worth more than the convenience it costs.

The refusal must name the replacement — `use "@/components/foo/helper" instead
of "./helper"` — because an error a model can act on is repaired in one step,
and an error it cannot is repaired by guessing.

### Confinement

A resolved import may not leave the template root. `../../../etc/passwd` is
`RWE_IMPORT_BOUNDARY`, not a file read. This holds for every import form.

## What a file may not do

These are engine constraints, not house style.

- **No `document.*`, and no manual `render()`.** A page renders on the server
  first, where there is no DOM, and the runtime owns the single root. Reaching
  for either produces a page that works in one place and not the other.
- **Every file imports its own hooks from `"zeb/react"`.** A component that uses a
  hook without importing it works only when something else happened to import
  it first.

A project may hold further rules — which UI components to prefer, whether to
write CSS at all. Those live in the project's own guidance, not here.

## What travels

A distributed set carries the named file and every file it reaches through
`"@/"`, recursively. `"zeb/react"` imports carry nothing; the runtime provides them.
`npm:` and URL imports carry nothing and are the receiver's problem.

## Collisions

Adding a file that already exists is refused and names the path. Add is not
merge: the receiving project owns its source, and silently overwriting an
authored file is the one outcome nothing can undo.

## Stylesheets

A page reaches CSS the way it reaches anything else: it imports it.

```tsx
import "@/globals.css";
```

There is no implicit global and no automatic root layout, so nothing loads a
stylesheet on a page's behalf. This is Next's mechanism minus the part Zebflow
does not have — `app/layout.tsx` imports `globals.css` once and every page
inherits it, and here a page inherits it from whatever layout component it
imports.

Two properties the compiler guarantees, both of which the alternative designs
depend on:

- **A layout's import reaches the pages built on it.** A component's `.css`
  import is collected while that component is inlined, so a page that imports
  only the layout still carries the rules.
- **A stylesheet reached twice is emitted once.** `collect_inline_style`
  canonicalises the path and checks the shared `visited` set before reading, so
  a diamond — a button imports it, a sidebar imports the button and it, a page
  imports both — inlines one copy.

`globals.css` at the repository root is what a new project starts with. The name
is Next's and Astro's, and it is the only one of the candidates that answers
*when it loads*: `main.css` in a subfolder is just as plausibly a local file,
which is the question a second one raises. Design tokens live in it rather than
in a `theme.css` beside it — Tailwind v4 moved configuration into the CSS for the
same reason, and splitting them out leaves two files each claiming to be the
global one.

Token names follow Tailwind's namespaces — `--color-*`, `--font-*`,
`--spacing-*` — so they read the same as every other Tailwind project and
already match if `@theme` support arrives. They sit in `:root` because this
engine reads `--theme()` inside preflight and not the at-rule. Nothing needs a
custom class to spend one: Tailwind's own arbitrary-value syntax compiles here,
so `bg-[var(--color-surface)]` emits the rule.

## Rejections

A page without exactly one default export. A relative import. An import
escaping the template root. A hook used without an import in that file. An
import from `react`, `react-dom`, `preact` or `preact/hooks` — the refusal names
`"zeb/react"`, because this runtime is not React's and `usePageState`, `useRouter`
and `cx` do not exist there.

## Open

- **Behavior scripts.** A `.ts` file's own import rules are undefined; it is
  not a component and does not export a page.
- **What a stylesheet may import.** `@/…` resolves a `.css` import through a
  separate path, and whether that file may `@import` another is unstated.
- **A root layout.** Nothing wraps every rendered page, so "import it once"
  has nowhere to live. Until that exists, a page with no layout repeats the
  stylesheet import.
- **The compiled artifact.** This kind governs the source. The compiled bundle
  and the wire protocol are the other half of `stability-matrix.md` row 13 and
  are still one undivided row.
