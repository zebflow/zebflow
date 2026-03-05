# Zeb Libraries

## Goal

Define how Zebflow ships and manages official project libraries without coupling
the RWE core to a product-specific library catalog.

The initial scope is:

1. `zeb/codemirror`
2. `zeb/threejs`
3. `zeb/threejs-vrm`
4. `zeb/deckgl`
5. `zeb/d3`
6. `zeb/devicons`

Offline-first remains mandatory: Zeb Libraries are bundled with Zebflow or
shipped as local static assets and never fetched from a CDN at runtime.

## Boundary

Zeb Libraries are a **platform concern**, not an RWE-owned product subsystem.

The ownership split is:

1. `platform`
   - owns the Zeb Libraries catalog
   - owns version pinning
   - owns install/update/remove UX
   - owns project vendoring (`app/libraries/`, `app/libraries.lock.json`)
   - owns remote registry/cache policy

2. `rwe`
   - stays generic
   - consumes local modules/assets/runtime bundles
   - does not hardcode a product library list
   - does not become a package manager

This distinction matters. If RWE directly owns `zeb/codemirror`,
`zeb/threejs`, and future catalog policy, the engine becomes coupled to product
catalog decisions and slows down.

## Current RWE mechanics (relevant parts)

1. **Component imports**  
   PascalCase tags (e.g. `<ThreeScene />`) are resolved from the compile-time import graph under `ReactiveWebOptions.templates.template_root`. Imported `.tsx` components are lowered to HTML. Props are substituted as `{{props.key}}`; optional `hydrate="visible"|"idle"|"immediate"|"interaction"` wraps the expansion in a hydration island.

2. **Script allow-list**  
   `<script src="...">` in template markup is stripped unless the URL is
   allowed. RWE can inject trusted local assets and runtime bundles, but it
   should not own a product-specific library catalog.

3. **Runtime**  
   After HTML is delivered, the RWE runtime mounts the control script from `export const app` and discovers `[hydrate]` islands. There is no built-in "library init" hook; the only extension point is the page's control script (state/actions/memo/effect).

4. **Runtime bundle precedent**  
   The RWE runtime is injected as inline script via `RuntimeBundle { name,
   source }` at render time. Platform-managed Zeb Libraries can use the same
   generic hook without teaching RWE about specific library identities.

## Canonical contract

### 1. Namespace

Zeb Libraries are referenced through a Zebflow-owned namespace:

1. `zeb/codemirror`
2. `zeb/threejs`
3. `zeb/threejs-vrm`
4. `zeb/deckgl`
5. `zeb/d3`
6. `zeb/devicons`

These names are stable product contracts. The backing implementation may be:

1. inline bundled script
2. locally served static asset
3. thin Zebflow wrapper package over a shipped upstream asset

User code should never care which of those delivery methods is used.

### 2. Project ownership model

The canonical project shape should be:

```text
app/
  libraries/
    zeb/
      codemirror/
        0.1/
      threejs/
        0.3/
  libraries.lock.json
```

The project repo is the durable source of truth for library versions actually in
use. This avoids silent breakage when Zebflow itself is upgraded.

Resolution order should be:

1. `app/libraries/...`
2. local machine cache installed by Zebflow
3. remote Zeb Libraries registry
4. vendored copy written back into `app/libraries/...`

### 3. Delivery model

Zeb Libraries should be delivered in one of two engine-controlled ways:

1. inline injection at render time
2. local static path served by the host

The exact delivery path is generic RWE/platform plumbing. The library catalog,
version, and vendoring rules remain platform concerns.

The contract requirement is:

1. no runtime CDN dependency
2. no npm install step required by the Zebflow user
3. versions are controlled by Zebflow
4. offline use remains possible

### 3.1 Install, autocomplete, compile pipeline

To keep the system lightspeed and non-redundant, Zeb Libraries should follow a
three-tier pipeline:

1. install
   - platform fetches the selected package source/dist
   - platform normalizes it into project-owned library files
   - platform generates library metadata once
2. save
   - editor/compiler reuse generated metadata
   - no upstream re-ingest happens on every save
   - only project files are reparsed and recompiled
3. commit/publish
   - platform emits the final optimized combined page artifact
   - runtime stays separate from compiled page code

The critical rule is:

- expensive library intelligence is generated at install time
- save-time compile only consumes prepared metadata

That metadata should include:

1. `library.json`
   - identity, version, trust level, source, install provenance
2. `exports.json`
   - raw exports and wrapper exports
   - symbol -> runtime chunk/module mapping
3. `keywords.json`
   - autocomplete-optimized symbol list
4. `types.json` later
   - lightweight interface/type hints if available

This lets CodeMirror autocomplete stay JSON-driven instead of depending on a
browser LSP or repeated upstream parsing.

### 4. Usage pattern

The author-facing model should stay simple:

1. import a Zebflow-owned helper or wrapper from the `zeb/*` namespace
2. render a normal TSX component or call a normal helper
3. let platform/library resolution ensure the underlying library is present

Example directions:

```tsx
import { codemirror, CodeEditor } from "zeb/codemirror";
import { threejs, ThreeScene } from "zeb/threejs";
import { vrm, VrmViewer } from "zeb/threejs-vrm";
import { deckgl, DeckMap } from "zeb/deckgl";
import { d3, D3Chart } from "zeb/d3";
import { ensureDevicons } from "zeb/devicons";
```

The first implementations can be thin wrappers over container markup and init
contracts. They do not need to start as a deep component framework.

Every Zeb Library should expose two surfaces:

1. raw export surface
2. Zeb wrapper surface

That keeps advanced escape hatches and high-level productivity under one import
namespace.

### 5. Suggested first wrappers

The first useful wrappers are:

1. `CodeEditor`
   - main editor surface for the platform templates workspace
   - later reusable in user templates or script-node UI
2. `ThreeScene`
   - simple canvas/container ownership contract
3. `VrmViewer`
   - thin specialization on top of `ThreeScene`
4. `DeckMap`
   - map/canvas mount surface with config payload
5. `D3Chart`
   - svg/div mount surface with config payload

Example minimal wrapper shape:

```tsx
export default function ThreeScene(props) {
  return (
    <canvas
      data-zeb-lib="threejs"
      data-config="{{props.config}}"
      className="w-full h-full"
      hydrate="visible"
    />
  );
}
```

The wrapper stays simple. The platform/runtime decides how the backing library is mounted.

### 6. Codemirror is special

`zeb/codemirror` is not just another optional visualization library.
It is part of Zebflow's own product surface.

So it should be treated as:

1. a platform dependency
2. shippable with the installer
3. reusable by the build/templates workspace first
4. optionally exposed to user projects later

### 7. Packaging direction

Zebflow should be installable as one product package.

That package can include:

1. platform web templates/pages
2. RWE runtime JS
3. Zeb Libraries assets
4. fonts
5. default theme/preflight assets

This is compatible with a single installer as long as the binary or package knows how to:

1. expose static assets locally
2. embed or ship vendor JS/fonts
3. seed the default project/app structure

The right mental model is:

- Zebflow user installs one product
- platform and default assets come with it
- project templates and themes start from seeded local files
- Zeb Libraries are available under `zeb/*`
- projects pin concrete versions in `app/libraries.lock.json`

The ideal output contract for page delivery remains:

1. shared runtime script
2. one combined compiled page script

Internal library metadata, symbol graphs, or draft chunks may be more complex,
but the served result should stay simple.
