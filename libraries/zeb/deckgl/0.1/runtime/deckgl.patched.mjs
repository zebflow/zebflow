import * as base from "./deckgl.bundle.mjs";

const deckNamespace = base.deckgl || {};
const originalBuildLayer =
  typeof base.buildLayer === "function" ? base.buildLayer.bind(base) : null;
const patchedInstances = new Map();
let patchedTooltipEl = null;
let patchedObserver = null;

function isObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function asArray(value) {
  if (Array.isArray(value)) return value;
  if (value == null) return [];
  return [value];
}

function flattenBuiltLayers(value) {
  return asArray(value).flatMap((entry) => asArray(entry)).filter(Boolean);
}

function withoutUndefined(object) {
  return Object.fromEntries(
    Object.entries(object).filter(([, value]) => value !== undefined),
  );
}

function resolveFeatureAccessor(spec, fallback) {
  if (typeof spec === "function") return spec;
  if (Array.isArray(spec)) return () => spec;
  if (typeof spec === "number" || typeof spec === "boolean") return () => spec;
  if (typeof spec !== "string" || spec.trim() === "") {
    return fallback ? fallback : ((value) => value);
  }
  const trimmed = spec.trim();
  const bracketMatch = trimmed.match(/^\[([^\]]+)\]$/);
  if (bracketMatch) {
    const fields = bracketMatch[1].split(",").map((field) => field.trim());
    return (value) => fields.map((field) => {
      if (/^\d+$/.test(field)) {
        const index = Number(field);
        return Array.isArray(value) ? value[index] : value?.[index];
      }
      return (
        value?.properties?.[field] ??
        value?.[field] ??
        value?.geometry?.[field] ??
        value?.[field]
      );
    });
  }
  return (value) =>
    value?.properties?.[trimmed] ??
    value?.[trimmed] ??
    value?.geometry?.[trimmed];
}

function cloneFeature(feature, geometry) {
  const cloned = isObject(feature) ? { ...feature } : { type: "Feature" };
  cloned.type = "Feature";
  cloned.geometry = geometry;
  cloned.properties = isObject(feature?.properties) ? { ...feature.properties } : {};
  return cloned;
}

function collectGeoJsonFeatures(input, sink = []) {
  if (!input) return sink;
  if (Array.isArray(input)) {
    input.forEach((entry) => collectGeoJsonFeatures(entry, sink));
    return sink;
  }
  if (input.type === "FeatureCollection" && Array.isArray(input.features)) {
    input.features.forEach((feature) => collectGeoJsonFeatures(feature, sink));
    return sink;
  }
  if (input.type === "Feature" && input.geometry) {
    const geometry = input.geometry;
    if (geometry.type === "GeometryCollection" && Array.isArray(geometry.geometries)) {
      geometry.geometries.forEach((entry) =>
        collectGeoJsonFeatures(cloneFeature(input, entry), sink),
      );
      return sink;
    }
    if (geometry.type === "MultiPolygon") {
      geometry.coordinates.forEach((coords) =>
        sink.push(
          cloneFeature(input, {
            type: "Polygon",
            coordinates: coords,
          }),
        ),
      );
      return sink;
    }
    if (geometry.type === "MultiLineString") {
      geometry.coordinates.forEach((coords) =>
        sink.push(
          cloneFeature(input, {
            type: "LineString",
            coordinates: coords,
          }),
        ),
      );
      return sink;
    }
    if (geometry.type === "MultiPoint") {
      geometry.coordinates.forEach((coords) =>
        sink.push(
          cloneFeature(input, {
            type: "Point",
            coordinates: coords,
          }),
        ),
      );
      return sink;
    }
    sink.push(input);
    return sink;
  }
  if (typeof input.type === "string" && input.coordinates) {
    collectGeoJsonFeatures({ type: "Feature", geometry: input, properties: {} }, sink);
  }
  return sink;
}

function buildGeoJsonLayers(cfg) {
  const features = collectGeoJsonFeatures(cfg.data);
  if (!features.length) return [];

  const polygons = [];
  const lines = [];
  const points = [];

  features.forEach((feature) => {
    const geometry = feature?.geometry;
    if (!geometry || !Array.isArray(geometry.coordinates)) return;
    if (geometry.type === "Polygon") polygons.push(feature);
    else if (geometry.type === "LineString") lines.push(feature);
    else if (geometry.type === "Point") points.push(feature);
  });

  const layers = [];
  const baseId = cfg.id || "geojson";
  const filled = cfg.filled !== false;
  const stroked = cfg.stroked !== false;
  const pointType = String(cfg.pointType || "").toLowerCase();

  if (polygons.length && (filled || stroked)) {
    layers.push(
      ...flattenBuiltLayers(
        originalBuildLayer?.({
          id: `${baseId}__polygons`,
          type: "PolygonLayer",
          data: polygons,
          pickable: cfg.pickable,
          filled,
          stroked,
          extruded: cfg.extruded,
          wireframe: cfg.wireframe,
          opacity: cfg.opacity,
          material: cfg.material,
          lineWidthUnits: cfg.lineWidthUnits,
          lineWidthMinPixels: cfg.lineWidthMinPixels,
          lineWidthMaxPixels: cfg.lineWidthMaxPixels,
          getPolygon: (feature) => feature.geometry.coordinates,
          getFillColor: resolveFeatureAccessor(cfg.getFillColor, () => [0, 0, 0, 0]),
          getLineColor: resolveFeatureAccessor(cfg.getLineColor, () => [255, 255, 255, 255]),
          getLineWidth: resolveFeatureAccessor(cfg.getLineWidth, () => 1),
          getElevation: resolveFeatureAccessor(cfg.getElevation, () => 0),
          updateTriggers: cfg.updateTriggers,
        }) || [],
      ),
    );
  }

  if (lines.length && stroked) {
    layers.push(
      ...flattenBuiltLayers(
        originalBuildLayer?.({
          id: `${baseId}__lines`,
          type: "PathLayer",
          data: lines,
          pickable: cfg.pickable,
          widthUnits: cfg.lineWidthUnits,
          widthMinPixels: cfg.lineWidthMinPixels,
          widthMaxPixels: cfg.lineWidthMaxPixels,
          opacity: cfg.opacity,
          getPath: (feature) => feature.geometry.coordinates,
          getColor: resolveFeatureAccessor(cfg.getLineColor, () => [255, 255, 255, 255]),
          getWidth: resolveFeatureAccessor(cfg.getLineWidth, () => 1),
          getDashArray: cfg.getDashArray,
          capRounded: cfg.capRounded,
          jointRounded: cfg.jointRounded,
          billboard: cfg.billboard,
          updateTriggers: cfg.updateTriggers,
        }) || [],
      ),
    );
  }

  if (points.length) {
    const textAccessor = resolveFeatureAccessor(cfg.getText, (feature) =>
      feature?.properties?.name || feature?.properties?.label || "",
    );
    const iconAccessor = resolveFeatureAccessor(cfg.getIcon, () => "marker");
    const sizeAccessor = resolveFeatureAccessor(cfg.getSize, () => 24);
    const colorAccessor = resolveFeatureAccessor(cfg.getColor, () => [255, 255, 255, 255]);
    const textColorAccessor = resolveFeatureAccessor(cfg.getTextColor, () => [255, 255, 255, 255]);
    const textSizeAccessor = resolveFeatureAccessor(cfg.getTextSize, () => 16);
    const angleAccessor = resolveFeatureAccessor(cfg.getAngle || cfg.getTextAngle, () => 0);
    const pointRadiusAccessor = resolveFeatureAccessor(cfg.getPointRadius, () => 3);
    const pointFillAccessor = resolveFeatureAccessor(
      cfg.getPointColor || cfg.getFillColor,
      () => [255, 255, 255, 200],
    );
    const lineColorAccessor = resolveFeatureAccessor(cfg.getLineColor, () => [255, 255, 255, 255]);
    const lineWidthAccessor = resolveFeatureAccessor(cfg.getLineWidth, () => 1);
    const pointRows = points.map((feature) => ({
      __feature: feature,
      position: feature.geometry.coordinates,
      text: textAccessor(feature),
      icon: iconAccessor(feature),
      size: sizeAccessor(feature),
      angle: angleAccessor(feature),
      color: colorAccessor(feature),
      textColor: textColorAccessor(feature),
      textSize: textSizeAccessor(feature),
      radius: pointRadiusAccessor(feature),
      fillColor: pointFillAccessor(feature),
      lineColor: lineColorAccessor(feature),
      lineWidth: lineWidthAccessor(feature),
      properties: feature.properties,
    }));
    const commonPointProps = {
      data: pointRows,
      pickable: cfg.pickable,
      opacity: cfg.opacity,
      updateTriggers: cfg.updateTriggers,
    };
    if (pointType.includes("text") || cfg.getText) {
      layers.push(
        new deckNamespace.TextLayer({
          id: `${baseId}__labels`,
          ...commonPointProps,
          getPosition: (row) => row.position,
          getText: (row) => row.text,
          getColor: (row) => row.textColor,
          getSize: (row) => row.textSize,
          billboard: cfg.billboard !== false,
          pickable: cfg.pickable,
          opacity: cfg.opacity,
          updateTriggers: cfg.updateTriggers,
        }),
      );
    } else if (pointType.includes("icon") || cfg.getIcon || cfg.iconAtlas || cfg.iconMapping) {
      layers.push(
        ...flattenBuiltLayers(
          originalBuildLayer?.({
            id: `${baseId}__icons`,
            type: "IconLayer",
            ...commonPointProps,
            iconAtlas: cfg.iconAtlas,
            iconMapping: cfg.iconMapping,
            sizeScale: cfg.sizeScale,
            sizeMinPixels: cfg.sizeMinPixels,
            sizeMaxPixels: cfg.sizeMaxPixels,
            billboard: cfg.billboard !== false,
            getPosition: "position",
            getIcon: "icon",
            getSize: "size",
            getAngle: "angle",
            getColor: "color",
          }) || [],
        ),
      );
    } else {
      layers.push(
        ...flattenBuiltLayers(
          originalBuildLayer?.({
            id: `${baseId}__points`,
            type: "ScatterplotLayer",
            ...commonPointProps,
            radiusUnits: cfg.pointRadiusUnits,
            radiusMinPixels: cfg.pointRadiusMinPixels,
            radiusMaxPixels: cfg.pointRadiusMaxPixels,
            stroked: cfg.pointStroked,
            filled: cfg.pointFilled !== false,
            lineWidthUnits: cfg.lineWidthUnits,
            lineWidthMinPixels: cfg.lineWidthMinPixels,
            lineWidthMaxPixels: cfg.lineWidthMaxPixels,
            getPosition: "position",
            getFillColor: "fillColor",
            getLineColor: "lineColor",
            getLineWidth: "lineWidth",
            getRadius: "radius",
          }) || [],
        ),
      );
    }
  }

  return layers;
}

function buildTripsLayer(cfg) {
  return flattenBuiltLayers(
    originalBuildLayer?.({
      ...cfg,
      type: "PathLayer",
      widthUnits: cfg.widthUnits || cfg.lineWidthUnits,
      widthMinPixels: cfg.widthMinPixels || cfg.lineWidthMinPixels,
      widthMaxPixels: cfg.widthMaxPixels || cfg.lineWidthMaxPixels,
      getPath:
        cfg.getPath ||
        cfg.getCoords ||
        ((row) => row?.path || row?.coordinates || row?.waypoints || []),
      getColor: cfg.getColor || cfg.getLineColor || (() => [255, 255, 255, 255]),
      getWidth: cfg.getWidth || cfg.getLineWidth || (() => 2),
    }) || [],
  );
}

function buildIconLayer(cfg) {
  if (!deckNamespace.IconLayer) return [];
  return [
    new deckNamespace.IconLayer(
      withoutUndefined({
        ...cfg,
        id: cfg.id,
        data: cfg.data || [],
        pickable: cfg.pickable,
        opacity: cfg.opacity,
        iconAtlas: cfg.iconAtlas,
        iconMapping: cfg.iconMapping,
        sizeScale: cfg.sizeScale,
        sizeMinPixels: cfg.sizeMinPixels,
        sizeMaxPixels: cfg.sizeMaxPixels,
        sizeUnits: cfg.sizeUnits,
        billboard: cfg.billboard,
        updateTriggers: cfg.updateTriggers,
        getPosition: cfg.getPosition,
        getIcon: cfg.getIcon,
        getSize: cfg.getSize,
        getAngle: cfg.getAngle,
        getColor: cfg.getColor,
      }),
    ),
  ];
}

function patchedBuildLayer(cfg) {
  if (!cfg || !cfg.type) return null;
  if (cfg.type === "GeoJsonLayer") return buildGeoJsonLayers(cfg);
  if (cfg.type === "TripsLayer") return buildTripsLayer(cfg);
  if (cfg.type === "IconLayer") return buildIconLayer(cfg);
  return originalBuildLayer ? originalBuildLayer(cfg) : null;
}

function patchedBuildLayers(specs) {
  if (!Array.isArray(specs)) return [];
  return specs.flatMap((cfg) => flattenBuiltLayers(patchedBuildLayer(cfg)));
}

function patchRuntimeRegistry() {
  if (typeof window === "undefined") return;
  const runtime = window.__zebDeck || {};
  const originalGet = typeof runtime.get === "function" ? runtime.get.bind(runtime) : null;
  const originalDeck = runtime.deck || deckNamespace;
  runtime.buildLayer = patchedBuildLayer;
  runtime.buildLayers = patchedBuildLayers;
  runtime.get = (id) => patchedInstances.get(id);
  runtime.deck = {
    ...originalDeck,
    GeoJsonLayer: deckNamespace.GeoJsonLayer || function GeoJsonLayerAdapter(props) {
      return buildGeoJsonLayers(props);
    },
    TripsLayer: deckNamespace.TripsLayer || function TripsLayerAdapter(props) {
      return buildTripsLayer(props);
    },
  };
  window.__zebDeck = runtime;
  ensurePatchedObserver(originalGet);
}

patchRuntimeRegistry();

export function ensureDeck() {
  patchRuntimeRegistry();
  if (typeof window === "undefined") return deckgl;
  return window.__zebDeck?.deck || deckgl;
}

export function buildLayer(cfg) {
  return patchedBuildLayer(cfg);
}

export function buildLayers(specs) {
  return patchedBuildLayers(specs);
}

export function createDeckMapRuntime(host, options = {}) {
  patchRuntimeRegistry();
  return createPatchedDeckMapRuntime(host, options);
}

export function mountDeckMap(container) {
  patchRuntimeRegistry();
  let config = {};
  try {
    config = JSON.parse(container?.dataset?.config || "{}");
  } catch {
    config = {};
  }
  return createPatchedDeckMapRuntime(container, config);
}

export const deckgl = {
  ...(deckNamespace || {}),
  GeoJsonLayer:
    deckNamespace.GeoJsonLayer || function GeoJsonLayerAdapter(props) {
      return buildGeoJsonLayers(props);
    },
  TripsLayer:
    deckNamespace.TripsLayer || function TripsLayerAdapter(props) {
      return buildTripsLayer(props);
    },
};

export const haversine = base.haversine;
export const bearing = base.bearing;
export const colorRamp = base.colorRamp;
export const interpolateAlongPath = base.interpolateAlongPath;
export const createAnimationLoop = base.createAnimationLoop;
export function DeckMap(props) {
  const _h = globalThis.h;
  const _useRef = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = {
    initialViewState: props.initialViewState || {
      longitude: 0,
      latitude: 20,
      zoom: 1.5,
      pitch: 0,
      bearing: 0,
    },
    controller: props.controller !== false,
    layers: props.layers || [],
    stateKey: props.stateKey || null,
    layerKey: props.layerKey || null,
    tooltip: props.tooltip || false,
    background: props.background || "transparent",
  };

  if (_useRef && _useEffect) {
    const hostRef = _useRef(null);
    const instanceRef = _useRef(null);

    _useEffect(() => {
      return () => {
        instanceRef.current?.destroy?.();
        instanceRef.current = null;
      };
    }, []);

    _useEffect(() => {
      instanceRef.current?.setOptions?.(config);
    }, [
      props.background,
      props.controller,
      props.initialViewState,
      props.layerKey,
      props.layers,
      props.stateKey,
      props.tooltip,
    ]);

    const attachHost = (node) => {
      hostRef.current = node;
      if (!node || instanceRef.current || node._zebDeckPatched) return;
      instanceRef.current?.destroy?.();
      instanceRef.current = createPatchedDeckMapRuntime(node, config);
    };

    return _h("div", {
      ref: attachHost,
      id: props.id,
      className: props.className,
      style: { width: "100%", height: props.height || "400px" },
    });
  }

  return _h("div", {
    "data-zeb-lib": "deckgl",
    "data-zeb-wrapper": "DeckMap",
    "data-config": JSON.stringify(config),
    id: props.id,
    class: props.className,
    style: { width: "100%", height: props.height || "400px" },
  });
}

function ensureTooltipEl() {
  if (patchedTooltipEl) return patchedTooltipEl;
  if (typeof document === "undefined") return null;
  patchedTooltipEl = document.createElement("div");
  patchedTooltipEl.className = "zeb-deck-tooltip";
  patchedTooltipEl.style.cssText =
    "position:fixed;pointer-events:none;z-index:9999;padding:6px 10px;" +
    "background:rgba(0,0,0,0.82);color:#fff;font-size:12px;line-height:1.4;" +
    "border-radius:4px;max-width:300px;white-space:pre-wrap;display:none;" +
    "font-family:system-ui,sans-serif;box-shadow:0 2px 8px rgba(0,0,0,0.3);";
  document.body.appendChild(patchedTooltipEl);
  return patchedTooltipEl;
}

function showTooltip(info) {
  const tooltip = ensureTooltipEl();
  if (!tooltip) return;
  if (!info || !info.object) {
    tooltip.style.display = "none";
    return;
  }
  const object = info.object;
  const lines = [];
  if (isObject(object.properties)) {
    Object.entries(object.properties).forEach(([key, value]) => {
      if (typeof value !== "function" && value != null) {
        lines.push(`${key}: ${value}`);
      }
    });
  } else {
    Object.entries(object).forEach(([key, value]) => {
      if (key.startsWith("_")) return;
      if (typeof value === "function") return;
      if (Array.isArray(value) && value.length > 4) return;
      if (isObject(value)) return;
      if (value != null) lines.push(`${key}: ${value}`);
    });
  }
  if (!lines.length) {
    tooltip.style.display = "none";
    return;
  }
  tooltip.textContent = lines.join("\n");
  tooltip.style.display = "block";
  tooltip.style.left = `${(info.x || 0) + 12}px`;
  tooltip.style.top = `${(info.y || 0) + 12}px`;
}

function normalizeOptions(next = {}) {
  return {
    initialViewState: next.initialViewState || {
      longitude: 0,
      latitude: 20,
      zoom: 1.5,
      pitch: 0,
      bearing: 0,
    },
    controller: next.controller !== false,
    layers: Array.isArray(next.layers) ? next.layers : [],
    views: next.views || null,
    stateKey: next.stateKey || null,
    layerKey: next.layerKey || null,
    tooltip: next.tooltip || false,
    background: next.background || "transparent",
    deckProps: isObject(next.deckProps) ? next.deckProps : {},
  };
}

function mergeOptions(prev, next = {}) {
  return normalizeOptions({
    ...prev,
    ...next,
    deckProps: {
      ...(prev?.deckProps || {}),
      ...(next?.deckProps || {}),
    },
  });
}

function applyHostStyles(host, config) {
  host.style.position = "relative";
  if (!host.style.width) host.style.width = "100%";
  if (!host.style.height && !host.style.minHeight) host.style.height = "400px";
  host.style.background =
    config.background && config.background !== "transparent" ? config.background : "";
}

function resolveRuntimeLayers(config) {
  if (config.layerKey && window.__rwePageState?.[config.layerKey] !== undefined) {
    return patchedBuildLayers(window.__rwePageState[config.layerKey]);
  }
  return asArray(config.layers).flatMap((layer) => {
    if (layer && typeof layer.draw === "function") return [layer];
    return flattenBuiltLayers(patchedBuildLayer(layer));
  });
}

function createPatchedDeckMapRuntime(host, options = {}) {
  if (!(host instanceof Element)) {
    throw new Error("zeb/deckgl: host element is required");
  }
  if (!host.id) host.id = `zd-${Math.random().toString(36).slice(2, 8)}`;
  if (patchedInstances.has(host.id)) {
    patchedInstances.get(host.id)?.destroy();
  }

  const instanceId = host.id;
  let stateListener = null;
  let currentViewState = options.initialViewState || {
    longitude: 0,
    latitude: 20,
    zoom: 1.5,
    pitch: 0,
    bearing: 0,
  };
  let currentOptions = normalizeOptions(options);

  applyHostStyles(host, currentOptions);

  function bindStateListener(config, deck) {
    if (stateListener) {
      window.removeEventListener("rwe:state:change", stateListener);
      stateListener = null;
    }
    if (!(config.stateKey || config.layerKey)) return;
    stateListener = (event) => {
      const patch = {};
      if (config.stateKey && event.detail?.[config.stateKey] !== undefined) {
        const nextViewState = event.detail[config.stateKey];
        if (nextViewState && typeof nextViewState === "object") {
          currentViewState = nextViewState;
          patch.initialViewState = nextViewState;
        }
      }
      if (config.layerKey && event.detail?.[config.layerKey] !== undefined) {
        patch.layers = patchedBuildLayers(event.detail[config.layerKey]);
      }
      if (Object.keys(patch).length) deck.setProps(patch);
    };
    window.addEventListener("rwe:state:change", stateListener);
  }

  function buildDeckProps(config) {
    const deckProps = {
      ...config.deckProps,
      controller: config.controller,
      views: config.views || [new deckNamespace.MapView({ repeat: true })],
      initialViewState: currentViewState,
      layers: resolveRuntimeLayers(config),
      parameters: {
        ...(config.deckProps?.parameters || {}),
        ...(config.background === "transparent" ? { clearColor: [0, 0, 0, 0] } : {}),
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

  const deck = new deckNamespace.Deck({
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
      deck.setProps({ layers: resolveRuntimeLayers(currentOptions) });
    },
    setViewState(viewState) {
      currentViewState = viewState;
      deck.setProps({ initialViewState: viewState });
    },
    setOptions(nextOptions = {}) {
      currentOptions = mergeOptions(currentOptions, nextOptions);
      if (nextOptions.initialViewState !== undefined) {
        currentViewState = currentOptions.initialViewState;
      }
      applyHostStyles(host, currentOptions);
      bindStateListener(currentOptions, deck);
      deck.setProps(buildDeckProps(currentOptions));
    },
    destroy() {
      if (stateListener) {
        window.removeEventListener("rwe:state:change", stateListener);
        stateListener = null;
      }
      deck.finalize();
      patchedInstances.delete(instanceId);
      host._zdMounted = false;
      host._zdInstance = null;
      host._zebDeckPatched = false;
      host.innerHTML = "";
    },
  };

  patchedInstances.set(instanceId, instance);
  host._zdMounted = true;
  host._zdInstance = instance;
  host._zebDeckPatched = true;

  host.dispatchEvent(
    new CustomEvent("zeb:deck:ready", {
      bubbles: true,
      detail: { instance, id: instanceId, deck },
    }),
  );

  return instance;
}

function mountExistingHost(host, originalGet) {
  if (!(host instanceof Element)) return;
  if (host._zebDeckPatched) return;
  if (!host.id && host.dataset?.config) {
    host.id = `zd-${Math.random().toString(36).slice(2, 8)}`;
  }
  const staleInstance = patchedInstances.get(host.id) || originalGet?.(host.id);
  staleInstance?.destroy?.();
  host._zdMounted = false;
  host._zdInstance = null;
  host._zebDeckPatched = false;
  host.innerHTML = "";
  mountDeckMap(host);
}

function scanDeckHosts(root, originalGet) {
  if (!(root instanceof Element) && root !== document) return;
  const hosts =
    root === document
      ? document.querySelectorAll('[data-zeb-lib="deckgl"]')
      : root.matches?.('[data-zeb-lib="deckgl"]')
        ? [root]
        : root.querySelectorAll?.('[data-zeb-lib="deckgl"]') || [];
  hosts.forEach((host) => mountExistingHost(host, originalGet));
}

function ensurePatchedObserver(originalGet) {
  if (typeof document === "undefined") return;
  if (!patchedObserver) {
    patchedObserver = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        mutation.addedNodes.forEach((node) => {
          if (node instanceof Element) scanDeckHosts(node, originalGet);
        });
      });
    });
    patchedObserver.observe(document.documentElement || document.body, {
      childList: true,
      subtree: true,
    });
  }
  scanDeckHosts(document, originalGet);
}
