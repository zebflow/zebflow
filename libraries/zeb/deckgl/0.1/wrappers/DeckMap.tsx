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
  const config = JSON.stringify({
    initialViewState: props.initialViewState,
    controller:       props.controller !== false,
    layers:           props.layers || [],
    stateKey:         props.stateKey || null,
    layerKey:         props.layerKey || null,
    background:       props.background || "transparent",
  });

  return (
    <div
      data-zeb-lib="deckgl"
      data-zeb-wrapper="DeckMap"
      data-config={config}
      id={props.id}
      className={props.className}
      style={{ width: "100%", height: props.height || "400px" }}
    />
  );
}
