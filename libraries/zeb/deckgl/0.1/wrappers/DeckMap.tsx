/**
 * DeckMap — RWE wrapper for Deck.gl geospatial / data visualisation maps.
 *
 * IMPORT in TSX page:
 *   import DeckMap from "zeb/deckgl";
 *
 * ─── Simple points map ────────────────────────────────────────────────────
 *   <DeckMap
 *     height="400px"
 *     initialViewState={{ longitude: -74.006, latitude: 40.713, zoom: 10 }}
 *     layers={[{
 *       type: "ScatterplotLayer",
 *       id: "points",
 *       data: myPoints,
 *       getPosition: "position",
 *       getFillColor: [0, 180, 255],
 *       getRadius: 80,
 *       pickable: true,
 *     }]}
 *     tooltip={true}
 *   />
 *
 * ─── Reactive layers from page state ─────────────────────────────────────
 *   const [pts, setPts] = usePageState("mapPoints", []);
 *   useEffect(() => { fetch("/api/points").then(r => r.json()).then(setPts); }, []);
 *
 *   <DeckMap
 *     height="500px"
 *     initialViewState={{ longitude: 103.8, latitude: 1.35, zoom: 11 }}
 *     layerKey="mapPoints"    ← data array from page state → ScatterplotLayer auto-built
 *     stateKey="mapView"      ← view state synced two-ways
 *     tooltip={true}
 *   />
 *
 * ─── Heatmap overlay ─────────────────────────────────────────────────────
 *   <DeckMap
 *     height="500px"
 *     initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 11 }}
 *     layers={[{
 *       type: "HeatmapLayer",
 *       data: incidents,
 *       getPosition: "location",
 *       getWeight: "severity",
 *       radiusPixels: 60,
 *       intensity: 2,
 *     }]}
 *   />
 *
 * ─── Imperative access ────────────────────────────────────────────────────
 *   document.getElementById("my-map").addEventListener("zeb:deck:ready", (e) => {
 *     const { deck, instance } = e.detail;
 *     instance.setLayers([
 *       new ArcLayer({ data: arcs, getSourcePosition: d => d.from, ... })
 *     ]);
 *   });
 *
 *   // Or via registry:
 *   const inst = window.__zebDeck.get("my-map");
 *   inst.setViewState({ longitude: 0, latitude: 0, zoom: 2 });
 */
export const app = {};

export default function DeckMap(props) {
  const _h = globalThis.h;
  const _useRef = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = {
    initialViewState: props.initialViewState,
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
      if (typeof globalThis.createDeckMapRuntime !== "function") return;
      instanceRef.current?.destroy?.();
      instanceRef.current = globalThis.createDeckMapRuntime(node, config);
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
