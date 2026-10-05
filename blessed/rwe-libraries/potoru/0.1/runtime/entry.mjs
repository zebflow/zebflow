/**
 * zeb/potoru 0.1 — Potoru 2D animation for RWE templates: a player, still snapshots, an editor and
 * an in-browser compiler.
 *
 * ── WHAT THIS FILE IS ───────────────────────────────────────────────────────
 *  The readable source of the entry bundle (`potoru.bundle.mjs`). It holds only the Zebflow
 *  bridge: the three client components, the registries and the MutationObserver that mounts
 *  server-rendered placeholders. Everything heavy loads on demand, from files next to the bundle:
 *   • the player (`src/player.mjs` → `player-*.mjs` chunks, wasm engines, `fonts/`) when the first
 *     PotoPlayer mounts or `mountPotoPlayer()` is called;
 *   • the snapshot (`src/snapshot.mjs`: decode + evaluate one frame + Canvas renderer; never the
 *     action or score engines, no element, no clock) when the first PotoSnapshot mounts;
 *   • the compiler and the editor (`src/compiler.mjs` → `compiler-*.mjs` chunks; Script mode also
 *     `potoru-authoring-sandbox.js`) only on the first `compile()` call or when a PotoEditor mounts.
 *  A page that uses only the player never fetches a compiler file.
 *
 * ── QUICK REFERENCE ─────────────────────────────────────────────────────────
 *  TSX import:   import { PotoPlayer, PotoSnapshot, PotoEditor, compile } from "zeb/potoru";
 *  Player:       <PotoPlayer id="intro" src="/assets/intro.poto" controls autoplay muted />
 *  Still:        <PotoSnapshot src="/assets/intro.poto" time={1.5} aspect="16 / 9" />
 *  Imperative:   window.__zebPotoru.get("intro").play()
 *  Events:       el.addEventListener("zeb:potoru:ready", (e) => e.detail.duration)
 *  Compile:      const result = await compile({ yaml: filesMap });   // { ok, poto, diagnostics, sizes }
 *  Editor:       <PotoEditor id="lab" yaml={filesMap} />  →  window.__zebPotoEditor.get("lab")
 *
 * ── BUILD ───────────────────────────────────────────────────────────────────
 *  POTORU_SRC=<potoru checkout>/designer/e3 node build/build.mjs   (see NOTES.md and manifest note)
 */

export const VERSION = "0.1.1";

const PLAYER_LIB = "potoru";
const PLAYER_WRAPPER = "PotoPlayer";
const EDITOR_WRAPPER = "PotoEditor";
const SNAPSHOT_WRAPPER = "PotoSnapshot";
const SELECTOR = `[data-zeb-lib="${PLAYER_LIB}"]`;

const players = new Map();
const editors = new Map();
let counter = 0;

/* ── Lazy parts ──────────────────────────────────────────────────────────── */

let playerModule;
let compilerModule;
let snapshotModule;
/** The snapshot chunk (one frame on a canvas; no engines, no custom element); loaded once. */
function loadSnapshot() {
  snapshotModule ??= import("./src/snapshot.mjs").catch((error) => { snapshotModule = undefined; throw error; });
  return snapshotModule;
}
/** The player chunk (registers `<potoru-player>`); loaded once. */
export function loadPlayer() {
  playerModule ??= import("./src/player.mjs").catch((error) => { playerModule = undefined; throw error; });
  return playerModule;
}
/** The compiler chunk (format compiler, authoring sandbox runner, widget); loaded once. */
function loadCompiler() {
  compilerModule ??= import("./src/compiler.mjs").catch((error) => { compilerModule = undefined; throw error; });
  return compilerModule;
}

/* ── Helpers ─────────────────────────────────────────────────────────────── */

function readConfig(host) {
  try { return JSON.parse(host.getAttribute("data-config") || "{}") || {}; } catch { return {}; }
}

function ensureId(host, prefix) {
  if (!host.id) host.id = `${prefix}-${(counter += 1).toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
  return host.id;
}

function emit(host, type, detail) {
  host.dispatchEvent(new CustomEvent(type, { detail, bubbles: true, composed: true }));
}

function placeholderStyle(config) {
  const style = { width: "100%", display: "block", position: "relative" };
  if (config.height) style.height = typeof config.height === "number" ? `${config.height}px` : String(config.height);
  if (config.aspect) style.aspectRatio = String(config.aspect).replace(":", " / ");
  return style;
}

function playerConfig(props = {}) {
  const libraries = Array.isArray(props.libraries) ? props.libraries : String(props.libraries || "").split(/[\s,]+/).filter(Boolean);
  const config = { src: props.src || "" };
  if (libraries.length) config.libraries = libraries;
  for (const key of ["controls", "autoplay", "muted", "loop", "captions", "still"]) if (props[key]) config[key] = true;
  for (const key of ["time", "mode", "renderer", "height", "aspect", "fit", "camera"]) if (props[key] !== undefined && props[key] !== null && props[key] !== "") config[key] = props[key];
  return config;
}

function snapshotConfig(props = {}) {
  const libraries = Array.isArray(props.libraries) ? props.libraries : String(props.libraries || "").split(/[\s,]+/).filter(Boolean);
  const config = { src: props.src || "" };
  for (const key of ["time", "camera", "fit", "height", "aspect"]) if (props[key] !== undefined && props[key] !== null && props[key] !== "") config[key] = props[key];
  if (libraries.length) config.libraries = libraries;
  return config;
}

function editorConfig(props = {}) {
  const config = { mode: props.mode === "script" ? "script" : "yaml" };
  if (props.yaml) config.yaml = props.yaml;
  if (props.script) config.script = props.script;
  if (props.height) config.height = props.height;
  return config;
}

/* ── Player ──────────────────────────────────────────────────────────────── */

/**
 * Mounts a Potoru player into `host` (any element). Options: src, libraries (string[] or a
 * space-separated string), controls, autoplay, muted, loop, captions, mode (play | slide |
 * interactive), renderer (canvas | canvas-exact | svg), height, aspect ("16/9"), fit, camera, id,
 * still + time (show the frame at `time` seconds and do not play; PotoSnapshot is lighter).
 * Resolves to the instance once the player code is loaded (before the story has loaded; wait for
 * `zeb:potoru:ready` on the host for duration, scenes and capabilities).
 */
export async function mountPotoPlayer(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/potoru: mountPotoPlayer needs a host element");
  if (options.id && !host.id) host.id = options.id;
  const id = ensureId(host, "potoru");
  const existing = players.get(id);
  if (existing && existing.host === host) return existing;
  const config = playerConfig(options);
  for (const [key, value] of Object.entries(placeholderStyle(config))) if (!host.style[key]) host.style[key] = value;
  const module = await loadPlayer();
  const instance = module.createPlayer(host, config, { id, emit: (type, detail) => emit(host, `zeb:potoru:${type}`, detail) });
  instance.destroy = ((destroy) => () => { destroy(); if (players.get(id) === instance) players.delete(id); })(instance.destroy);
  players.set(id, instance);
  return instance;
}

/* ── Snapshot ────────────────────────────────────────────────────────────── */

/**
 * Draws one moment of a story into `host` (a still: thumbnails, galleries, storyboards). Options:
 * src, time (seconds, default 0), libraries, camera, fit, height, aspect, id. No controls, no
 * sound, no interaction engine, no playback loop; it draws again when src or time change.
 * Registered in `window.__zebPotoru` like players (`kind: "snapshot"`).
 */
export async function mountPotoSnapshot(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/potoru: mountPotoSnapshot needs a host element");
  if (options.id && !host.id) host.id = options.id;
  const id = ensureId(host, "potoru-snapshot");
  const existing = players.get(id);
  if (existing && existing.host === host) return existing;
  const config = snapshotConfig(options);
  for (const [key, value] of Object.entries(placeholderStyle(config))) if (!host.style[key]) host.style[key] = value;
  const module = await loadSnapshot();
  const instance = module.createSnapshot(host, config, { id, emit: (type, detail) => emit(host, `zeb:potoru:${type}`, detail) });
  instance.destroy = ((destroy) => () => { destroy(); if (players.get(id) === instance) players.delete(id); })(instance.destroy);
  players.set(id, instance);
  return instance;
}

/* ── Compiler ────────────────────────────────────────────────────────────── */

/**
 * Compiles Potoru source to a `.poto` package, in the browser.
 *  • YAML: `{ yaml }` — a format v4 source folder as a map path → text (or Uint8Array for binaries).
 *  • Script (experimental): `{ script, yaml? }` — JavaScript using the authoring API, run in a
 *    sandboxed iframe (no network, opaque origin, hard timeout); it returns a Project.
 * Options: `libraries` (Uint8Array[] of .potolib / library .poto), `scene`, `state`, `locale`,
 * `profile` ("presentation": every root State becomes a slide), `timeoutMs` (Script, default 10000).
 * Resolves to `{ ok, poto?, diagnostics, files?, sizes }`; never rejects for bad source.
 */
export async function compile(input = {}) {
  const module = await loadCompiler();
  return module.compile(input);
}

/** Mounts the editor (YAML / Script toggle, file list, editor, diagnostics, live preview) into `host`. */
export async function mountPotoEditor(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/potoru: mountPotoEditor needs a host element");
  if (options.id && !host.id) host.id = options.id;
  const id = ensureId(host, "potoru-editor");
  const existing = editors.get(id);
  if (existing && existing.host === host) return existing;
  for (const [key, value] of Object.entries(placeholderStyle({ height: options.height || "560px" }))) if (!host.style[key]) host.style[key] = value;
  const module = await loadCompiler();
  const widget = await module.createEditor(host, editorConfig(options), {
    id,
    emit: (type, detail) => emit(host, `zeb:potoru-editor:${type}`, detail)
  });
  widget.destroy = ((destroy) => () => { destroy(); if (editors.get(id) === widget) editors.delete(id); })(widget.destroy);
  editors.set(id, widget);
  return widget;
}

/* ── Auto-mount of server-rendered placeholders ──────────────────────────── */

function mountNode(host) {
  if (host.__zebPotoru) return;
  const wrapper = host.getAttribute("data-zeb-wrapper") || PLAYER_WRAPPER;
  const config = readConfig(host);
  host.__zebPotoru = wrapper;
  const mount = wrapper === EDITOR_WRAPPER ? mountPotoEditor : wrapper === SNAPSHOT_WRAPPER ? mountPotoSnapshot : mountPotoPlayer;
  const event = wrapper === EDITOR_WRAPPER ? "zeb:potoru-editor:error" : "zeb:potoru:error";
  mount(host, config).catch((error) => {
    host.__zebPotoru = undefined;
    console.error("zeb/potoru: mount failed:", error);
    emit(host, event, { id: host.id, code: "mount", message: error instanceof Error ? error.message : String(error) });
  });
}

function unmountNode(node) {
  if (node.nodeType !== 1) return;
  const nodes = [...(node.matches?.(SELECTOR) ? [node] : []), ...(node.querySelectorAll?.(SELECTOR) ?? [])];
  for (const host of nodes) {
    if (!host.__zebPotoru || host.isConnected) continue;
    const registry = host.__zebPotoru === EDITOR_WRAPPER ? editors : players;
    const instance = registry.get(host.id);
    if (instance?.host === host) instance.destroy();
    host.__zebPotoru = undefined;
  }
}

function configChanged(host) {
  if (!host.__zebPotoru) return;
  const registry = host.__zebPotoru === EDITOR_WRAPPER ? editors : players;
  const instance = registry.get(host.id);
  if (instance?.host === host && typeof instance.configure === "function") instance.configure(readConfig(host));
}

function scan(root) {
  if (root.matches?.(SELECTOR)) mountNode(root);
  root.querySelectorAll?.(SELECTOR).forEach(mountNode);
}

if (typeof document !== "undefined" && typeof MutationObserver !== "undefined") {
  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === "attributes") { if (mutation.target.matches?.(SELECTOR)) configChanged(mutation.target); continue; }
      for (const node of mutation.removedNodes) unmountNode(node);
      for (const node of mutation.addedNodes) if (node.nodeType === 1) scan(node);
    }
  });
  const start = () => {
    observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-config"] });
    scan(document.body);
  };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start, { once: true });
  else start();
}

/* ── Client components (the SSR placeholders render the same markup) ────── */

/**
 * PotoPlayer — props: src, libraries, controls, autoplay, muted, loop, captions, mode, renderer,
 * height, aspect, fit, camera, still + time (show that frame, do not play), id, className. Renders the placeholder; the runtime mounts the
 * player inside it (in a shadow root, so re-renders never touch it).
 */
export function PotoPlayer(props = {}) {
  const h = globalThis.h;
  if (!h) return null;
  const config = playerConfig(props);
  return h("div", { "data-zeb-lib": PLAYER_LIB, "data-zeb-wrapper": PLAYER_WRAPPER, "data-config": JSON.stringify(config), id: props.id, className: props.className, style: placeholderStyle(config) });
}

/**
 * PotoSnapshot — props: src, time, libraries, camera, fit, height, aspect, id, className. One
 * still frame; much lighter than a PotoPlayer for many thumbnails.
 */
export function PotoSnapshot(props = {}) {
  const h = globalThis.h;
  if (!h) return null;
  const config = snapshotConfig(props);
  return h("div", { "data-zeb-lib": PLAYER_LIB, "data-zeb-wrapper": SNAPSHOT_WRAPPER, "data-config": JSON.stringify(config), id: props.id, className: props.className, style: placeholderStyle(config) });
}

/** PotoEditor — props: yaml (a source folder files map), script, mode ("yaml" | "script"), height, id, className. */
export function PotoEditor(props = {}) {
  const h = globalThis.h;
  if (!h) return null;
  const config = editorConfig(props);
  return h("div", { "data-zeb-lib": PLAYER_LIB, "data-zeb-wrapper": EDITOR_WRAPPER, "data-config": JSON.stringify(config), id: props.id, className: props.className, style: placeholderStyle({ height: config.height || "560px" }) });
}

/* ── Registries and namespace ────────────────────────────────────────────── */

export const potoru = {
  version: VERSION,
  /** A mounted player or snapshot by container id. */
  get: (id) => players.get(id),
  /** A mounted editor by container id. */
  getEditor: (id) => editors.get(id),
  mountPotoPlayer,
  mountPotoSnapshot,
  mountPotoEditor,
  compile,
  loadPlayer
};

if (typeof window !== "undefined") {
  window.__zebPotoru = { version: VERSION, get: (id) => players.get(id), ids: () => [...players.keys()], mountPotoPlayer, mountPotoSnapshot, compile };
  window.__zebPotoEditor = { version: VERSION, get: (id) => editors.get(id), ids: () => [...editors.keys()], mountPotoEditor, compile };
}
