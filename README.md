# Zebflow

Deploy once, evolve safely

Primary docs:

1. [Zebflow Overview](./docs/OVERVIEW.md)
2. [Zebflow RWE](./docs/RWE.md)
3. [Zebflow Platform Web](./docs/PLATFORM_WEB.md)

Zebflow is a framework runtime for:

1. Pin-based pipeline orchestration.
2. Sandboxed user scripting.
3. Reactive web rendering.
4. Platform web shell (login/home/project flow) with swappable adapters.

The crate is library-first. `main.rs` (Axum/app boot) can stay thin and call exported APIs from `lib.rs`.

## What Is Zebflow

Zebflow is an observable automation + fullstack runtime with one core design:

1. pipeline orchestration is Rust-first (`*.zf.json`)
2. scripting is sandboxed and portable (`language` engines)
3. web rendering is SSR-first and reactive (`*.tsx`)
4. theme and base CSS are compile-scoped from the template tree (`styles/base.css`, `styles/main.css`)

In practice:

1. `framework` executes graph/pipeline nodes
2. `language` executes script logic for nodes/pages
3. `rwe` compiles and renders web templates with lean hydration
4. `platform` composes adapters/services/web for multi-user project management

This crate contains the runtime primitives for all three, not a heavy app server by itself.

## Product Focus Areas

1. Lean fullstack reactive web development.
    - Web API
    - Fullstack reactive website
2. Data engineering and analysis.
    - Long-running worker mode
    - Adhoc analysis and visualization
3. AI workflow management.
4. Real-time data processing.
    - Game
    - IoT

## Core Modules

1. `framework`
Responsibility: orchestration engine for `*.zf.json` graph execution.
What it owns: node/edge graph model, pin routing, graph validation, execution contract, run trace envelope.
What it does not own: JavaScript sandbox internals, HTML rendering internals.

2. `language`
Responsibility: portable script runtime and compilation pipeline.
What it owns: parse/compile/run interface, engine registry, sandbox implementations.
Current engine: Deno sandbox with policy config, loop guards, allow-listed fetch, process-level limits.

3. `rwe` (Reactive Web Engine)
Responsibility: template compile/render layer that can depend on `language`.
What it owns: template contracts, render contracts, RWE engine registry.
What it does not own: graph traversal logic.

4. `platform`
Responsibility: web app shell and service composition for Zebflow runtime operations.
What it owns: login flow, user/project services, adapter factories, filesystem project layout, platform routes.
What it does not own: low-level pipeline execution internals or template compiler internals.

## What `framework` Means Here

`framework` is not a UI framework.
`framework` is the orchestration runtime layer for node graphs:

1. Read a pipeline graph.
2. Validate node/pin connectivity.
3. Execute nodes in graph flow.
4. Emit deterministic traces/observability events.

It is the execution control plane above `language` and `rwe`.

## Folder Responsibilities

`Cargo.toml`
Crate package and dependency boundary.

`conventions/`
Canonical examples and file conventions used by Zebflow contracts.

`conventions/pipelines/`
Example pipeline contracts (`*.zf.json`), pin-based (`from_pin` -> `to_pin`).

`conventions/templates/`
Example template sources (`*.tsx`) used by RWE.

`runtime/`
Runtime assets required by engines.
Current use: Deno sandbox JS runner (`secure_js_runner.js`).

`src/lib.rs`
Top-level assembly point and engine kit wiring.

`src/framework/`
Pipeline orchestration module.

`src/framework/interface.rs`
Framework engine trait contract (`validate_graph`, `execute`).

`src/framework/model.rs`
Pipeline graph domain model (`PipelineGraph`, `PipelineNode`, `PipelineEdge`).

`src/framework/registry.rs`
Framework engine registry for variant implementations.

`src/framework/engines/`
Concrete framework engine implementations.

`src/framework/nodes/`
Framework-level node interfaces and node implementations.
This is where pipeline node contracts live (not in `rwe` or `language`).

`src/framework/nodes/basic/`
Built-in/basic node set.

`src/framework/nodes/basic/web_render/mod.rs`
`x.n.web.render` node interface and adapter that composes pure `rwe` + pure `language`.

`src/language/`
Script/runtime module.

`src/language/interface.rs`
Language engine trait (`parse`, `compile`, `run`).

`src/language/model.rs`
Language domain model (IR, compiled artifact, execution context/output).

`src/language/registry.rs`
Language engine registry for pluggable runtimes.

`src/language/engines/`
Concrete language engines.

`src/language/engines/deno_sandbox/`
Sandboxed Deno engine internals.

`src/language/engines/deno_sandbox/config.rs`
Security config model and layered patch merge.

`src/language/engines/deno_sandbox/instrument.rs`
Source policy checks and loop guard instrumentation.

`src/language/engines/deno_sandbox/runner.rs`
Subprocess execution boundary to Deno runtime.

`src/language/engines/deno_sandbox/engine.rs`
Public engine surface and `LanguageEngine` integration.

`src/rwe/`
Reactive Web Engine module.

`src/platform/`
Platform module (adapters + services + web routes).

`src/platform/adapters/`
Swappable adapter layer:
- `data` adapters (`sekejap`, `sqlite` placeholder, `dynamodb` placeholder, `firebase` placeholder)
- `file` adapters (`filesystem` current default, git-sync friendly)

`src/platform/services/`
Service layer:
- `platform` bootstrap composition
- `auth` login/session checks
- `user` user management
- `project` project management + directory provisioning

`src/platform/web/`
Axum handlers and HTML pages for platform flow:
- `/login`
- `/home`
- `/projects/{owner}/{project}`
- `/api/...`

`src/bin/zebflow_platform.rs`
Runnable Zebflow platform server entrypoint (thin `main` that calls library module).

`src/rwe/interface.rs`
RWE trait contract (`ReactiveWebEngine`) for template compile/render only.

`src/rwe/model.rs`
Template, compile artifact, render output, and RWE option contracts:
- style engine mode (`tailwind-like` / off)
- runtime mode (dev/prod runtime bundle)
- reactive mode (`@click`, `j-text`, `j-model`, `j-attr:*` extraction)
- resource allow-list (`css`, `scripts`, `urls`)
- language run patch pass-through (`language.run_patch`) for sandbox control
- processor pipeline (`processors`) for feature toggles (`tailwind`, `markdown`)

`src/rwe/registry.rs`
RWE engine registry for pluggable render backends.

`src/rwe/engines/`
Concrete RWE engine implementations.

`src/rwe/processors/`
Compile-stage processor pipeline resolver. This is where feature toggles are applied.

`src/rwe/processors/tailwind/`
Tailwind-like style compiler module used by the processor pipeline.

`src/rwe/processors/tailwind/mod.rs`
Tailwind processor facade and API surface.

`src/rwe/processors/tailwind/compiler.rs`
Token scanner + utility compiler, including:
- responsive/pseudo variants (`md:`, `hover:`, etc.)
- arbitrary values (`w-[24rem]`, etc.)
- CSS injection into `<style data-rwe-tw>`

`src/rwe/processors/markdown/`
Markdown processor module for compile-time conversion of `<markdown>...</markdown>` blocks.

`src/rwe/MILESTONE.md`
RWE roadmap focused on modularity, SSR-first rendering, and lean hydration.

`src/rwe/ADAPTER.md`
Cross-language integration guide (for FastAPI/Node/etc.) using JSON protocol envelopes.

`tests/`
Integration test entrypoint split by domain.

`tests/framework/`
Framework-specific tests.

`tests/language/`
Language-specific tests.

`tests/platform/`
Platform-specific tests (bootstrap + login/home/project flow).

`tests/rwe/`
RWE-specific tests.

## File Conventions

1. `*.zf.json`
Pipeline graph contract file.
Must be pin-based edges (`from_node`, `from_pin`, `to_node`, `to_pin`).

2. `*.tsx`
Template/reactive source file for RWE input.
Use TSX module shape:
```tsx
export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    links: [
      { rel: "canonical", href: "{{input.seo.canonical}}" }
    ],
    meta: [
      { property: "og:title", content: "{{input.seo.title}}" }
    ],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export const app = {
  state: { ... },
  actions: { ... },
  memo: { ... },
  effect: { ... }
};

export default function Page(input) {
  return (
    <Page>
      <main>...</main>
    </Page>
  );
}
```

## RWE Keyword Reference (Current)

This list reflects what the current `rwe.noop` engine actually supports.

TSX module contracts:

1. `export const page = { ... }` for page-level document metadata (`head.title`, `head.description`, `head.meta`, `head.links`, `head.scripts`, `html`, `body`, `navigation`)
2. `export const app = { ... }` for state/actions/memo/effect
3. `export default function Page(input) { return <Page>...</Page>; }` for page markup
4. `<markdown>...</markdown>` (processed when `markdown` processor is enabled)

Reactive attributes in TSX:

1. `onClick`, `onInput`, `onChange`, `onSubmit`
2. `jText="path.to.value"`
3. `jModel="path.to.value"`
4. `jAttrClass="path.to.value"`
5. `jShow="path.to.bool"`
6. `jHide="path.to.bool"`
7. `jFor="item in some.list.path"` (keyed list rendering)
8. `jKey="item.id"` (stable key selector for `jFor`)
9. `hydrate="off|interaction|visible|idle|immediate"` (hydration island mode)

SSR placeholders in TSX expressions:

1. `{input.path.to.value}`
2. `{ctx.route}`
3. `{ctx.requestId}`
4. `{ctx.metadata.someField}`

Component keywords:

1. PascalCase imported components, e.g. `<Button />`
2. compile option: `ReactiveWebOptions.templates.template_root`
3. compile option: `ReactiveWebOptions.components.strict`

Adapter protocol keywords:

1. `RWE_PROTOCOL_VERSION` (`rwe.v1`)
2. `CompileTemplateRequest` / `CompileTemplateResponse`
3. `RenderTemplateRequest` / `RenderTemplateResponse`
4. `ProtocolError`

Control-script return keys (runtime convention):

1. `state`
2. `actions`
3. `memo`
4. `effect`

Not implemented yet (planned in `src/rwe/MILESTONE.md`):

1. `.ts` helper-module import graph
2. directive-level branching (`j-if`, `j-else`)

## RWE Processor Features

RWE compile features are toggleable via `ReactiveWebOptions.processors`.

```rust
let options = ReactiveWebOptions {
    processors: vec!["tailwind".to_string(), "markdown".to_string()],
    ..Default::default()
};
```

Rules:

1. if `processors` is empty, legacy behavior applies (Tailwind follows `style_engine`)
2. if `processors` is non-empty, only listed processors run (in listed order)
3. current built-in processors: `tailwind`, `markdown`
4. `tailwind` injects a Tailwind-compatible preflight/reset base plus utility rules

Example templates:
- `conventions/templates/pages/blog-home.tsx`
- `conventions/templates/pages/blog-post.tsx`
- `conventions/templates/pages/blog-home-composed.tsx`
- `conventions/templates/pages/state-sharing-composed.tsx`
- `conventions/templates/pages/list-hydration.tsx`
- `conventions/templates/components/blog-header.tsx`
- `conventions/templates/components/blog-hero.tsx`
- `conventions/templates/components/tree-a.tsx`
- `conventions/templates/components/tree-b.tsx`
- `conventions/templates/components/tree-c.tsx`
- `conventions/templates/components/tree-d.tsx`
- `conventions/templates/components/tree-f.tsx`

## Current Status

1. Framework, language, and RWE are scaffolded with interface-first boundaries.
2. Pin-based pipeline sample exists at `conventions/pipelines/common_backend.zf.json`.
3. Deno sandbox language engine is wired and reusable.
4. Axum demo bootstrap is available via `axum_rwe_demo` binary.

## Axum Browser Demo

You can run a local Axum server that renders real TSX pages:

```bash
cargo run -p zebflow --bin axum_rwe_demo
```

Open in browser:

1. `http://127.0.0.1:8787/`
2. `http://127.0.0.1:8787/showcase`
3. `http://127.0.0.1:8787/recycling`
4. `http://127.0.0.1:8787/todo`
5. `http://127.0.0.1:8787/list-hydration`
6. `http://127.0.0.1:8787/state-sharing?seed=7`
7. `http://127.0.0.1:8787/blog`
8. `http://127.0.0.1:8787/blog/post-a`
9. `http://127.0.0.1:8787/blog/composed`
