---
name: procedural-assets
description: Building 2D and 3D assets in code, iteratively — an SVG icon, illustration or diagram; a Three.js mesh, prop, scene or character (zeb/threejs, or any engine); from a reference image or a brief. Use whenever you are about to draw or model something rather than download it — covers observe-then-spec, passes, render-and-compare, one decision per pass, budgets, and when to stop.
license: MIT
metadata:
  version: "1"
  derived_from: "method inspired by github.com/img2threejs/img2threejs (Apache-2.0); text written from scratch"
---

# Procedural assets: sculpt, don't one-shot

An asset built in code — an SVG, a Three.js mesh, a scene — is never right on
the first attempt, and the failure mode is always the same: one big emission,
judged by how it *feels*, "improved" reported as "done". The cure is the one
sculptors use: observe, decide the parts, build in passes, look at each pass
against the reference, change one thing, repeat within a budget.

## 0. Decide what you are making

| Asset | Build in | Render check |
|---|---|---|
| icon, logo mark, illustration, chart glyph, diagram | inline `<svg>` in TSX (server-rendered, themeable) or a `.svg` file under `static/` | the page, at 1× and 3× zoom |
| a 3D prop, product, character, scene | `zeb/threejs` (Three.js r183) — a factory function that returns a `Group` | screenshots from ≥ 3 azimuths |
| another engine (Godot, Unity, Babylon, R3F) | the same passes and gates; only the primitives' names change | that engine's viewport |

State the fidelity you are aiming for **before** starting — `stylized`, `low-poly`,
`likeness`, `hero` — and the budget: SVG path count / kilobytes, or a triangle
count (≤ 6k a prop, ≤ 60k a hero, more only with a reason). A single reference
view cannot show hidden sides; say what you will infer.

## 1. Observe before you infer

Look at the reference (or write the brief as if it were one) and write down,
in 3D or 2D geometric vocabulary, not adjectives:

1. **Identity-defining features** — the 3–5 things that make it *this* object
   (a hook-curved beak, the asymmetric collar, the gap between the letters).
   These are what every pass is judged on; a global "looks similar" that
   misses one is a failed pass.
2. **Decomposition**, macro → meso → micro: silhouette and volumes; parts and
   their relationships (what attaches where, pivots, symmetry); surface
   detail (bevels, seams, fasteners, strokes).
3. **Materials / fills** in engine terms: PBR (colour, roughness, metalness,
   emissive) or SVG (fill, stroke width, joins, gradients) — never "shiny".
4. **What the view hides** and how you will handle it (mirror, infer, ask).

Write this as the asset's spec — `docs/assets/<name>.md` in a project — with
the component list, each component's primitive strategy (box, cylinder, lathe,
extruded `Shape`, curve + tube, path in SVG), its pivot, and the pass in which
it appears. A shallow spec (one root, no parts, no local details) is not ready
for code; do not "fix it in the geometry".

## 2. Build in locked passes

Only touch the current pass; the previous ones are frozen once reviewed.

| Pass | Goal | Done when |
|---|---|---|
| **blockout** | volumes at the right proportions and positions, flat material | the silhouette matches from the review angles |
| **structure** | parts as their own nodes with correct pivots, symmetry by mirroring | every identity feature has a node; left/right are mirrors, not rotations |
| **form** | curves, bevels, tapers, thickness; the meso detail | the identity features read from 3 m / at 24 px |
| **material** | colour, roughness, metalness, emissive; strokes and fills | materials read as the named ones in a neutral light |
| **lighting / composition** | key, fill, rim; camera; background | one hero view and it holds off-axis |
| **interaction / animation** | pivots animate, parts explode, hover states | the rig moves without tearing |
| **optimization** | merge, instance, decimate, simplify paths | inside the budget with the identity features intact |

Rules that hold in every pass:

- **Reflect, never rotate, for pairs**: a left part is the right part with one
  axis negated (and its winding flipped in 3D).
- **Prefer primitives and math over art**: box / cylinder / sphere / lathe /
  extruded `Shape` / `TubeGeometry` along a curve / instancing / displacement;
  generated canvas textures before image files. Seeds are deterministic.
- **Keep the decisions out of the generated code**: the spec is the record;
  the factory (`scripts/assets/<name>.ts` → `createXModel(options)` returning a
  `Group`, or `components/icons/<name>.tsx` for SVG) is a rendering of it.
- **Name the parts** (`group.name`, `data-part` on SVG groups) — a model whose
  parts you can click, hide and explode is a model you can review.

## 3. Render and compare — every pass

1. Render the current pass where it will live (the page, through the
   `zebflow-verify` probe) and screenshot it: SVG at 1× and 3×; 3D from
   front / 90° / 180° / 270° plus the hero angle. One frame is not evidence.
2. Put reference and render side by side and judge **the identity features
   one by one**, then the whole. Silhouette agreement is cheap and blind — a
   face deleted inside the silhouette scores the same as a finished one.
   Look *inside*: proportions between parts, curvature (a straight cone and a
   hooked one share an outline), small features at the size they will be
   seen (a 3-px detail vanishes in a downscaled comparison; zoom).
3. Write down, with numbers: what changed this pass (values, coordinates),
   what matches now, what still does not.

## 4. One decision per pass

`continue` · `refine-spec` (the plan was wrong or shallow — fix the spec, then
regenerate; never patch code around a wrong plan) · `refine-code` (the plan
is right, the geometry/material is not) · `request-input` (another view, a
cleaner reference, an accepted stylization) · `stop` (this fidelity is not
reachable from this input — a valid result).

Bound it: at most 3 corrections per pass and 6 in total; oscillation between
two states, or no measurable change over two rounds, is a `stop` or a
`request-input`, not a seventh try. Keep the count in the spec file, not in
your memory.

## 5. In a Zebflow project

**SVG** — server-rendered, themeable, no library:

```tsx
export function LeafMark({ className }) {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <g data-part="blade"><path d="M4 20c6-9 11-13 16-16-1 7-4 12-11 15" /></g>
      <g data-part="vein"><path d="M4 20 15 9" /></g>
    </svg>
  );
}
```

`currentColor` and theme classes (`text-primary`, `fill-chart-2`) keep it on
the theme; `viewBox` keeps it crisp at any size; a group per part keeps it
reviewable. Files under `static/` are served at `/static/{owner}/{project}/…`.

**Three.js** — a factory, mounted in the browser:

```tsx
import { useEffect, useRef } from "zeb/react";
import { mountThreeScene } from "zeb/threejs";
import { createLampModel } from "@/scripts/assets/lamp";

export default function LampView({ className }) {
  const host = useRef(null);
  useEffect(() => {
    const rt = mountThreeScene(host.current, { background: "#0b1020", cameraZ: 4 });
    rt.scene.remove(rt.cube);                       // the runtime's demo cube
    rt.scene.add(createLampModel(rt.THREE, { seed: 7 }));
    return () => rt.destroy();
  }, []);
  return <div ref={host} className={className} />;
}
```

`mountThreeScene` returns `{ THREE, scene, camera, renderer, cube, resize, destroy }`;
build with `THREE` from the runtime, in `useEffect` only — the server has no
WebGL. The factory takes `THREE` and options, returns a named `Group`, and
lives in `scripts/assets/`. Screenshots for the compare step come from the
probe in `zebflow-verify` (`page.screenshot`), rotating the camera between
shots.

## 6. Report honestly

Each pass: what changed, with values; what matches; what does not; the
decision. "Approximate", "stylized", "hidden side inferred" are statements to
make, not to hide. A passing side-by-side is not proof of realism; a model
that reached the budget with an identity feature missing is not done.
