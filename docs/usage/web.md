# Web

Zebflow renders TSX directly. A project does not need a separate frontend build
project for normal pages and components.

## Main Tools

- Zeb React provides components, state, effects, and events.
- Zeb Tailwind turns supported utility classes into CSS.
- RWE compiles TSX, renders the first HTML, and adds browser behavior.
- Pipelines load data and return pages or API responses.

## Small Page

```tsx
export default function Home({ input }) {
  return (
    <main className="mx-auto max-w-4xl p-6">
      <h1 className="text-2xl font-semibold">{input.title}</h1>
    </main>
  );
}
```

Use `className`, not `class`. Use Zeb React and Zeb Tailwind for project UI.

## Source Layout

A common layout under `repo/pipelines/` is:

```text
pages/
components/
styles/
scripts/
assets/
```

This is one source tree. Pages may import nearby components and supported Zeb
libraries.

## Data Flow

A page usually receives the last pipeline payload as `input`. Web trigger data
such as route parameters, query values, and safe public auth claims are exposed
through the render context. Keep private claims and secrets on the server.

## Work Loop

1. Create the TSX file.
2. Compile it in Project Studio.
3. Connect it to a web response pipeline.
4. Open the route in a browser.
5. Test loading, empty, error, and mobile states.

When a normal class or component fails in several pages, fix the shared RWE or
Zeb Tailwind behavior. Do not add a different workaround to every page.
