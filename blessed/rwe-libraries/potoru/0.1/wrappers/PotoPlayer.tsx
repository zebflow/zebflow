/**
 * PotoPlayer — RWE wrapper for Potoru .poto stories (zeb/potoru).
 *
 * IMPORT in a TSX page:
 *   import { PotoPlayer } from "zeb/potoru";
 *
 * ─── Basic usage ──────────────────────────────────────────────────────────
 *   <PotoPlayer src="/assets/stories/intro.poto" controls autoplay muted />
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
 *   still      boolean            show the frame at `time` and do not play
 *   time       number             seconds, with `still` (for many thumbnails, PotoSnapshot is lighter)
 *   voices     string             URL of the site's voice catalog (Piper voices a viewer can download)
 *   voiceFallback "browser" | "none"  who speaks a voice the viewer has not chosen (default: the browser's voice)
 *   inputs     object             host data { target: { field: value } } (story capability host-data)
 *   collections object            host lists { name: [{ id, ...fields }] }, each the whole list
 *   onReady, onReport, onError, onEnded, onPlay, onPause, onScene, …   event handlers (detail)
 *   id         string             container id for window.__zebPotoru.get(id)
 *   className  string             classes on the container
 *
 * ─── Events (on the container, bubbling) ──────────────────────────────────
 *   zeb:potoru:ready   { id, mode, duration, scenes, capabilities, name, slides }
 *   zeb:potoru:report  { id, from, event, payload, tick, scene }   (host bridge reports)
 *   zeb:potoru:ended   { id }
 *   zeb:potoru:error   { id, code, message }   code includes "needs-newer-player"
 *   zeb:potoru:voice-needed  { id, aliases, missing, fallback }   aliases with no chosen voice
 *   also play, pause, slide, scene, caption, camera, voices
 *
 * ─── Host data: the page drives the story ─────────────────────────────────
 *   const [people, setPeople] = useState([]);   // e.g. from a WebSocket
 *   <PotoPlayer id="town" src="/_files/town.poto" camera="play"
 *     collections={{ people }} inputs={{ $root: { camX: me.x, camY: me.y } }}
 *     onReport={(r) => r.event === "tap" && socket.send(JSON.stringify(r.payload))} />
 *   A collection is the whole list: new ids appear, missing ids go, the rest glide to new values.
 *   Every value is checked by the player (declared inputs only, types, sizes, clean text);
 *   refusals arrive as onError / zeb:potoru:error with code "host-input". Or imperatively:
 *   const town = usePotoPlayer("town"); town.apply({ collections: { people } });
 *
 * ─── Imperative registry ──────────────────────────────────────────────────
 *   const player = window.__zebPotoru.get("intro");
 *   player.play(); player.seek(2); player.set("hero", "speed", 2); player.fire("hero", "jump");
 *   player.openVoices();   // the Voices sheet: who speaks each voice, downloads, own keys
 *
 * ─── Voices ───────────────────────────────────────────────────────────────
 *   Lines without baked audio speak through the voice each viewer chooses (browser voice, a
 *   downloaded Piper voice in WebAssembly, ElevenLabs or an OpenAI-compatible endpoint with their
 *   own key). Choices, voices and keys stay in the viewer's browser; nothing reaches Zebflow.
 *
 * The server renders this placeholder; the zeb/potoru runtime mounts the player inside it in the
 * browser (in a shadow root, so re-renders never touch it). This file documents the markup only:
 * the browser runs `PotoPlayer` from runtime/entry.mjs (event props, inputs, collections), and the
 * server renders src/rwe/runtime/zeb_ssr_init.js's stub; neither reads this file.
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
  if (props.still) config.still = true;
  if (props.time !== undefined && props.time !== null && props.time !== "") config.time = props.time;
  if (props.mode) config.mode = props.mode;
  if (props.renderer) config.renderer = props.renderer;
  if (props.height) config.height = props.height;
  if (props.aspect) config.aspect = props.aspect;
  if (props.fit) config.fit = props.fit;
  if (props.camera) config.camera = props.camera;
  if (props.voices) config.voices = props.voices;
  if (props.voiceFallback) config.voiceFallback = props.voiceFallback;
  return config;
}

export default function PotoPlayer(props) {
  const config = playerConfig(props || {});
  const style = { width: "100%", display: "block", position: "relative" };
  if (config.height) style.height = typeof config.height === "number" ? `${config.height}px` : String(config.height);
  if (config.aspect) style.aspectRatio = String(config.aspect).replace(":", " / ");
  return (
    <div
      data-zeb-lib="potoru"
      data-zeb-wrapper="PotoPlayer"
      data-config={JSON.stringify(config)}
      id={props.id}
      className={props.className}
      style={style}
    />
  );
}
