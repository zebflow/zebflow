# RWE Template Authoring

RWE (Reactive Web Engine) renders TSX templates server-side into HTML.

## Page Template Structure

```tsx
// Required: default export is the page component
export default function Page(input: PageInput) {
  const state = usePageState(input.state ?? { count: 0 });

  return (
    <div class="p-4">
      <h1 class="text-2xl font-bold">{state.title}</h1>
      <p>{state.count}</p>
      <button onClick={() => state.count++}>Increment</button>
    </div>
  );
}

// Required: page metadata
export const page = {
  title: "My Page",
  description: "A page description",
};

// Required: app config
export const app = {
  hydration: "reactive",   // "reactive" | "static" | "none"
};
```

## `usePageState(initialState)`

Returns a reactive state object. On server: renders with the initial snapshot. On client: enables reactivity.

```tsx
const state = usePageState({ count: 0, title: "Hello" });
// state.count, state.title are reactive
```

## PageInput Type

```ts
interface PageInput {
  state?: Record<string, unknown>;  // initial state from pipeline
  request?: {
    method: string;
    path: string;
    query: Record<string, string>;
    headers: Record<string, string>;
    body?: unknown;
  };
}
```

## Hydration Modes

- `"reactive"` — full client-side reactivity, JavaScript loaded
- `"static"` — SSR only, no client JS
- `"none"` — raw HTML string, no hydration wrapper

## Component Imports

Import components using relative paths:
```tsx
import Button from "../components/ui/button";
import { Card } from "../components/ui/card";
```

## Styling

Use Tailwind CSS utility classes. Classes are processed at render time.

```tsx
<div class="flex items-center gap-4 p-6 bg-white rounded-lg shadow">
  <span class="text-sm text-gray-500">Label</span>
</div>
```

## Template API Endpoints

```
GET  /api/projects/{owner}/{project}/templates              — list workspace
GET  /api/projects/{owner}/{project}/templates/file?rel_path=pages/home.tsx
PUT  /api/projects/{owner}/{project}/templates/file         — save file
POST /api/projects/{owner}/{project}/templates/create       — create new
DELETE /api/projects/{owner}/{project}/templates/file       — delete
POST /api/projects/{owner}/{project}/templates/compile      — check for errors
```
