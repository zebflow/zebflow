/**
 * PotoruPlayer — RWE wrapper for Potoru .poto stories (zeb/potoru).
 *
 * IMPORT in a TSX page:
 *   import { PotoruPlayer } from "zeb/potoru";
 *
 * ─── Basic usage ──────────────────────────────────────────────────────────
 *   <PotoruPlayer src="/assets/stories/intro.poto" controls autoplay muted />
 *
 * ─── Props ────────────────────────────────────────────────────────────────
 *   src        string             URL of a .poto package (same origin or CORS)
 *   libraries  string[] | string  linked library URLs (.potolib), or space-separated
 *   controls   boolean            show the control bar
 *   autoplay   boolean            start playing (play / slide modes)
 *   muted      boolean            start muted
 *   loop       boolean            loop (play mode)
 *   captions   boolean            show captions
 *   mode       "play" | "slide" | "interactive"   force a mode the story supports
 *   renderer   "canvas" | "canvas-exact" | "svg"  default canvas
 *   height     string | number    CSS height (otherwise the story's aspect ratio)
 *   aspect     string             CSS aspect ratio of the box, e.g. "16 / 9"
 *   id         string             container id for window.__zebPotoru.get(id)
 *   className  string             classes on the container
 *
 * ─── Events (on the container, bubbling) ──────────────────────────────────
 *   zeb:potoru:ready   { id, mode, duration, scenes, capabilities, name, slides }
 *   zeb:potoru:report  { id, from, event, payload, tick, scene }   (host bridge reports)
 *   zeb:potoru:ended   { id }
 *   zeb:potoru:error   { id, code, message }   code includes "needs-newer-player"
 *   also play, pause, slide, scene, caption, camera
 *
 * ─── Imperative registry ──────────────────────────────────────────────────
 *   const player = window.__zebPotoru.get("intro");
 *   player.play(); player.seek(2); player.set("hero", "speed", 2); player.fire("hero", "jump");
 *
 * The server renders this placeholder; the zeb/potoru runtime mounts the player inside it in the
 * browser (in a shadow root, so re-renders never touch it).
 */
export const app = {};

function playerConfig(props) {
  const libraries = Array.isArray(props.libraries)
    ? props.libraries
    : String(props.libraries || "").split(/[\s,]+/).filter(Boolean);
  const config = { src: props.src || "" };
  if (libraries.length) config.libraries = libraries;
  if (props.controls) config.controls = true;
  if (props.autoplay) config.autoplay = true;
  if (props.muted) config.muted = true;
  if (props.loop) config.loop = true;
  if (props.captions) config.captions = true;
  if (props.mode) config.mode = props.mode;
  if (props.renderer) config.renderer = props.renderer;
  if (props.height) config.height = props.height;
  if (props.aspect) config.aspect = props.aspect;
  if (props.fit) config.fit = props.fit;
  if (props.camera) config.camera = props.camera;
  return config;
}

export default function PotoruPlayer(props) {
  const config = playerConfig(props || {});
  const style = { width: "100%", display: "block", position: "relative" };
  if (config.height) style.height = typeof config.height === "number" ? `${config.height}px` : String(config.height);
  if (config.aspect) style.aspectRatio = String(config.aspect).replace(":", " / ");
  return (
    <div
      data-zeb-lib="potoru"
      data-zeb-wrapper="PotoruPlayer"
      data-config={JSON.stringify(config)}
      id={props.id}
      className={props.className}
      style={style}
    />
  );
}
