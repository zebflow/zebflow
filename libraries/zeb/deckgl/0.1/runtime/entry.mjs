/**
 * zeb/deckgl 0.1 — Deck.gl runtime for RWE templates.
 *
 * ── FEATURES ────────────────────────────────────────────────────────────────
 *  • Offline: @deck.gl/core + @deck.gl/layers bundled inline — no CDN.
 *  • Standalone: runs on a plain WebGL canvas, no map backend required.
 *  • SPA / API-first: MutationObserver auto-mounts whenever [data-zeb-lib="deckgl"]
 *    appears in the DOM — works after fetch-driven Preact re-renders.
 *  • Reactive: stateKey syncs view state; layerKey feeds layer data from page state.
 *  • Declarative layers: JSON layer specs with simple string accessors.
 *  • Imperative: window.__zebDeck.get(id) → { deck, setLayers, setViewState }
 *  • zeb:deck:ready event for post-mount access.
 *
 * ── OFFLINE BUNDLE ───────────────────────────────────────────────────────────
 *  cd /tmp/zeb-deckgl-build
 *  npm install
 *  node_modules/.bin/esbuild entry.mjs \
 *    --bundle --format=esm --minify \
 *    --outfile=deckgl.bundle.mjs
 *  cp deckgl.bundle.mjs libraries/zeb/deckgl/0.1/runtime/
 *
 * ── QUICK REFERENCE ─────────────────────────────────────────────────────────
 *  TSX import:   import DeckMap from "zeb/deckgl";
 *  Imperative:   window.__zebDeck.get("map-id").deck.setProps({ layers })
 *  Event:        container.addEventListener("zeb:deck:ready", e => e.detail.deck)
 */

/* ── Static imports — bundled inline by esbuild ── */
import {
  Deck,
  MapView,
  OrthographicView,
  OrbitView,
  FirstPersonView,
  WebMercatorViewport,
  COORDINATE_SYSTEM,
  picking,
  project,
  project32,
} from "@deck.gl/core";

import {
  ScatterplotLayer,
  LineLayer,
  ArcLayer,
  PathLayer,
  PolygonLayer,
  SolidPolygonLayer,
  IconLayer,
  TextLayer,
  ColumnLayer,
  GridCellLayer,
  PointCloudLayer,
  BitmapLayer,
} from "@deck.gl/layers";

/* ── The full deck namespace — exported for power users ── */
const _deck = {
  /* Core */
  Deck,
  MapView,
  OrthographicView,
  OrbitView,
  FirstPersonView,
  WebMercatorViewport,
  COORDINATE_SYSTEM,
  picking,
  project,
  project32,
  /* Layers */
  ScatterplotLayer,
  LineLayer,
  ArcLayer,
  PathLayer,
  PolygonLayer,
  SolidPolygonLayer,
  IconLayer,
  TextLayer,
  ColumnLayer,
  GridCellLayer,
  PointCloudLayer,
  BitmapLayer,
};

/* ── Layer type registry — used by buildLayer() ── */
const LAYER_TYPES = {
  ScatterplotLayer,
  LineLayer,
  ArcLayer,
  PathLayer,
  PolygonLayer,
  SolidPolygonLayer,
  IconLayer,
  TextLayer,
  ColumnLayer,
  GridCellLayer,
  PointCloudLayer,
  BitmapLayer,
};

/* ── Default view state ── */
const DEFAULT_VIEW_STATE = {
  longitude: 0,
  latitude: 20,
  zoom: 1.5,
  pitch: 0,
  bearing: 0,
};

/* ── String accessor helper ──────────────────────────────────────────────────
 * Converts simple string accessor specs to functions:
 *   "position"        → d => d.position
 *   "[lon, lat]"      → d => [d.lon, d.lat]   (bracket expansion)
 *   "[0]"             → d => d[0]              (array index)
 * Non-string values (arrays, numbers, functions) pass through unchanged.
 */
function resolveAccessor(spec) {
  if (typeof spec !== "string") return spec;
  /* Bracket expansion: "[fieldA, fieldB]" → d => [d.fieldA, d.fieldB] */
  const bracketMatch = spec.match(/^\[([^\]]+)\]$/);
  if (bracketMatch) {
    const fields = bracketMatch[1].split(",").map((f) => f.trim());
    return (d) => fields.map((f) => (/^\d+$/.test(f) ? d[Number(f)] : d[f]));
  }
  /* Simple property: "position" → d => d.position */
  return (d) => d[spec];
}

/* ── Layer builder — builds a Layer instance from a plain config object ──────
 *
 * Config shape:
 *   { type: "ScatterplotLayer", id: "pts", data: [...], getPosition: "position",
 *     getFillColor: [255, 100, 0], getRadius: 120, radiusScale: 1, pickable: true }
 *
 * All accessor props (getPosition, getFillColor, getRadius, etc.) accept either
 * real values / functions OR simple string accessors (resolved by resolveAccessor).
 */
const ACCESSOR_PROPS = new Set([
  "getPosition", "getSourcePosition", "getTargetPosition",
  "getColor", "getFillColor", "getLineColor", "getSourceColor", "getTargetColor",
  "getRadius", "getWidth", "getHeight", "getSize", "getWeight",
  "getIcon", "getText", "getElevation", "getOrientation",
  "getPath", "getPolygon", "getNormal",
]);

function buildLayer(cfg) {
  if (!cfg || !cfg.type) return null;
  const LayerClass = LAYER_TYPES[cfg.type];
  if (!LayerClass) {
    console.warn(`zeb/deckgl: unknown layer type "${cfg.type}"`);
    return null;
  }
  const props = {};
  for (const [k, v] of Object.entries(cfg)) {
    if (k === "type") continue;
    props[k] = ACCESSOR_PROPS.has(k) ? resolveAccessor(v) : v;
  }
  return new LayerClass(props);
}

function buildLayers(specs) {
  if (!Array.isArray(specs)) return [];
  return specs.map(buildLayer).filter(Boolean);
}

/* ── Instance registry ── */
const _instances = new Map();

/* ── Core mount ──────────────────────────────────────────────────────────────
 * Mounts a Deck.gl instance into the given container element.
 * Called by the MutationObserver whenever [data-zeb-lib="deckgl"] enters the DOM.
 */
function mountDeckCanvas(container) {
  if (container._zdMounted) return;
  container._zdMounted = true;

  /* Parse config from data-config attribute */
  let config = {};
  try { config = JSON.parse(container.dataset.config || "{}"); } catch {}

  const {
    initialViewState = DEFAULT_VIEW_STATE,
    controller       = true,
    views            = null,   /* null → Deck.gl default (MapView) */
    layers: layerSpecs = [],
    stateKey         = null,
    layerKey         = null,
    background       = "transparent",
  } = config;

  /* Auto-id */
  if (!container.id) {
    container.id = `zd-${Math.random().toString(36).slice(2, 8)}`;
  }
  const instanceId = container.id;

  /* Container sizing — deck.gl needs position:relative and explicit dimensions */
  container.style.position = "relative";
  if (!container.style.width)  container.style.width  = "100%";
  if (!container.style.height && !container.style.minHeight) {
    container.style.height = "400px";
  }

  /* Read initial data from page state if layerKey is set */
  let initialLayers = buildLayers(layerSpecs);
  if (layerKey && window.__rwePageState?.[layerKey]) {
    initialLayers = buildLayers(window.__rwePageState[layerKey]);
  }

  let currentViewState = initialViewState;

  const deck = new Deck({
    parent:           container,
    controller,
    views:            views || [new MapView({ repeat: true })],
    initialViewState: currentViewState,
    layers:           initialLayers,
    parameters: {
      clearColor: background === "transparent" ? [0, 0, 0, 0] : null,
    },

    /* Push view state changes back to page state */
    onViewStateChange({ viewState }) {
      currentViewState = viewState;
      if (stateKey) {
        window.__rweSetPageState?.({ [stateKey]: viewState });
      }
    },
  });

  /* Reactive listener — responds to page-state changes */
  let _listener = null;
  if (stateKey || layerKey) {
    _listener = (e) => {
      const patch = {};
      if (stateKey && e.detail?.[stateKey] !== undefined) {
        const vs = e.detail[stateKey];
        if (vs && typeof vs === "object" && vs !== currentViewState) {
          currentViewState = vs;
          patch.initialViewState = vs;
        }
      }
      if (layerKey && e.detail?.[layerKey] !== undefined) {
        patch.layers = buildLayers(e.detail[layerKey]);
      }
      if (Object.keys(patch).length) deck.setProps(patch);
    };
    window.addEventListener("rwe:state:change", _listener);
  }

  /* Public instance */
  const instance = {
    deck,
    /** Replace all layers. Accepts Layer instances or JSON config objects. */
    setLayers(layers) {
      deck.setProps({
        layers: layers.map((l) => (l && typeof l.draw === "function" ? l : buildLayer(l))).filter(Boolean),
      });
    },
    /** Update view state. */
    setViewState(vs) {
      currentViewState = vs;
      deck.setProps({ initialViewState: vs });
    },
    /** Destroy the instance and clean up. */
    destroy() {
      if (_listener) window.removeEventListener("rwe:state:change", _listener);
      deck.finalize();
      _instances.delete(instanceId);
      container._zdMounted = false;
    },
  };

  _instances.set(instanceId, instance);

  container.dispatchEvent(new CustomEvent("zeb:deck:ready", {
    bubbles: true,
    detail: { instance, id: instanceId, deck },
  }));
}

function destroyDeckCanvas(node) {
  if (node.nodeType !== 1) return;
  if (node._zdMounted && node.id) _instances.get(node.id)?.destroy();
  node.querySelectorAll?.("[data-zeb-lib='deckgl']").forEach((el) => {
    if (el._zdMounted && el.id) _instances.get(el.id)?.destroy();
  });
}

/* ── MutationObserver — auto-mount / auto-destroy ── */
const _observer = new MutationObserver((mutations) => {
  for (const mut of mutations) {
    for (const node of mut.addedNodes) {
      if (node.nodeType !== 1) continue;
      if (node.matches?.("[data-zeb-lib='deckgl']")) mountDeckCanvas(node);
      node.querySelectorAll?.("[data-zeb-lib='deckgl']").forEach(mountDeckCanvas);
    }
    for (const node of mut.removedNodes) {
      destroyDeckCanvas(node);
    }
  }
});

if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      _observer.observe(document.body, { childList: true, subtree: true });
      document.querySelectorAll("[data-zeb-lib='deckgl']").forEach(mountDeckCanvas);
    });
  } else {
    _observer.observe(document.body, { childList: true, subtree: true });
    document.querySelectorAll("[data-zeb-lib='deckgl']").forEach(mountDeckCanvas);
  }
}

/* ── Public surface ── */
if (typeof window !== "undefined") {
  window.__zebDeck = {
    get(id)               { return _instances.get(id); },
    buildLayer,
    buildLayers,
    deck: _deck,
  };
}

/* ── Exports ── */

export function ensureDeck() {
  return _deck;
}

export { buildLayer, buildLayers };

export function createDeckMapRuntime(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/deckgl: host element is required");
  if (!host.id) host.id = `zd-${Math.random().toString(36).slice(2, 8)}`;

  host.style.position = "relative";
  if (!host.style.width)  host.style.width  = "100%";
  if (!host.style.height && !host.style.minHeight) host.style.height = "400px";

  const {
    initialViewState = DEFAULT_VIEW_STATE,
    controller = true,
    layers: layerCfgs = [],
    views,
  } = options;

  const layers = layerCfgs.map((l) =>
    l && typeof l.draw === "function" ? l : buildLayer(l)
  ).filter(Boolean);

  const deck = new Deck({
    parent: host,
    controller,
    views: views || [new MapView({ repeat: true })],
    initialViewState,
    layers,
    ...options.deckProps,
  });

  return {
    deck,
    setLayers(next) {
      deck.setProps({
        layers: next.map((l) => (l && typeof l.draw === "function" ? l : buildLayer(l))).filter(Boolean),
      });
    },
    setViewState(vs) { deck.setProps({ initialViewState: vs }); },
    destroy()        { deck.finalize(); },
  };
}

export function mountDeckMap(host, options = {}) {
  return createDeckMapRuntime(host, options);
}

export const deckgl = {
  ensureDeck,
  createDeckMapRuntime,
  mountDeckMap,
  buildLayer,
  buildLayers,
  ..._deck,
};

/**
 * DeckMap — Preact component for RWE templates.
 *
 * Uses useRef + useEffect to avoid Preact hydration conflicts (same pattern
 * as zeb/prosemirror's ProseEditor). The component renders an invisible
 * display:contents wrapper; useEffect appends the real [data-zeb-lib="deckgl"]
 * div after hydration — MutationObserver catches it → mountDeckCanvas().
 *
 * Props:
 *   id                string    Container id (for window.__zebDeck.get(id))
 *   height            string    CSS height, default "400px"
 *   className         string    Tailwind classes on the sentinel div
 *   initialViewState  object    { longitude, latitude, zoom, pitch, bearing }
 *   controller        boolean   Enable pan/zoom/rotate. Default true.
 *   layers            array     JSON layer config objects (see buildLayer)
 *   stateKey          string    Page state key for two-way view state sync
 *   layerKey          string    Page state key → layer data array
 *   background        string    "transparent" (default) or CSS color
 */
export function DeckMap(props) {
  const _h         = globalThis.h;
  const _useRef    = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = {
    initialViewState: props.initialViewState || DEFAULT_VIEW_STATE,
    controller:       props.controller !== false,
    layers:           props.layers || [],
    stateKey:         props.stateKey  || null,
    layerKey:         props.layerKey  || null,
    background:       props.background || "transparent",
  };

  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const inner = document.createElement("div");
      inner.setAttribute("data-zeb-lib", "deckgl");
      inner.setAttribute("data-config", JSON.stringify(config));
      if (props.id) inner.id = props.id;
      inner.style.width  = "100%";
      inner.style.height = props.height || "400px";
      if (props.className) inner.className = props.className;
      wrap.appendChild(inner);

      return () => { inner.remove(); };
    }, []);

    return _h("div", {
      ref:                wrapRef,
      "data-zeb-wrapper": "DeckMap",
      style:              { display: "contents" },
    });
  }

  /* SSR fallback */
  return _h("div", {
    "data-zeb-lib":     "deckgl",
    "data-zeb-wrapper": "DeckMap",
    "data-config":      JSON.stringify(config),
    id:                 props.id,
    style:              { width: "100%", height: props.height || "400px" },
    class:              props.className,
  });
}
