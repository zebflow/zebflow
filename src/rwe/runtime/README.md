# Zeb React

Zebflow owns the UI runtime used by RWE. `zeb_react.js` contains original,
dependency-free JavaScript shared by embedded V8 SSR workers and the browser.
`zeb_react.mjs` exposes the browser module API. `zeb_ssr_init.js` supplies
Zebflow page state, navigation stubs, island metadata and library placeholders.
There is no Preact bundle, npm install, CDN request or build step for this engine.

The Rust compiler transforms TSX into `h` / `Fragment` calls using OXC. Template
authors import core APIs from `zeb/react`; existing `zeb` hook imports remain compatible. Generated browser modules import the
embedded `/assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs` adapter. Static
exports copy the adapter and its adjacent `.js` implementation together. The
runtime is internal infrastructure, not an installable Hub library.

## Imports and API

```tsx
import { useState, useEffect, useRef } from "zeb/react";
import { usePageState, useNavigate, Link, cx } from "zeb";
```

`zeb/react` supports named imports, aliases, default imports (`import React
from "zeb/react"`) and namespace imports (`import * as React from "zeb/react"`).
It is always available, regardless of enabled optional libraries. The compiler
binds these imports to the same engine instance on server and browser; it
refuses unsupported named exports. `src/rwe/core/zeb_react.rs` defines this
import contract. Zebflow-specific page state, navigation and class-name helpers
remain in `zeb`.

| Category | Exports from `zeb/react` |
| --- | --- |
| State | `useState`, `useReducer`, `useSyncExternalStore` |
| Recovery | `ErrorBoundary` (Zeb-specific component) |
| Effects | `useEffect`, `useLayoutEffect` |
| Refs and IDs | `useRef`, `useImperativeHandle`, `forwardRef`, `useId` |
| Memoization | `useMemo`, `useCallback`, `memo` |
| Context | `createContext`, `useContext` (with Provider and Consumer) |
| Elements | `createElement`, `h`, `Fragment` |
| Portals | `createPortal` |
| Low-level rendering | `render`, `hydrate`, `renderToString` |
| JSX compiler helpers | `jsx`, `jsxs`, `jsxDEV` |

RWE mounts and hydrates template pages automatically. Low-level rendering
functions are for integrations that own their own DOM container; normal page
components should not call them.

## Supported contract

- Function components, nested children, fragments and keyed reconciliation.
- Hydration that reuses matching DOM, splits coalesced text nodes, removes stale
  server nodes and repairs mismatches. Early edits to input values survive the
  hydration commit; subsequent controlled updates apply the supplied value.
- `useState`, functional setters, `useReducer`, stable refs, `useMemo`,
  `useCallback`, scoped context, `useId`, `useEffect`, `useLayoutEffect`,
  `useImperativeHandle`, `forwardRef`, `memo` and context-preserving portals.
- HTML properties, boolean and ARIA attributes, SVG namespaces, CSS units,
  controlled inputs/selects, raw HTML and native DOM event handlers with capture.
- Effect cleanup, ref release and portal removal on unmount. Imperative library
  children survive updates to their managed wrapper.
- Escaped SSR HTML, scoped providers and deterministic IDs per render. SSR never
  executes effects or subscribes to stores. Boundaries can render server fallbacks.
  Uncaught errors from direct `renderToString` calls throw; the RWE adapter
  preserves the existing `RWE component error` comment contract outside boundaries.

Instances own hook slots and child identity. State updates mark the affected
instance and its ancestor path, then batch into a microtask. Unaffected component
instances reuse their previous output. Keyed siblings match by key and type;
unkeyed siblings match by position and type. Refs attach after DOM placement,
layout effects run at commit and passive effects run in a later task. A full
unmount discards the root so RWE navigation can hydrate the next server page.

## Error boundaries and recovery

`ErrorBoundary` is a built-in Zeb component, not React's class-component API.
It catches descendant render, ref, effect, subscription and cleanup failures.
Other branches retain their state and DOM. Failed branches are fully unmounted;
retrying mounts fresh children. Pending effects and subscriptions from abandoned
renders are discarded, including portal children.

```tsx
import { ErrorBoundary } from "zeb/react";
import Editor from "@/components/editor";

export default function EditorPanel({ documentId }) {
  return (
    <ErrorBoundary
      resetKeys={[documentId]}
      onError={(error, info) => console.error(error, info.componentStack)}
      fallbackRender={({ resetErrorBoundary }) => (
        <section role="alert">
          <p>The editor could not render.</p>
          <button onClick={() => resetErrorBoundary()}>Try again</button>
        </section>
      )}
    >
      <Editor documentId={documentId} />
    </ErrorBoundary>
  );
}
```

- Supply `fallback` (any renderable value, including `null`) or
  `fallbackRender({ error, resetErrorBoundary })`. `fallbackRender` takes priority.
  Without either, the error propagates to the next boundary or caller.
- `onError(error, { componentStack })` runs once per captured failure, on the
  server as well as the browser. Treat thrown values as unknown, not always `Error`.
- `resetErrorBoundary(...args)` retries. Optional `onReset` runs first with
  `{ reason: "imperative-api", args }`; use it to repair the failed input/store.
- Changing a value in `resetKeys` while failed also retries, comparing each value
  with `Object.is`. `onReset` receives `{ reason: "keys", prev, next }`.
- A failed retry shows the fallback again. Errors in the fallback or boundary
  callbacks propagate to an enclosing boundary. Render fallback components with
  JSX; do not call hooks directly inside `fallbackRender`.
- Event-handler errors, rejected promises, and independently scheduled timer
  callbacks are outside the boundary. Handle them at their source. An effect
  callback's synchronous errors are covered; its later asynchronous work is not.

Unlike React's class error boundaries, Zeb boundaries also render fallbacks in
synchronous SSR and static generation. Reset is a no-op on the server. Hydration
tries the children again and attaches recovery controls if they still fail.

## External stores

```tsx
import { useSyncExternalStore } from "zeb/react";

const listeners = new Set();
let count = 0;
function subscribe(notify) {
  listeners.add(notify);
  return () => listeners.delete(notify);
}
function getSnapshot() { return count; }
function getServerSnapshot() { return 0; }
function increment() {
  count++;
  for (const notify of listeners) notify();
}

export default function Counter() {
  const value = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
  return <button onClick={increment}>Count {value}</button>;
}
```

The signature follows React's `useSyncExternalStore` contract. `subscribe` must
return an unsubscribe function. Keep its identity stable to avoid unnecessary
resubscription. Changing `getSnapshot` alone does not resubscribe. Snapshots are
compared with `Object.is`; object snapshots must be immutable and cached until
the store changes. Uncached results fail explicitly instead of causing a loop.

`getServerSnapshot` is required for SSR and hydration, and must return the same
initial value on both sides. For request data, derive it from the serialized
page input; never put request-specific mutable data into a shared server store.
Server rendering never invokes `subscribe` or the browser getter. Hydration
first uses the server snapshot, then checks and synchronously applies live data.

Notifications update synchronously outside an active render/commit. During a
commit, changes are reconciled before it completes. Every mounted subscriber is
checked before rendering, including memoized siblings. Checks after subscription
cover changes between rendering and listening. Unmounting, navigation, failed
renders and store replacement release subscriptions; stale callbacks are ignored.
This is a synchronous engine, without React's concurrent transition scheduling.

## Deliberate limits

This implements Zebflow's function-component API, not the complete React or
Preact package APIs. Class components, Suspense, concurrent rendering,
transitions, React synthetic events and streaming hydration
are not implemented. Events follow native DOM semantics (for example, text
input changes use `onInput`). IDs are scoped to RWE's single page root; consumers
building multiple independent roots must provide their own ID namespace.
Render functions must be synchronous and follow hook ordering rules. Functional
state setters must be pure. Server and initial client output should agree;
browser-only libraries retain their existing SSR placeholder adapters.

This initial engine needs continued regression coverage and workload benchmarks
before making React/Preact parity or performance claims. RWE's historical
component-error comments can conceal broken templates, so browser checks and
explicit assertions against those comments remain necessary.

## Verification

Run from the repository root:

```sh
node --test tests/rwe/runtime/ssr.test.mjs
cargo test --test rwe
cargo test --lib rwe::
cargo test --lib web_static
cargo test --lib web_docs_generate
cargo check
```

The dependency-free browser harness is
`tests/rwe/runtime/browser.html`. Serve the repository with
`python3 -m http.server 10631 --bind 127.0.0.1`, open
`http://127.0.0.1:10631/tests/rwe/runtime/browser.html` in a real browser, and
inspect its results. Every entry in `globalThis.__zebReactTestResults` must have
`passed: true`. It checks identity, events, forms, effects, keyed fragments,
context, portals, memo scheduling and repeated navigation hydration. The adjacent
`recovery.mjs` cases exercise nested boundary failures, resets, cleanup, store
consistency, subscription races and SSR-to-browser snapshot handoff.

For Studio integration, rebuild an isolated instance with `bash dev.sh 10630`,
wait for its health endpoint, then run
`ZEBFLOW_BASE_URL=http://127.0.0.1:10630 npm test --prefix tests/e2e`.
The existing database and repository-tree cases assume a populated project:
at least one SQL table, root `README.md`, and `pipelines/` and `schemas/` folders.
Prepare fixtures through platform APIs, never by modifying project storage.
