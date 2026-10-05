/**
 * PotoSnapshot — RWE wrapper for a still picture of a Potoru story at one moment (zeb/potoru).
 *
 * IMPORT in a TSX page:
 *   import { PotoSnapshot } from "zeb/potoru";
 *
 * ─── Basic usage ──────────────────────────────────────────────────────────
 *   <PotoSnapshot src="/assets/stories/intro.poto" time={1.5} aspect="16 / 9" />
 *
 * For thumbnails, galleries, storyboards and scrubbers: no controls, no sound, no interaction
 * engine and no playback loop. It draws once, and again when `src` or `time` change. Much lighter
 * than a PotoPlayer: it never loads the action or score engines.
 *
 * ─── Props ────────────────────────────────────────────────────────────────
 *   src        string             URL of a .poto package
 *   time       number             seconds into the story's first timeline (default 0)
 *   libraries  string[] | string  linked library URLs (.potolib), or space-separated
 *   camera     string             a story camera id, or "stage"
 *   fit        "contain" | "cover" | "fill"
 *   height     string | number    CSS height (otherwise the story's aspect ratio)
 *   aspect     string             CSS aspect ratio of the box, e.g. "16 / 9"
 *   id         string             container id for window.__zebPotoru.get(id)
 *   className  string             classes on the container
 *
 * ─── Events (on the container, bubbling) ──────────────────────────────────
 *   zeb:potoru:ready   { id, kind: "snapshot", time, duration, name, width, height }  (after each load)
 *   zeb:potoru:error   { id, code, message }
 *
 * ─── Imperative registry (shared with players; kind: "snapshot") ──────────
 *   window.__zebPotoru.get("thumb").seek(2.5);   // redraw at another time
 */
export const app = {};

export default function PotoSnapshot(props) {
  const p = props || {};
  const libraries = Array.isArray(p.libraries) ? p.libraries : String(p.libraries || "").split(/[\s,]+/).filter(Boolean);
  const config = { src: p.src || "" };
  if (p.time !== undefined && p.time !== null && p.time !== "") config.time = p.time;
  if (p.camera) config.camera = p.camera;
  if (p.fit) config.fit = p.fit;
  if (p.height) config.height = p.height;
  if (p.aspect) config.aspect = p.aspect;
  if (libraries.length) config.libraries = libraries;
  const style = { width: "100%", display: "block", position: "relative" };
  if (config.height) style.height = typeof config.height === "number" ? `${config.height}px` : String(config.height);
  if (config.aspect) style.aspectRatio = String(config.aspect).replace(":", " / ");
  return (
    <div
      data-zeb-lib="potoru"
      data-zeb-wrapper="PotoSnapshot"
      data-config={JSON.stringify(config)}
      id={p.id}
      className={p.className}
      style={style}
    />
  );
}
