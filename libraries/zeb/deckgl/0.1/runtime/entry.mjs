/**
 * zeb/deckgl 0.1 — Deck.gl runtime for RWE templates.
 *
 * ── FEATURES ────────────────────────────────────────────────────────────────
 *  • Offline: @deck.gl/core + layers + aggregation + geo + mesh + extensions
 *    bundled inline — no CDN.
 *  • Standalone: runs on a plain WebGL canvas, no map backend required.
 *  • SPA / API-first: MutationObserver auto-mounts whenever [data-zeb-lib="deckgl"]
 *    appears in the DOM — works after fetch-driven Preact re-renders.
 *  • Reactive: stateKey syncs view state; layerKey feeds layer data from page state.
 *  • Declarative layers: JSON layer specs with simple string accessors.
 *  • Imperative: window.__zebDeck.get(id) → { deck, setLayers, setViewState }
 *  • Tooltip: built-in hover tooltip via tooltip config.
 *  • Utilities: haversine, bearing, colorRamp, interpolateAlongPath, createAnimationLoop.
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
  FlyToInterpolator,
  LinearInterpolator,
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

import {
  HeatmapLayer,
  HexagonLayer,
  GridLayer,
  ContourLayer,
  ScreenGridLayer,
} from "@deck.gl/aggregation-layers";

import {
  GeoJsonLayer,
  TileLayer,
  MVTLayer,
} from "@deck.gl/geo-layers";

import {
  ScenegraphLayer,
  SimpleMeshLayer,
} from "@deck.gl/mesh-layers";

import {
  PathStyleExtension,
  DataFilterExtension,
  BrushingExtension,
  CollisionFilterExtension,
} from "@deck.gl/extensions";

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
  FlyToInterpolator,
  LinearInterpolator,
  picking,
  project,
  project32,
  /* Basic Layers */
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
  /* Aggregation Layers */
  HeatmapLayer,
  HexagonLayer,
  GridLayer,
  ContourLayer,
  ScreenGridLayer,
  /* Geo Layers */
  GeoJsonLayer,
  TileLayer,
  MVTLayer,
  /* Mesh Layers */
  ScenegraphLayer,
  SimpleMeshLayer,
  /* Extensions */
  PathStyleExtension,
  DataFilterExtension,
  BrushingExtension,
  CollisionFilterExtension,
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
  /* Aggregation */
  HeatmapLayer,
  HexagonLayer,
  GridLayer,
  ContourLayer,
  ScreenGridLayer,
  /* Geo */
  GeoJsonLayer,
  TileLayer,
  MVTLayer,
  /* Mesh */
  ScenegraphLayer,
  SimpleMeshLayer,
};

/* ── Extension type registry ── */
const EXTENSION_TYPES = {
  PathStyleExtension,
  DataFilterExtension,
  BrushingExtension,
  CollisionFilterExtension,
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

/* ── Extension builder ── */
function buildExtension(cfg) {
  if (!cfg || !cfg.type) return null;
  const ExtClass = EXTENSION_TYPES[cfg.type];
  if (!ExtClass) {
    console.warn(`zeb/deckgl: unknown extension type "${cfg.type}"`);
    return null;
  }
  const opts = {};
  for (const [k, v] of Object.entries(cfg)) {
    if (k === "type") continue;
    opts[k] = v;
  }
  return new ExtClass(opts);
}

/* ── Layer builder — builds a Layer instance from a plain config object ──────
 *
 * Config shape:
 *   { type: "ScatterplotLayer", id: "pts", data: [...], getPosition: "position",
 *     getFillColor: [255, 100, 0], getRadius: 120, radiusScale: 1, pickable: true }
 *
 * All accessor props (getPosition, getFillColor, getRadius, etc.) accept either
 * real values / functions OR simple string accessors (resolved by resolveAccessor).
 *
 * Special: extensions: [{ type: "PathStyleExtension", dash: true }]
 * Special: TileLayer renderSubLayers: "bitmap" shorthand
 */
const ACCESSOR_PROPS = new Set([
  "getPosition", "getSourcePosition", "getTargetPosition",
  "getColor", "getFillColor", "getLineColor", "getSourceColor", "getTargetColor",
  "getRadius", "getWidth", "getHeight", "getSize", "getWeight",
  "getIcon", "getText", "getElevation", "getOrientation",
  "getPath", "getPolygon", "getNormal", "getDashArray",
  "getFilterValue", "getFilterCategory",
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
    if (k === "extensions" && Array.isArray(v)) {
      props.extensions = v.map(buildExtension).filter(Boolean);
      continue;
    }
    /* TileLayer renderSubLayers shorthand: "bitmap" → auto BitmapLayer */
    if (k === "renderSubLayers" && v === "bitmap") {
      props.renderSubLayers = (tileProps) => {
        const { boundingBox: [[west, south], [east, north]], data: image } = tileProps;
        return new BitmapLayer({
          ...tileProps,
          data: null,
          image,
          bounds: [west, south, east, north],
        });
      };
      continue;
    }
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

/* ── Tooltip ─────────────────────────────────────────────────────────────────
 * Built-in hover tooltip. Injects a floating div, positions on hover,
 * populates with object properties.
 */
let _tooltipEl = null;

function ensureTooltipEl() {
  if (_tooltipEl) return _tooltipEl;
  if (typeof document === "undefined") return null;
  _tooltipEl = document.createElement("div");
  _tooltipEl.className = "zeb-deck-tooltip";
  _tooltipEl.style.cssText =
    "position:fixed;pointer-events:none;z-index:9999;padding:6px 10px;" +
    "background:rgba(0,0,0,0.82);color:#fff;font-size:12px;line-height:1.4;" +
    "border-radius:4px;max-width:300px;white-space:pre-wrap;display:none;" +
    "font-family:system-ui,sans-serif;box-shadow:0 2px 8px rgba(0,0,0,0.3);";
  document.body.appendChild(_tooltipEl);
  return _tooltipEl;
}

function showTooltip(info) {
  const el = ensureTooltipEl();
  if (!el) return;
  if (!info || !info.object) {
    el.style.display = "none";
    return;
  }
  const obj = info.object;
  const lines = [];
  /* Build tooltip from object properties — skip internal/function/array fields */
  if (obj.properties) {
    /* GeoJSON feature */
    for (const [k, v] of Object.entries(obj.properties)) {
      if (typeof v !== "function" && v !== null && v !== undefined) {
        lines.push(`${k}: ${v}`);
      }
    }
  } else {
    for (const [k, v] of Object.entries(obj)) {
      if (k.startsWith("_") || typeof v === "function") continue;
      if (Array.isArray(v) && v.length > 4) continue;
      if (typeof v === "object" && v !== null) continue;
      if (v !== null && v !== undefined) lines.push(`${k}: ${v}`);
    }
  }
  if (!lines.length) {
    el.style.display = "none";
    return;
  }
  el.textContent = lines.join("\n");
  el.style.display = "block";
  el.style.left = (info.x + 12) + "px";
  el.style.top  = (info.y + 12) + "px";
}

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
    tooltip          = false,
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

  const deckProps = {
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
  };

  /* Tooltip support */
  if (tooltip) {
    deckProps.onHover = showTooltip;
    deckProps.getCursor = ({ isHovering }) => isHovering ? "pointer" : "grab";
  }

  const deck = new Deck(deckProps);

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
      container._zdInstance = null;
    },
  };

  _instances.set(instanceId, instance);
  container._zdInstance = instance;

  container.dispatchEvent(new CustomEvent("zeb:deck:ready", {
    bubbles: true,
    detail: { instance, id: instanceId, deck },
  }));
}

function destroyDeckCanvas(node) {
  if (node.nodeType !== 1) return;
  if (node._zdMounted) {
    node._zdInstance?.destroy();
    if (node.id) _instances.get(node.id)?.destroy();
  }
  node.querySelectorAll?.("[data-zeb-lib='deckgl']").forEach((el) => {
    if (el._zdMounted) {
      el._zdInstance?.destroy();
      if (el.id) _instances.get(el.id)?.destroy();
    }
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

/* ══════════════════════════════════════════════════════════════════════════════
 * UTILITY FUNCTIONS
 * ══════════════════════════════════════════════════════════════════════════════ */

const DEG2RAD = Math.PI / 180;
const RAD2DEG = 180 / Math.PI;
const EARTH_RADIUS = 6371000; /* meters */

/**
 * haversine(a, b) — distance in meters between two [longitude, latitude] points.
 */
export function haversine(a, b) {
  const [lon1, lat1] = a;
  const [lon2, lat2] = b;
  const dLat = (lat2 - lat1) * DEG2RAD;
  const dLon = (lon2 - lon1) * DEG2RAD;
  const sinLat = Math.sin(dLat / 2);
  const sinLon = Math.sin(dLon / 2);
  const h = sinLat * sinLat +
    Math.cos(lat1 * DEG2RAD) * Math.cos(lat2 * DEG2RAD) * sinLon * sinLon;
  return 2 * EARTH_RADIUS * Math.asin(Math.sqrt(h));
}

/**
 * bearing(a, b) — bearing in degrees (0–360) from point A to point B.
 * Points are [longitude, latitude].
 */
export function bearing(a, b) {
  const [lon1, lat1] = a;
  const [lon2, lat2] = b;
  const dLon = (lon2 - lon1) * DEG2RAD;
  const y = Math.sin(dLon) * Math.cos(lat2 * DEG2RAD);
  const x = Math.cos(lat1 * DEG2RAD) * Math.sin(lat2 * DEG2RAD) -
            Math.sin(lat1 * DEG2RAD) * Math.cos(lat2 * DEG2RAD) * Math.cos(dLon);
  return ((Math.atan2(y, x) * RAD2DEG) + 360) % 360;
}

/**
 * colorRamp(t, stops) — interpolate a color ramp at position t (0–1).
 * Returns [r, g, b, a].
 *
 * stops can be:
 *   - A named preset: "green-red", "blue-red", "cool", "warm", "viridis"
 *   - An array: [[pos, [r,g,b]], [pos, [r,g,b]], ...]
 */
const COLOR_PRESETS = {
  "green-red": [[0, [0, 200, 0]], [0.5, [255, 200, 0]], [1, [255, 0, 0]]],
  "blue-red":  [[0, [0, 100, 255]], [0.5, [255, 255, 0]], [1, [255, 0, 0]]],
  "cool":      [[0, [0, 255, 255]], [1, [255, 0, 255]]],
  "warm":      [[0, [255, 255, 0]], [1, [255, 0, 0]]],
  "viridis":   [[0, [68, 1, 84]], [0.25, [59, 82, 139]], [0.5, [33, 145, 140]], [0.75, [94, 201, 98]], [1, [253, 231, 37]]],
};

export function colorRamp(t, stops) {
  const ramp = typeof stops === "string" ? COLOR_PRESETS[stops] || COLOR_PRESETS["green-red"] : stops;
  if (!ramp || !ramp.length) return [128, 128, 128, 255];
  const ct = Math.max(0, Math.min(1, t));

  /* Find bracketing stops */
  if (ct <= ramp[0][0]) return [...ramp[0][1], 255];
  if (ct >= ramp[ramp.length - 1][0]) return [...ramp[ramp.length - 1][1], 255];

  for (let i = 1; i < ramp.length; i++) {
    if (ct <= ramp[i][0]) {
      const [t0, c0] = ramp[i - 1];
      const [t1, c1] = ramp[i];
      const f = (ct - t0) / (t1 - t0);
      return [
        Math.round(c0[0] + (c1[0] - c0[0]) * f),
        Math.round(c0[1] + (c1[1] - c0[1]) * f),
        Math.round(c0[2] + (c1[2] - c0[2]) * f),
        255,
      ];
    }
  }
  return [128, 128, 128, 255];
}

/**
 * interpolateAlongPath(path, progress) — returns [longitude, latitude, heading]
 * at a given progress (0–1) along a coordinate array.
 *
 * path: [[lon, lat], [lon, lat], ...] — at least 2 points
 * progress: 0–1 (clamped)
 */
export function interpolateAlongPath(path, progress) {
  if (!path || path.length < 2) {
    return path?.[0] ? [...path[0], 0] : [0, 0, 0];
  }

  const t = Math.max(0, Math.min(1, progress));

  /* Compute cumulative distances */
  const dists = [0];
  for (let i = 1; i < path.length; i++) {
    dists.push(dists[i - 1] + haversine(path[i - 1], path[i]));
  }
  const totalDist = dists[dists.length - 1];
  if (totalDist === 0) return [...path[0], 0];

  const targetDist = t * totalDist;

  /* Find segment */
  let seg = 0;
  for (let i = 1; i < dists.length; i++) {
    if (dists[i] >= targetDist) { seg = i - 1; break; }
    if (i === dists.length - 1) seg = i - 1;
  }

  const segLen = dists[seg + 1] - dists[seg];
  const f = segLen > 0 ? (targetDist - dists[seg]) / segLen : 0;

  const [lon0, lat0] = path[seg];
  const [lon1, lat1] = path[seg + 1];
  const lon = lon0 + (lon1 - lon0) * f;
  const lat = lat0 + (lat1 - lat0) * f;
  const hdg = bearing(path[seg], path[seg + 1]);

  return [lon, lat, hdg];
}

/**
 * createAnimationLoop(options) — requestAnimationFrame loop with play/pause/speed.
 *
 * Options:
 *   duration    number    Total duration in ms (default: 30000)
 *   speed       number    Playback speed multiplier (default: 1)
 *   loop        boolean   Repeat when done (default: false)
 *   onTick      function  Called each frame with progress (0–1)
 *   onComplete  function  Called when playback finishes (if not looping)
 *
 * Returns: { play, pause, seek, setSpeed, getProgress, destroy }
 */
export function createAnimationLoop(options = {}) {
  const {
    duration   = 30000,
    speed: initSpeed = 1,
    loop       = false,
    onTick     = () => {},
    onComplete = () => {},
  } = options;

  let _speed     = initSpeed;
  let _playing   = false;
  let _progress  = 0;  /* 0–1 */
  let _raf       = null;
  let _lastTime  = null;
  let _destroyed = false;

  function tick(now) {
    if (_destroyed || !_playing) return;
    if (_lastTime === null) _lastTime = now;

    const dt = (now - _lastTime) * _speed;
    _lastTime = now;
    _progress += dt / duration;

    if (_progress >= 1) {
      if (loop) {
        _progress = _progress % 1;
      } else {
        _progress = 1;
        _playing = false;
        onTick(_progress);
        onComplete();
        return;
      }
    }

    onTick(_progress);
    _raf = requestAnimationFrame(tick);
  }

  return {
    play() {
      if (_destroyed) return;
      _playing = true;
      _lastTime = null;
      _raf = requestAnimationFrame(tick);
    },
    pause() {
      _playing = false;
      if (_raf) { cancelAnimationFrame(_raf); _raf = null; }
    },
    seek(progress) {
      _progress = Math.max(0, Math.min(1, progress));
      _lastTime = null;
      onTick(_progress);
    },
    setSpeed(s) { _speed = s; },
    getProgress() { return _progress; },
    isPlaying() { return _playing; },
    destroy() {
      _destroyed = true;
      _playing = false;
      if (_raf) { cancelAnimationFrame(_raf); _raf = null; }
    },
  };
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
  if (_instances.has(host.id)) {
    _instances.get(host.id)?.destroy();
  }

  const instanceId = host.id;
  let stateListener = null;
  let currentViewState = options.initialViewState || DEFAULT_VIEW_STATE;

  function normalizeOptions(next = {}) {
    return {
      initialViewState: next.initialViewState || DEFAULT_VIEW_STATE,
      controller: next.controller !== false,
      layers: Array.isArray(next.layers) ? next.layers : [],
      views: next.views || null,
      stateKey: next.stateKey || null,
      layerKey: next.layerKey || null,
      tooltip: next.tooltip || false,
      background: next.background || "transparent",
      deckProps: next.deckProps && typeof next.deckProps === "object" ? next.deckProps : {},
    };
  }

  function mergeOptions(prev, next = {}) {
    return normalizeOptions({
      ...prev,
      ...next,
      deckProps: {
        ...(prev?.deckProps || {}),
        ...(next.deckProps || {}),
      },
    });
  }

  function applyHostStyles(config) {
    host.style.position = "relative";
    if (!host.style.width) host.style.width = "100%";
    if (!host.style.height && !host.style.minHeight) host.style.height = "400px";
    host.style.background = config.background && config.background !== "transparent"
      ? config.background
      : "";
  }

  function resolveLayers(config) {
    if (config.layerKey && window.__rwePageState?.[config.layerKey] !== undefined) {
      return buildLayers(window.__rwePageState[config.layerKey]);
    }
    return config.layers
      .map((layer) => (layer && typeof layer.draw === "function" ? layer : buildLayer(layer)))
      .filter(Boolean);
  }

  function bindStateListener(config, deck) {
    if (stateListener) {
      window.removeEventListener("rwe:state:change", stateListener);
      stateListener = null;
    }
    if (!(config.stateKey || config.layerKey)) return;

    stateListener = (e) => {
      const patch = {};
      if (config.stateKey && e.detail?.[config.stateKey] !== undefined) {
        const nextViewState = e.detail[config.stateKey];
        if (nextViewState && typeof nextViewState === "object") {
          currentViewState = nextViewState;
          patch.initialViewState = nextViewState;
        }
      }
      if (config.layerKey && e.detail?.[config.layerKey] !== undefined) {
        patch.layers = buildLayers(e.detail[config.layerKey]);
      }
      if (Object.keys(patch).length) {
        deck.setProps(patch);
      }
    };

    window.addEventListener("rwe:state:change", stateListener);
  }

  function buildDeckProps(config) {
    const deckProps = {
      ...config.deckProps,
      controller: config.controller,
      views: config.views || [new MapView({ repeat: true })],
      initialViewState: currentViewState,
      layers: resolveLayers(config),
      parameters: {
        ...(config.deckProps?.parameters || {}),
        clearColor: config.background === "transparent" ? [0, 0, 0, 0] : null,
      },
    };

    const userOnViewStateChange = config.deckProps?.onViewStateChange;
    deckProps.onViewStateChange = (event) => {
      currentViewState = event.viewState;
      userOnViewStateChange?.(event);
      if (config.stateKey) {
        window.__rweSetPageState?.({ [config.stateKey]: event.viewState });
      }
    };

    const userOnHover = config.deckProps?.onHover;
    if (config.tooltip) {
      deckProps.onHover = (info) => {
        showTooltip(info);
        userOnHover?.(info);
      };
      if (!config.deckProps?.getCursor) {
        deckProps.getCursor = ({ isHovering }) => (isHovering ? "pointer" : "grab");
      }
    }

    return deckProps;
  }

  let currentOptions = normalizeOptions(options);
  applyHostStyles(currentOptions);

  const deck = new Deck({
    parent: host,
    ...buildDeckProps(currentOptions),
  });

  bindStateListener(currentOptions, deck);

  const instance = {
    deck,
    setLayers(next) {
      currentOptions = mergeOptions(currentOptions, {
        layers: Array.isArray(next) ? next : [],
        layerKey: null,
      });
      deck.setProps({ layers: resolveLayers(currentOptions) });
    },
    setViewState(vs) {
      currentViewState = vs;
      deck.setProps({ initialViewState: vs });
    },
    setOptions(nextOptions = {}) {
      currentOptions = mergeOptions(currentOptions, nextOptions);
      if (nextOptions.initialViewState !== undefined) {
        currentViewState = currentOptions.initialViewState;
      }
      applyHostStyles(currentOptions);
      bindStateListener(currentOptions, deck);
      deck.setProps(buildDeckProps(currentOptions));
    },
    destroy() {
      if (stateListener) {
        window.removeEventListener("rwe:state:change", stateListener);
        stateListener = null;
      }
      deck.finalize();
      _instances.delete(instanceId);
      host._zdMounted = false;
      host._zdInstance = null;
    },
  };

  _instances.set(instanceId, instance);
  host._zdInstance = instance;

  host.dispatchEvent(new CustomEvent("zeb:deck:ready", {
    bubbles: true,
    detail: { instance, id: instanceId, deck },
  }));

  return instance;
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
  haversine,
  bearing,
  colorRamp,
  interpolateAlongPath,
  createAnimationLoop,
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
 *   tooltip           boolean   Enable default hover tooltip. Default false.
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
    tooltip:          props.tooltip   || false,
    background:       props.background || "transparent",
  };

  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);
    const hostRef = _useRef(null);
    const instanceRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const reuseWrap = wrap.dataset?.zebLib === "deckgl" || (props.id && wrap.id === props.id);
      const host = reuseWrap ? wrap : document.createElement("div");

      if (props.id && !host.id) host.id = props.id;
      host.style.width = "100%";
      host.style.height = props.height || "400px";
      host.className = props.className || "";

      if (!reuseWrap) {
        wrap.appendChild(host);
      }

      hostRef.current = host;
      instanceRef.current = createDeckMapRuntime(host, config);

      return () => {
        instanceRef.current?.destroy();
        instanceRef.current = null;
        hostRef.current = null;
        if (!reuseWrap) {
          host.remove();
        }
      };
    }, []);

    _useEffect(() => {
      const inner = hostRef.current;
      if (!inner) return;

      inner.style.width = "100%";
      inner.style.height = props.height || "400px";
      inner.className = props.className || "";

      instanceRef.current?.setOptions(config);
    }, [
      props.background,
      props.className,
      props.controller,
      props.height,
      props.initialViewState,
      props.layerKey,
      props.layers,
      props.stateKey,
      props.tooltip,
    ]);

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
