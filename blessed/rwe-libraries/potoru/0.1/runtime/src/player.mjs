/**
 * zeb/potoru — player part (loaded on demand by entry.mjs; bundled into `player-*.mjs` chunks).
 *
 * Importing Potoru's web player registers the `<potoru-player>` custom element. Its engines (action
 * wasm for interactive stories, score wasm + Worker for music, render wasm for path guides) and
 * built-in fonts load lazily from next to the bundle (`new URL(…, import.meta.url)`).
 * `@potoru-src/` is resolved by build/build.mjs to $POTORU_SRC (the Potoru `designer/e3` folder).
 */
import { PLAYER_CAPABILITIES, PLAYER_VERSION } from "@potoru-src/app/src/player/index.ts";

export { PLAYER_CAPABILITIES, PLAYER_VERSION };

/** Player events forwarded to the container as `zeb:potoru:<name>` (`status` is left out: chatty). */
const FORWARDED = ["ready", "play", "pause", "ended", "slide", "scene", "report", "caption", "error", "camera"];
const BOOLEAN_ATTRIBUTES = ["controls", "autoplay", "muted", "loop", "captions"];
const VALUE_ATTRIBUTES = ["mode", "renderer", "fit", "camera"];
const SHADOW_STYLE = ":host{display:block;position:relative}potoru-player{display:block;width:100%}potoru-player[data-fill]{height:100%;aspect-ratio:auto}";

function applyConfig(element, config, previous = {}) {
  for (const name of BOOLEAN_ATTRIBUTES) if (Boolean(config[name]) !== Boolean(previous[name]) || !(name in previous)) element.toggleAttribute(name, Boolean(config[name]));
  for (const name of VALUE_ATTRIBUTES) {
    const value = config[name];
    if (value === undefined || value === null || value === "") element.removeAttribute(name);
    else if (element.getAttribute(name) !== String(value)) element.setAttribute(name, String(value));
  }
  const libraries = (config.libraries ?? []).join(" ");
  if ((element.getAttribute("libraries") ?? "") !== libraries) {
    if (libraries) element.setAttribute("libraries", libraries); else element.removeAttribute("libraries");
  }
  element.toggleAttribute("data-fill", Boolean(config.height || config.aspect));
  // `src` last: the element loads when it is connected with a src, or when src changes.
  if (config.src && element.getAttribute("src") !== config.src) element.setAttribute("src", config.src);
}

/**
 * Mounts `<potoru-player>` inside `host`'s shadow root (a VDOM re-render of the placeholder never
 * reaches it) and returns the instance the registry hands out.
 */
export function createPlayer(host, config, { id, emit }) {
  const root = host.shadowRoot ?? host.attachShadow({ mode: "open" });
  root.textContent = "";
  const style = document.createElement("style");
  style.textContent = SHADOW_STYLE;
  // Parsed, not `document.createElement("potoru-player")`: the element's constructor sets an
  // attribute (`data-bar`), which the DOM forbids for synchronous construction ("The result must
  // not have attributes"); an element created by the HTML parser is upgraded instead, which allows it.
  const template = document.createElement("template");
  template.innerHTML = '<potoru-player part="player"></potoru-player>';
  const element = document.importNode(template.content, true).firstElementChild;
  let current = { ...config };
  applyConfig(element, current);
  const listeners = FORWARDED.map((name) => {
    const listener = (event) => emit(name, { id, ...(event.detail ?? {}) });
    element.addEventListener(`potoru:${name}`, listener);
    return [`potoru:${name}`, listener];
  });
  root.append(style, element);
  customElements.upgrade(element);

  const instance = {
    id,
    host,
    element,
    version: PLAYER_VERSION,
    capabilities: PLAYER_CAPABILITIES,
    play: () => element.play(),
    pause: () => element.pause(),
    seek: (seconds) => element.seek(Number(seconds)),
    restart: () => element.restart(),
    next: () => element.next(),
    prev: () => element.prev(),
    goto: (index) => element.goto(Number(index)),
    /** Host bridge: data the story declares `input: true`. */
    set: (target, data, value) => element.set(target, data, value),
    /** Host bridge: events the story declares `input: true`. */
    fire: (target, event, payload) => element.fire(target, event, payload),
    setCamera: (camera) => element.setCamera(camera),
    /** Loads another story: a URL, or `.poto` bytes (Uint8Array / ArrayBuffer); `libraries` URLs or bytes. */
    load: (src, options = {}) => element.load(src, options),
    currentTime: () => element.currentTime,
    duration: () => element.duration,
    get paused() { return element.paused; },
    get mode() { return element.mode; },
    get cameras() { return element.cameras; },
    get readyState() { return element.readyState; },
    /** New wrapper props (the placeholder's data-config changed). */
    configure(next) {
      const merged = { ...next };
      applyConfig(element, merged, current);
      current = merged;
    },
    destroy() {
      for (const [type, listener] of listeners) element.removeEventListener(type, listener);
      try { element.pause(); } catch { /* not loaded yet */ }
      element.remove();
      style.remove();
    }
  };
  return instance;
}
