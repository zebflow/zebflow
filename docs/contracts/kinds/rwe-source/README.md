# RweSource

Status: **review** — spec settled 2026-08-30, code catch-up owed.

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
import { useState } from "zeb";
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
| `"zeb"`, `"zeb/…"` | runtime-provided. Never a file, never bundled. Every file that uses a hook imports it here — there are no implicit globals |
| `"@/…"` | a project file, addressed from the template root |
| `"npm:…"`, `"node:…"`, `"jsr:…"`, `"http://…"`, `"https://…"` | left alone; the compiler does not resolve them |
| `"./…"`, `"../…"` | **refused** |

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
- **Every file imports its own hooks from `"zeb"`.** A component that uses a
  hook without importing it works only when something else happened to import
  it first.

A project may hold further rules — which UI components to prefer, whether to
write CSS at all. Those live in the project's own guidance, not here.

## What travels

A distributed set carries the named file and every file it reaches through
`"@/"`, recursively. `"zeb"` imports carry nothing; the runtime provides them.
`npm:` and URL imports carry nothing and are the receiver's problem.

## Collisions

Adding a file that already exists is refused and names the path. Add is not
merge: the receiving project owns its source, and silently overwriting an
authored file is the one outcome nothing can undo.

## Rejections

A page without exactly one default export. A relative import. An import
escaping the template root. A hook used without an import in that file.

## Open

- **Behavior scripts.** A `.ts` file's own import rules are undefined; it is
  not a component and does not export a page.
- **Stylesheets.** `@/…` resolves a `.css` import through a separate path;
  what a stylesheet may import is unstated.
- **The compiled artifact.** This kind governs the source. The compiled bundle
  and the wire protocol are the other half of `stability-matrix.md` row 13 and
  are still one undivided row.
