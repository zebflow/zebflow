# Scripts (`.ts` modules) and what the compiler refuses

A script is a plain `.ts` file in the project, imported by pages and
components with `@/`. Use one for logic that is too specific for a library:
formatting, validation, small state machines, API clients.

## Workflow

```
file_create  kind=script  name=format-address  parent_rel_path=scripts   → scripts/format-address.ts
file_write   rel_path=scripts/format-address.ts  content="<the module>"
```

```ts
// scripts/format-address.ts
export type AddressInput = { street: string; city: string };

export function formatAddress(input: AddressInput) {
  const street = String(input?.street || "").trim();
  const city = String(input?.city || "").trim();
  if (!street || !city) throw new Error("street and city are required");
  return { label: `${street}, ${city}` };
}
```

```tsx
import { formatAddress } from "@/scripts/format-address";
```

## Rules

1. Import with `@/`; never a relative path.
2. A script that uses a hook or `cx` imports it from `"zeb/react"` like any other file.
3. Keep names unique across the page: every imported file is inlined into one
   bundle per page, and two files exporting the same name onto the same page
   is a compile error (`RWE_BUNDLE_NAME_COLLISION`). Non-exported top-level
   names are prefixed per file and never collide, whatever their case.
4. No module-level side effects — the module also runs on the server.
5. No credentials, tokens or private URLs; the bundle is delivered to the browser.
6. `npm:`, `jsr:`, `node:` and `http(s):` specifiers are passed through
   unresolved, not bundled — a browser cannot load them, so a page must not
   depend on one. Capabilities come from `zeb/*` libraries (`help("web/libraries")`),
   Hub packages, or your own code.

## What the compiler refuses, and why

| Code | Trigger | Instead |
|---|---|---|
| `RWE_HOOK_NOT_IMPORTED` | a hook used without an import | `import { useState } from "zeb/react"` in that file |
| `RWE_IMPORT_NOT_ALLOWED` / relative-import refusal | `from "react"`, `from "../x"` | `"zeb/react"`, `"@/…"` |
| `RWE_IMPORT_NAMESPACE` | `import * as x from "@/…"` in a component | named imports |
| `RWE_DEFAULT_EXPORT_ANONYMOUS` | `export default () => …` | `export default function Name() {}` |
| `RWE_BUNDLE_NAME_COLLISION` | two inlined files export the same name | rename one |
| `RWE_SECURITY_GLOBAL` | `eval`, `Function` | none — write the code |
| `RWE_SECURITY_RAW_HTML` | `dangerouslySetInnerHTML` (always off) | render elements; `<Markdown>` / `renderDocumentHtml` on the server for rich text |
| `RWE_SECURITY_DYNAMIC_IMPORT` | `import()` while the project's strict mode is on (the default) | static `import { d3 } from "zeb/d3"` |
| `RWE_SECURITY_FETCH` | a literal `fetch("https://host/…")` to a host outside the project's allow-list (default: `registry.npmjs.org`, `jsr.io`) | call your own webhook (`/wh/…`) and let `http.request` reach the outside; or add the host in Settings → Policy |

A refusal is a compile error at save time (`file_write` reports it; `POST
/templates/diagnostics` checks without saving), which is the point: the
alternative is a page that ships and fails in the browser.
