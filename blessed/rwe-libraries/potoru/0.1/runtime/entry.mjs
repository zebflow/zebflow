/**
 * zeb/potoru 0.1 — Potoru 2D animation for RWE templates: a player and an in-browser compiler.
 *
 * ── WHAT THIS FILE IS ───────────────────────────────────────────────────────
 *  The readable source of the entry bundle (`potoru.bundle.mjs`). It holds only the Zebflow
 *  bridge: the two client components, the registries and the MutationObserver that mounts
 *  server-rendered placeholders. Everything heavy loads on demand, from files next to the bundle:
 *   • the player (`src/player.mjs` → `player-*.mjs` chunks, wasm engines, `fonts/`) when the first
 *     PotoruPlayer mounts or `mountPotoruPlayer()` is called;
 *   • the compiler (`src/compiler.mjs` → `compiler-*.mjs` chunks, `potoru-authoring-sandbox.js`)
 *     only on the first `compile()` call or when a PotoruCompiler mounts.
 *  A page that uses only the player never fetches a compiler file.
 *
 * ── QUICK REFERENCE ─────────────────────────────────────────────────────────
 *  TSX import:   import { PotoruPlayer, PotoruCompiler } from "zeb/potoru";
 *  Player:       <PotoruPlayer id="intro" src="/assets/intro.poto" controls autoplay muted />
 *  Imperative:   window.__zebPotoru.get("intro").play()
 *  Events:       el.addEventListener("zeb:potoru:ready", (e) => e.detail.duration)
 *  Compiler:     const result = await potoru.compile({ files });   // { ok, poto, diagnostics, sizes }
 *  Widget:       <PotoruCompiler id="lab" files={{ ... }} />  →  window.__zebPotoruCompiler.get("lab")
 *
 * ── BUILD ───────────────────────────────────────────────────────────────────
 *  POTORU_SRC=<potoru checkout>/designer/e3 node build/build.mjs   (see NOTES.md and manifest note)
 */

export const VERSION = "0.1.0";

const PLAYER_LIB = "potoru";
const PLAYER_WRAPPER = "PotoruPlayer";
const COMPILER_WRAPPER = "PotoruCompiler";
const SELECTOR = `[data-zeb-lib="${PLAYER_LIB}"]`;

const players = new Map();
const compilers = new Map();
let counter = 0;

/* ── Lazy parts ──────────────────────────────────────────────────────────── */

let playerModule;
let compilerModule;
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
  for (const key of ["controls", "autoplay", "muted", "loop", "captions"]) if (props[key]) config[key] = true;
  for (const key of ["mode", "renderer", "height", "aspect", "fit", "camera"]) if (props[key] !== undefined && props[key] !== null && props[key] !== "") config[key] = props[key];
  return config;
}

function compilerConfig(props = {}) {
  const config = { mode: props.mode === "script" ? "script" : "files" };
  if (props.files) config.files = props.files;
  if (props.script) config.script = props.script;
  if (props.height) config.height = props.height;
  return config;
}

/* ── Player ──────────────────────────────────────────────────────────────── */

/**
 * Mounts a Potoru player into `host` (any element). Options: src, libraries (string[] or a
 * space-separated string), controls, autoplay, muted, loop, captions, mode (play | slide |
 * interactive), renderer (canvas | canvas-exact | svg), height, aspect ("16/9"), fit, camera, id.
 * Resolves to the instance once the player code is loaded (before the story has loaded; wait for
 * `zeb:potoru:ready` on the host for duration, scenes and capabilities).
 */
export async function mountPotoruPlayer(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/potoru: mountPotoruPlayer needs a host element");
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

/* ── Compiler ────────────────────────────────────────────────────────────── */

/**
 * Compiles Potoru source to a `.poto` package, in the browser.
 *  • Way A: `{ files }` — a format v4 source folder as a map path → text (or Uint8Array for binaries).
 *  • Way B (experimental): `{ script, files? }` — JavaScript using the authoring API, run in a
 *    sandboxed iframe (no network, opaque origin, hard timeout); it returns a Project.
 * Options: `libraries` (Uint8Array[] of .potolib / library .poto), `scene`, `state`, `locale`,
 * `profile` ("presentation": every root State becomes a slide), `timeoutMs` (Way B, default 10000).
 * Resolves to `{ ok, poto?, diagnostics, files?, sizes }`; never rejects for bad source.
 */
export async function compile(input = {}) {
  const module = await loadCompiler();
  return module.compile(input);
}

/** Mounts the authoring widget (file list, editor, diagnostics, live preview) into `host`. */
export async function mountPotoruCompiler(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/potoru: mountPotoruCompiler needs a host element");
  if (options.id && !host.id) host.id = options.id;
  const id = ensureId(host, "potoru-compiler");
  const existing = compilers.get(id);
  if (existing && existing.host === host) return existing;
  for (const [key, value] of Object.entries(placeholderStyle({ height: options.height || "560px" }))) if (!host.style[key]) host.style[key] = value;
  const module = await loadCompiler();
  const widget = await module.createCompilerWidget(host, compilerConfig(options), {
    id,
    emit: (type, detail) => emit(host, `zeb:potoru-compiler:${type}`, detail)
  });
  widget.destroy = ((destroy) => () => { destroy(); if (compilers.get(id) === widget) compilers.delete(id); })(widget.destroy);
  compilers.set(id, widget);
  return widget;
}

/* ── Auto-mount of server-rendered placeholders ──────────────────────────── */

function mountNode(host) {
  if (host.__zebPotoru) return;
  const wrapper = host.getAttribute("data-zeb-wrapper") || PLAYER_WRAPPER;
  const config = readConfig(host);
  host.__zebPotoru = wrapper;
  const mount = wrapper === COMPILER_WRAPPER ? mountPotoruCompiler : mountPotoruPlayer;
  const event = wrapper === COMPILER_WRAPPER ? "zeb:potoru-compiler:error" : "zeb:potoru:error";
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
    const registry = host.__zebPotoru === COMPILER_WRAPPER ? compilers : players;
    const instance = registry.get(host.id);
    if (instance?.host === host) instance.destroy();
    host.__zebPotoru = undefined;
  }
}

function configChanged(host) {
  if (!host.__zebPotoru) return;
  const registry = host.__zebPotoru === COMPILER_WRAPPER ? compilers : players;
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
 * PotoruPlayer — props: src, libraries, controls, autoplay, muted, loop, captions, mode, renderer,
 * height, aspect, fit, camera, id, className. Renders the placeholder; the runtime mounts the
 * player inside it (in a shadow root, so re-renders never touch it).
 */
export function PotoruPlayer(props = {}) {
  const h = globalThis.h;
  if (!h) return null;
  const config = playerConfig(props);
  return h("div", { "data-zeb-lib": PLAYER_LIB, "data-zeb-wrapper": PLAYER_WRAPPER, "data-config": JSON.stringify(config), id: props.id, className: props.className, style: placeholderStyle(config) });
}

/** PotoruCompiler — props: files, script, mode ("files" | "script"), height, id, className. */
export function PotoruCompiler(props = {}) {
  const h = globalThis.h;
  if (!h) return null;
  const config = compilerConfig(props);
  return h("div", { "data-zeb-lib": PLAYER_LIB, "data-zeb-wrapper": COMPILER_WRAPPER, "data-config": JSON.stringify(config), id: props.id, className: props.className, style: placeholderStyle({ height: config.height || "560px" }) });
}

/* ── Registries and namespace ────────────────────────────────────────────── */

export const potoru = {
  version: VERSION,
  /** A mounted player by container id. */
  get: (id) => players.get(id),
  /** A mounted compiler widget by container id. */
  getCompiler: (id) => compilers.get(id),
  mountPotoruPlayer,
  mountPotoruCompiler,
  compile,
  loadPlayer
};

if (typeof window !== "undefined") {
  window.__zebPotoru = { version: VERSION, get: (id) => players.get(id), ids: () => [...players.keys()], mountPotoruPlayer, compile };
  window.__zebPotoruCompiler = { version: VERSION, get: (id) => compilers.get(id), ids: () => [...compilers.keys()], mountPotoruCompiler, compile };
}
