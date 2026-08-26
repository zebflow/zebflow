# zeb/threejs-vrm

VRM 0.x / 1.0 avatar viewer for RWE templates.
Fully offline — Three.js + `@pixiv/three-vrm` bundled inline, no CDN.

## Import

```tsx
import VrmViewer from "zeb/threejs-vrm";
// or helpers:
import { mountVrmViewer, vrm } from "zeb/threejs-vrm";
```

---

## `VrmViewer` Component

```tsx
<VrmViewer
  id="my-avatar"
  modelUrl="/assets/avatar.vrm"
  height="500px"
  autoRotate
  background="transparent"
  cameraZ={1.5}
/>
```

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `modelUrl` | `string` | — | URL to `.vrm` file |
| `height` | `string` | `"400px"` | CSS height of the canvas container |
| `background` | `string` | `"transparent"` | Canvas background or CSS color |
| `autoRotate` | `boolean` | `false` | Auto-rotate the avatar |
| `cameraZ` | `number` | `1.5` | Camera Z distance |
| `ambientColor` | `string` | `"#ffffff"` | Ambient light colour |
| `ambientIntensity` | `number` | `0.6` | Ambient light intensity |
| `dirColor` | `string` | `"#ffffff"` | Directional light colour |
| `dirIntensity` | `number` | `0.8` | Directional light intensity |
| `id` | `string` | auto | Container id for `window.__zebVrm.get(id)` |
| `className` | `string` | — | Tailwind classes on container |

---

## Patterns

### Basic avatar display

```tsx
<VrmViewer
  modelUrl="/assets/characters/miku.vrm"
  height="600px"
  autoRotate
/>
```

### Dark background + custom lighting

```tsx
<VrmViewer
  modelUrl="/assets/avatar.vrm"
  height="480px"
  background="#0f172a"
  ambientColor="#94a3b8"
  ambientIntensity={0.4}
  dirColor="#e2e8f0"
  dirIntensity={1.2}
/>
```

### Post-mount via event

```tsx
<VrmViewer id="hero-vrm" modelUrl="/assets/hero.vrm" height="500px" />
```

```ts
// In behavior file:
document.getElementById("hero-vrm").addEventListener("zeb:vrm:ready", (e) => {
  const { instance } = e.detail;

  // Set happy expression
  instance.setExpression("happy", 0.8);

  // Make avatar look at a point
  instance.lookAt(new THREE.Vector3(0, 1.5, 2));
});
```

---

## Events

### `zeb:vrm:ready`

Fires once when the VRM model finishes loading and mounting.

```tsx
container.addEventListener("zeb:vrm:ready", (e) => {
  const { instance, vrm, id } = e.detail;
  // instance — full API (see below)
  // vrm      — raw @pixiv/three-vrm VRM object (may be null if model failed)
  // id       — container element id
});
```

---

## Imperative API — `window.__zebVrm`

```ts
const inst = window.__zebVrm.get("my-avatar");

inst.setExpression("happy", 1.0);       // set blendshape 0–1
inst.setExpression("sad",   0.5);
inst.lookAt(new THREE.Vector3(0, 2, 1)); // point look-at target
inst.playClip(animationClip);            // play AnimationClip
inst.stopClip();                         // stop current animation
inst.destroy();                          // finalize + unmount

// Raw Three.js objects:
inst.scene     // THREE.Scene
inst.camera    // THREE.PerspectiveCamera
inst.renderer  // THREE.WebGLRenderer
inst.mixer     // THREE.AnimationMixer
inst.vrm       // @pixiv/three-vrm VRM object
```

### Access Three.js namespace

```ts
const { THREE, GLTFLoader, VRMLoaderPlugin, VRMUtils } = window.__zebVrm;
// Or via the vrm export:
import { vrm } from "zeb/threejs-vrm";
vrm.THREE, vrm.VRMLoaderPlugin, vrm.VRMHumanBoneName, ...
```

---

## `mountVrmViewer` — direct mount

```tsx
import { mountVrmViewer } from "zeb/threejs-vrm";

const host = document.getElementById("vrm-container");
const instance = await mountVrmViewer(host, {
  modelUrl:   "/assets/avatar.vrm",
  height:     "400px",
  autoRotate: true,
  cameraZ:    2.0,
});

// Control after mount:
instance.setExpression("surprised", 0.9);
instance.destroy();
```

---

## Expression presets (VRM 1.0)

Common expression names: `happy`, `sad`, `angry`, `surprised`, `relaxed`, `neutral`,
`aa`, `ih`, `ou`, `ee`, `oh`, `blink`, `blinkLeft`, `blinkRight`,
`lookUp`, `lookDown`, `lookLeft`, `lookRight`.

```ts
inst.setExpression("happy",     1.0);
inst.setExpression("blink",     1.0);
inst.setExpression("lookRight", 0.5);
```

---

## Bundle details

| Property | Value |
|----------|-------|
| Packages | `three` r0.171 + `@pixiv/three-vrm` 3.x |
| Bundle | `runtime/threejs-vrm.bundle.mjs` (~862 KB minified) |
| CDN fetches | **None** — fully offline |
| VRM versions | VRM 0.x and VRM 1.0 |
| Build tool | esbuild |

### Rebuild

```sh
cd /tmp/zeb-threejs-vrm-build
cp libraries/zeb/threejs-vrm/0.1/runtime/entry.mjs .
cp libraries/zeb/threejs-vrm/0.1/runtime/package.json .
npm install
node_modules/.bin/esbuild entry.mjs --bundle --format=esm --minify \
  --outfile=threejs-vrm.bundle.mjs
cp threejs-vrm.bundle.mjs libraries/zeb/threejs-vrm/0.1/runtime/
cargo build
```
