import { cx, useEffect, useMemo, useState } from "zeb/react";
import { World, earth, earthLite, layer } from "zeb/world";

/**
 * The Studio's own map: zeb/world on the Earth with no terrain, over the
 * earth-lite basemap that ships inside Zebflow, so a map draws on an office
 * with no internet and asks nothing from outside. A GIS view: no observation
 * levels, every layer shown and kept.
 *
 * Its two callers are the database geo preview (`components/db/`) and the
 * MapServer layer preview (`pages/.../db/mapserver/`); the only folder they
 * share is the templates root, so it sits with the platform-wide components,
 * beside the map picker.
 *
 * - `elements`: aslnut layers, each with an id, made once by the caller
 *   (`useMemo`); a new object each render counts as changed.
 * - `fit`: a [west, south, east, north] box; the camera jumps to show it
 *   whole whenever it changes. null leaves the camera where it is.
 *
 * The box carries what the world says about itself (`data-world-api`,
 * `data-world-elements`, `data-world-settled`), as aslnut asks a host to show
 * it somewhere a tester can read it.
 */

const EARTH_LITE_URL = "/assets/libraries/zeb/world/earth-lite";
const EARTH_LITE_MAX_ZOOM = 4;
const EARTH_LITE_CREDIT = "Sentinel-2 cloudless 2020 by EOX IT Services GmbH (contains modified Copernicus Sentinel data 2020), CC BY 4.0";
const FIT_MAX_ZOOM = 14;
const SETTLE_MS = 60_000;

export default function StudioMap({ elements, fit, className }) {
  const [w, setW] = useState(null);
  const [seen, setSeen] = useState({ api: "", elements: "", settled: "" });
  const options = useMemo(() => ({
    body: earth({
      imagery: [earthLite({ url: EARTH_LITE_URL, maxZoom: EARTH_LITE_MAX_ZOOM, attribution: EARTH_LITE_CREDIT })],
      terrain: [],
    }),
    view: { lon: 0, lat: 20, zoom: 1.6 },
  }), []);
  const fitKey = Array.isArray(fit) ? fit.join(",") : "";

  useEffect(() => {
    if (!w || w.isDisposed || !Array.isArray(fit)) return;
    w.go(w.viewToFit(fit, { maxZoom: FIT_MAX_ZOOM }));
  }, [w, fitKey]);

  useEffect(() => {
    if (!w || w.isDisposed) return;
    let current = true;
    setSeen((before) => ({ ...before, api: w.api, settled: "" }));
    w.settled({ timeout: SETTLE_MS }).then((ok) => {
      if (!current || w.isDisposed) return;
      setSeen({ api: w.api, elements: w.list.map((item) => item.id).join(" "), settled: ok ? "true" : "false" });
    });
    return () => { current = false; };
  }, [w, fitKey, elements]);

  return (
    <div
      className={cx("relative overflow-hidden", className)}
      data-studio-map="true"
      data-world-api={seen.api}
      data-world-elements={seen.elements}
      data-world-settled={seen.settled}
    >
      <World className="h-full w-full" options={options} elements={elements} onReady={setW} />
    </div>
  );
}

const KIND = {
  Point: "points",
  MultiPoint: "points",
  LineString: "lines",
  MultiLineString: "lines",
  Polygon: "areas",
  MultiPolygon: "areas",
};
const IS_AREA = (d) => KIND[d?.geometry?.type] === "areas";
const IS_LINE = (d) => KIND[d?.geometry?.type] === "lines";
const IS_POINT = (d) => KIND[d?.geometry?.type] === "points";

/**
 * One source drawn as up to three layers, one per kind of geometry
 * (`{id}-areas`, `{id}-lines`, `{id}-points`), each sign kept to its own kind
 * by a filter: a line sign would otherwise stroke every polygon and a points
 * sign mark every shape's middle.
 *
 * `look`: `area` (a fill, with `outline` as a line round it), `line`,
 * `points`, and a `tooltip` shared by all three. A kind left out is not drawn.
 */
export function geometryLayers(id, source, look) {
  const shared = look.tooltip ? { tooltip: look.tooltip } : {};
  const made = [];
  if (look.area) {
    const { outline, ...fill } = look.area;
    made.push(layer({
      id: `${id}-areas`,
      data: source,
      area: { ...fill, filter: IS_AREA },
      ...(outline ? { line: { ...outline, filter: IS_AREA } } : {}),
      ...shared,
    }));
  }
  if (look.line) made.push(layer({ id: `${id}-lines`, data: source, line: { ...look.line, filter: IS_LINE }, ...shared }));
  if (look.points) made.push(layer({ id: `${id}-points`, data: source, points: { ...look.points, filter: IS_POINT }, ...shared }));
  return made;
}
