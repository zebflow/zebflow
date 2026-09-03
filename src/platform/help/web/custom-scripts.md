# Custom TypeScript Scripts

Custom scripts are normal `.ts` files in the project template tree. Use them
for focused behavior that is too specific for a shared Zeb Library.

## MCP Workflow

Create the scaffold first:

```text
file_create kind=script name=format-address
```

Read the returned scaffold or fetch it directly:

```text
file_read rel_path=scripts/format-address.ts
```

Write the complete module:

```text
file_write rel_path=scripts/format-address.ts content="<complete TypeScript source>"
```

Read it again after writing and test the page that imports it.

## Script Rules

1. Export functions and values with camelCase names.
2. Use `@/` for project-local imports.
3. Do not import npm, JSR, CDN, React, Preact, or Node packages.
4. Do not call `render()` or create another application root.
5. Avoid module-level side effects.
6. Validate inputs and return clear errors.
7. Never include credentials, tokens, or private URLs.

Example:

```ts
export type AddressInput = {
  street: string;
  city: string;
};

export function formatAddress(input: AddressInput) {
  const street = String(input?.street || "").trim();
  const city = String(input?.city || "").trim();
  if (!street || !city) {
    throw new Error("street and city are required");
  }
  return { label: `${street}, ${city}` };
}
```

Import it from a page or component:

```tsx
import { formatAddress } from "@/scripts/format-address";
```

## Larger Capabilities

Search Hub for reviewed scripts and templates. Enable a bundled `zeb/*`
library for complex capabilities such as maps, charts, rich text, code editing,
or 3D. Direct npm ingestion is not supported.
