# Integrating blessed 3rd-party libraries (Three.js, D3) as RWE wrappers

## Goal

Allow RWE templates to use **Three.js** and **D3** as thin wrappers: author places a component (e.g. `<ThreeScene />`, `<D3Chart />`), RWE compiles to a known container shape, and a small client-side contract initializes the library on that container. No React; script-only usage.

**Offline-first:** Blessed lib scripts are **downloaded and bundled with the engine** (or cached locally at build/setup), so Zebflow runs fully **offline** on the client PC—no CDN at runtime.

## Current RWE mechanics (relevant parts)

1. **Component imports**  
   PascalCase tags (e.g. `<ThreeScene />`) are resolved from the compile-time import graph under `ReactiveWebOptions.templates.template_root`. Imported `.tsx` components are lowered to HTML. Props are substituted as `{{props.key}}`; optional `hydrate="visible"|"idle"|"immediate"|"interaction"` wraps the expansion in a hydration island.

2. **Script allow-list**  
   `<script src="...">` in template markup is **stripped** unless the URL is allowed (via `load_scripts` and optionally `allow_list.scripts`). For **blessed libs we do not rely on CDN**: scripts are bundled locally and injected by the engine (see below).

3. **Runtime**  
   After HTML is delivered, the RWE runtime (`rwe-runtime.js`) mounts the control script, binds `@click` / `j-text` / `j-model` etc., and discovers `[hydrate]` islands. There is no built-in “library init” hook; the only extension point is the page’s control script (state/actions/memo/effect).

4. **Runtime bundle precedent**  
   The RWE runtime is injected as **inline script** via `RuntimeBundle { name, source }` at render time. Blessed libs can follow the same idea: ship script **content** with the engine so no network is needed.

## Proposed pattern: blessed library wrappers

### 1. Convention components (wrapper markup only)

- **ThreeScene**  
  - Markup: a single container that Three.js will own. Prefer `<canvas>` for WebGL.  
  - Contract: `data-rwe-lib="three"`, optional `data-config="{{props.config}}"` (JSON string for scene/camera/renderer options).  
  - Example (in `.tsx`):

    ```html
    <canvas data-rwe-lib="three" data-config="{{props.config}}" class="w-full h-64"></canvas>
    ```

  - Usage in a page: `<ThreeScene config="{{input.sceneConfig}}" hydrate="visible" />`.  
  - The component does **not** contain any script; it only provides the DOM node and a config payload.

- **D3Chart**  
  - Markup: a container for D3 (typically SVG or a div).  
  - Contract: `data-rwe-lib="d3"`, optional `data-config="{{props.config}}"`.  
  - Example:

    ```html
    <div data-rwe-lib="d3" data-config="{{props.config}}" class="w-full h-64"></div>
    ```

  - Usage: `<D3Chart config="{{input.chartConfig}}" hydrate="visible" />`.

These components live in the template tree (for example `templates/components/three-scene.tsx` and `d3-chart.tsx`) and are imported by pages or other components.

### 2. Script loading: offline (bundle with engine)

Blessed lib scripts are **downloaded once** and **shipped/cached with the engine** so the client PC can run Zebflow offline.

**Acquisition (build/setup time):**

- Download the desired versions of Three.js and D3 (e.g. `three.min.js`, `d3.min.js`) from the official or CDN URLs.
- Store them in the engine’s asset tree, e.g.:
  - `crates/zebflow/vendor/three.min.js`, `crates/zebflow/vendor/d3.min.js`, or
  - a build script / optional crate that fetches them into a `vendor/` (or `assets/vendor/`) directory at build time.
- Do **not** rely on these URLs at runtime; they are only used to populate the local bundle.

**Delivery at render time (offline):**

Two equivalent options so the browser gets the script without network:

1. **Inline injection (recommended for full offline)**  
   At render time, for each blessed lib required by the page (e.g. inferred from used components or from options), inject a `<script data-rwe-vendor="three">...</script>` block whose content is the **local file content** (e.g. `include_str!("../vendor/three.min.js")` or from a runtime-loaded cache). Same pattern as the existing RWE runtime bundle: one inline script tag per blessed lib. No `src`; no request.

2. **Local path (host serves vendor files)**  
   If the host serves static files from a known prefix (e.g. `/assets/vendor/`), inject `<script src="/assets/vendor/three.min.js"></script>` and ensure those files are present on disk from the same build/setup step. Works offline as long as the client talks only to that host.

**RWE / host responsibilities:**

- Engine (or host) maintains a **vendor script store**: map lib id (e.g. `"three"`, `"d3"`) to script content or to a local path. Content can come from `include_str!` at compile time or from a file read at startup.
- When rendering a page that uses a blessed lib component, the renderer injects that lib’s script(s) before the RWE runtime (e.g. before `</body>`), in dependency order if needed (e.g. Three before app code). No CDN; no `load_scripts`/allow-list for these—they are first-class bundled assets.

### 3. Library init on the client

Two options; both keep Three/D3 logic out of the RWE core.

**Option A – Control script only (no RWE change)**  
In the page’s control script, use an **effect** that runs once (e.g. `once: true` and `immediate: true`) and:

- Queries all `[data-rwe-lib="three"]` (and optionally `[data-rwe-lib="d3"]`).
- Reads `data-config` (JSON), then calls a small init function that uses the global `THREE` / `d3` to create the scene or chart and attach it to the element.

Authors are responsible for defining that init (e.g. in the same control script or a shared snippet). RWE does not need to know about Three or D3.

**Option B – Optional runtime hook (small RWE extension)**  
Extend the RWE runtime so that after `R.mount` (and after hydration islands are set up), if `R.app.libs` exists and is an object, it runs:

- For each key `k` in `R.app.libs`, run `document.querySelectorAll('[data-rwe-lib="'+k+'"]').forEach(el => { const fn = R.app.libs[k]; if (typeof fn === 'function') fn(el, JSON.parse(el.getAttribute('data-config')||'{}')); });`

Then the page’s control script only needs to set e.g. `R.app.libs = { three: (el, config) => { /* Three.js init */ }, d3: (el, config) => { /* D3 init */ } }`. The runtime does not contain any Three/D3 code; it only invokes the provided functions. This avoids every page re-implementing the “find nodes and call init” logic.

**Recommendation:** Start with **Option A** (document the pattern; no runtime change). Add **Option B** if we want a single canonical hook and less boilerplate in every page.

### 4. Summary of touch points

| Area | Change |
|------|--------|
| **RWE core** | Optionally: (1) vendor script store (lib id → script content or path) and inject inline/local script at render when page uses a blessed lib; (2) optional “libs” init pass that calls `R.app.libs[k](el, config)` for each `[data-rwe-lib=k]`. |
| **Vendor assets** | Download Three.js and D3 (e.g. at build time) into `vendor/` (or equivalent); ship/cache with engine so client runs offline. |
| **Conventions** | Add two components: `ThreeScene`, `D3Chart` (markup only; container + `data-rwe-lib` + `data-config`). |
| **Platform / demo** | Import wrapper components from the template tree; ensure render path injects bundled vendor scripts (no CDN) when those components are used. |
| **Docs** | Document the “blessed lib” contract: `data-rwe-lib`, `data-config`, offline script bundle, and control-script init (effect or `R.app.libs`). |

This keeps the integration minimal, script-only (no React), and **offline-first**: no CDN at runtime; scripts live inside the engine (or host) and are injected at render so Zebflow works on a client PC without network.
