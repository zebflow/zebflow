---
name: zebflow-rwe
description: Writing or changing a TSX page, component or script in a Zebflow project — the RWE engine, zeb/react, imports, input, page config, hydration, Tailwind classes, and the compile-then-fetch-then-open loop. Use before any .tsx or .ts write.
license: MIT
metadata:
  version: "1"
---

# Pages on the RWE engine

A page is a TSX file the server renders and the browser hydrates. It is not
React-in-Node: there is no build, no npm, no `react` import, and one flat
bundle per page. Facts: `help(topic="web")`, `web/hooks`, `web/tailwind`,
`web/custom-scripts`, `web/libraries`.

## The contract, in order of how often it is broken

1. **One door for imports.** `import { useState, useEffect, cx, Link } from "zeb/react"`;
   `import { Button } from "zeb/ui/button"`; `import X from "@/components/x"`;
   `import "@/globals.css"`. Never `react`, `preact`, npm, a CDN, or `../`.
2. **Every file imports its own names.** A component that uses `useState` or
   `cx` without importing it works on one page and throws `X is not defined`
   on the next. Nothing is inherited.
3. **`input` is the payload.** The page's parameter is what the pipeline's last
   node produced, plus `route`, `params`, `query`, `search`, `headers`,
   `auth`. After `sekejap.query` that is `input.rows`. There is no
   `input.state`, no `input.request`.
4. **Server data from `input`; client state from hooks.** `useState` for local
   state, `usePageState("key", default)` for state shared across the page's
   components (`[value, setter]`). Do not read `window` or `document` during
   render — the server has neither; that belongs in `useEffect`.
5. **Class strings are literals.** The compiler scans the source for classes;
   `` `bg-${tone}` `` produces no CSS. Colours are theme roles (`bg-primary`,
   `text-muted-foreground`, `border-border`), never `bg-slate-500`, and a
   token that is not in the list (`text-muted`, `bg-accent-strong`) compiles
   to nothing, silently.
6. **One bundle per page.** Two imported files may not export the same name
   (`RWE_BUNDLE_NAME_COLLISION`); the platform's own `@/components/ui/*` and
   `zeb/ui/*` may not share a page.
7. **Runtime libraries are static imports** — `import { d3 } from "zeb/d3"`.
   Dynamic `import()` is refused under the default policy, as are
   `dangerouslySetInnerHTML`, `eval`, and a literal `fetch` to an outside host.

## Page shape

```tsx
import { useState } from "zeb/react";
import { Button } from "zeb/ui/button";
import PostCard from "@/components/post-card";
import "@/globals.css";

export default function Posts(input) {
  const posts = input.rows ?? [];
  const [open, setOpen] = useState(false);
  return (
    <main className="mx-auto max-w-3xl p-8">
      <h1 className="text-2xl font-semibold text-foreground">Posts</h1>
      {posts.map((p) => <PostCard key={p.id} post={p} />)}
      <Button variant="outline" onClick={() => setOpen(!open)}>{open ? "Hide" : "Show"} filters</Button>
    </main>
  );
}

export const page = {
  head: { title: "Posts" },
  body: { className: "bg-background text-foreground" },
};

export function getPage(input) {            // optional: head derived from data
  return { head: { title: `${input.rows?.length ?? 0} posts` } };
}
```

`page.head` knows `title`, `description`, `canonical`, `robots`, `icons`,
`links` (a stylesheet), `scripts`, `og`, `twitter`, `extra`. Defer a heavy
subtree with `hydrate="onview" | "oninteract" | "off"` on its element.
Navigation is `<Link href>` and `useRouter().push()`.

## The loop

Every write follows the same four steps; skipping one is how a broken page
gets reported as done.

1. **Write.** `file_create kind=page name=posts parent_rel_path=pages` then
   `file_write rel_path=pages/posts.tsx`. Components in `components/`,
   scripts in `scripts/`. A save evicts every compiled page that inlined the
   file; if something still looks old, `POST /api/projects/{o}/{p}/rwe/cache/clear`.
2. **Compile check.** The write itself reports a compiler refusal
   (`RWE_HOOK_NOT_IMPORTED`, `RWE_IMPORT_NOT_ALLOWED`, …); read it and fix
   it before going further. `POST /templates/diagnostics` checks without saving.
3. **Fetch.** Get the route and read the body. Search it for
   `RWE component error` — a throwing component is replaced by that comment
   and the response is still 200. Check the data you expected is in the HTML.
4. **Open.** A browser, with the console visible. A console error means
   hydration failed even though the server HTML was right — the usual causes
   are a missing import in a component and `window`/`document` during
   render. Then perform the interaction you built and check the state it
   should produce (`zebflow-verify`).

## Components and scripts

- A component takes props and imports its own names; it never reaches for a
  page variable.
- A module (`scripts/*.ts`) exports functions; no module-level side effects
  — it runs on the server too. Keep exported names unique across the page.
- Shared behaviour belongs in a hook or a component, not copied between
  pages; the second copy is the one that drifts.

## When something is off

Anything that "should normally work" in zeb/react or the compiler and does
not — a hook that behaves differently on the server, a class that needs a
workaround, a component that only works on one page — is a foundation
problem, not a page problem. Do not paper over it in the page with a
`globalThis` alias, a DOM hack or a duplicated helper. Say what you saw,
with the file and the line, and stop the workaround path.
