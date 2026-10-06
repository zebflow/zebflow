import { h } from "zeb/react";
import { defineExtension, routeSearch, safeHref } from "zeb/ui/editor-extension";
import { PotoruLibraries, mergePotoruLibraries, potoruLibraryUrl } from "zeb/ui/editor-potoru-libraries";

/**
 * Potoru extension — a Potoru story (`.poto`) as a block of the document,
 * played by `zeb/potoru`'s PotoPlayer.
 *
 *   potoruExtension()                                             paste an address in the panel
 *   potoruExtension({ route: "/api/stories/search" })             or pick one from a project route
 *   potoruExtension({ libraries: ["/_files/libs/basic.potolib"] })  libraries every block gets
 *
 * The node stores `{ key, src, mode, still, time, libraries, snapshot: { title, poster } }`.
 * It is drawn by components (the extension contract's `component` and
 * `editComponent`): `PotoruBlock` renders the placeholder PotoPlayer's server
 * stub renders — `data-zeb-lib="potoru"`, `data-zeb-wrapper="PotoPlayer"`
 * and the same `data-config` — so the page hydrates into the player, and the
 * editor shows it as a live preview with the block's library list under it
 * (`PotoruBlockEditor`). The poster frame, when the snapshot has one, sits
 * inside the placeholder: it shows until the player mounts in its shadow
 * root and hides it.
 *
 * Libraries: the extension's `libraries` are defaults for every block; a
 * block adds its own in the editor (stored in its attrs). The two merge —
 * the block's entry wins over a default with the same file name — and the
 * merged list is the player's `libraries`. Each address must be `https:` or
 * a path on this site (`/…`); a bad default is refused when the extension is
 * built, a bad stored entry is left out.
 *
 * This file never imports `zeb/potoru`. The page loads the library itself,
 * and only when a placeholder is actually on it (RWE's markup-mounted
 * libraries), so a page of documents without a story downloads nothing. The
 * project must have `zeb/potoru` enabled if it restricts its libraries.
 *
 * A picker route answers `GET route?q=…` with `{ id, label, href, snapshot }`
 * where `href` is the `.poto` address and `snapshot` may carry `poster` and
 * `libraries`. The story must be same-origin, or served with CORS.
 */

export const POTORU_MODES = ["play", "slide", "interactive"];

/**
 * `data-config` exactly as PotoPlayer builds it for
 * `{ src, libraries, controls, still, time, mode }`; `defaults` are the
 * extension's libraries.
 */
export function potoruConfig(attrs, defaults) {
  const a = attrs || {};
  const config = { src: a.src ? safeHref(a.src) : "" };
  const libraries = mergePotoruLibraries(defaults, a.libraries);
  if (libraries.length) config.libraries = libraries;
  config.controls = true;
  const still = a.still === true || a.still === "true";
  if (still) config.still = true;
  const time = Number(a.time);
  if (still && a.time !== "" && a.time !== null && a.time !== undefined && Number.isFinite(time) && time >= 0) config.time = time;
  if (POTORU_MODES.includes(a.mode)) config.mode = a.mode;
  return config;
}

/** The block as the page shows it: PotoPlayer's placeholder, its poster and its title. */
export function PotoruBlock({ attrs, options, edit }) {
  const a = attrs || {};
  const s = a.snapshot || {};
  const figure = { "data-potoru-block": options.name, "data-key": a.key || undefined, className: "my-4" };
  if (!a.src) {
    if (!edit) return null;
    return h("figure", figure, h("div", { className: "rounded-lg border border-dashed border-border px-4 py-6 text-center text-sm text-muted-foreground" }, `${options.label}: set its .poto address in the panel`));
  }
  return h("figure", figure,
    h("div", {
      "data-zeb-lib": "potoru", "data-zeb-wrapper": "PotoPlayer", "data-config": JSON.stringify(potoruConfig(a, options.libraries)),
      className: "relative block w-full overflow-hidden rounded-lg bg-muted",
    }, s.poster ? h("img", { src: safeHref(s.poster), alt: s.title || "", className: "block w-full" }) : null),
    s.title ? h("figcaption", { className: "mt-2 text-sm text-muted-foreground" }, s.title) : null);
}

/** The block in the editor: the live preview, then the libraries it plays with. */
export function PotoruBlockEditor(props) {
  const { attrs, options, readOnly, selected, update } = props;
  return h("div", { "data-potoru-editor": options.name, className: selected ? "rounded-lg ring-2 ring-ring/50" : "" },
    h(PotoruBlock, props),
    h(PotoruLibraries, { defaults: options.libraries, libraries: attrs.libraries, readOnly, onChange: (libraries) => update({ libraries }) }));
}

export function potoruExtension({ name = "potoru", route, search, label = "Potoru story", hint = "Play a .poto story", libraries = [] } = {}) {
  if (!Array.isArray(libraries)) throw new Error(`potoruExtension "${name}": libraries is a list of addresses`);
  for (const url of libraries) {
    if (!potoruLibraryUrl(url)) throw new Error(`potoruExtension "${name}": library ${JSON.stringify(url)} must be an https:// address or a path on this site (/…)`);
  }
  return defineExtension({
    name,
    node: {
      group: "block", atom: true, draggable: true,
      attrs: {
        key: { default: "" }, src: { default: "" }, mode: { default: "" }, still: { default: false }, time: { default: 0 },
        libraries: { default: [] }, snapshot: { default: {} },
      },
    },
    options: { name, label, libraries: libraries.map((url) => url.trim()) },
    component: PotoruBlock,
    editComponent: PotoruBlockEditor,
    text: (node) => (node.attrs.snapshot && node.attrs.snapshot.title) || node.attrs.key || node.attrs.src,
    picker: route || search ? {
      search: search || routeSearch(route),
      toAttrs: (item) => {
        const s = item.snapshot || {};
        const own = Array.isArray(s.libraries) ? s.libraries.filter(potoruLibraryUrl) : [];
        return { key: String(item.id), src: item.href || s.src || "", mode: s.mode || "", still: false, time: 0, libraries: own, snapshot: { title: item.label || "", poster: s.poster || "" } };
      },
    } : null,
    insert: [{ id: name, label, hint, keys: `potoru story animation poto ${name}`, run: (api) => (route || search ? api.pick() : api.insert(name)) }],
    fields: [
      { name: "src", label: ".poto address", type: "url", placeholder: "/_files/stories/intro.poto" },
      { name: "mode", label: "Mode", type: "select", options: ["", ...POTORU_MODES] },
      { name: "still", label: "Still frame", type: "checkbox" },
      { name: "time", label: "Seconds (with still)", type: "number" },
      { name: "snapshot.title", label: "Title" },
      { name: "snapshot.poster", label: "Poster image", type: "url" },
      { name: "key", label: "Key" },
    ],
  });
}

export default potoruExtension;
