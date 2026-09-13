import { geoLabel, geoViewState } from "@/components/db/geo-shape";
import DeckMap from "zeb/deckgl";

/**
 * Spatial preview for any engine that stores geometry.
 *
 * Declared by `capabilities.geo`, so PostGIS through PostgreSQL renders the
 * same preview as sekejap geometry. Nothing here is engine-specific: the
 * input is GeoJSON and the output is a map.
 */

function buildGeoOverlayLayer(geometry) {
  const coords = geometry.coordinates;
  if (geometry.type === "Point") {
    return {
      type: "ScatterplotLayer",
      id: "geo-point",
      data: [{ position: coords }],
      getPosition: "position",
      getFillColor: [255, 106, 0, 180],
      getLineColor: [255, 106, 0, 255],
      getRadius: 200,
      radiusMinPixels: 8,
      stroked: true,
      filled: true,
      lineWidthMinPixels: 2,
    };
  }
  if (geometry.type === "LineString") {
    return {
      type: "PathLayer",
      id: "geo-line",
      data: [{ path: coords }],
      getPath: "path",
      getColor: [255, 106, 0, 240],
      getWidth: 3,
      widthMinPixels: 3,
    };
  }
  if (geometry.type === "Polygon" || geometry.type === "MultiPolygon") {
    return {
      type: "PolygonLayer",
      id: "geo-poly",
      data: [{ polygon: coords }],
      getPolygon: "polygon",
      getFillColor: [255, 106, 0, 100],
      getLineColor: [255, 106, 0, 240],
      getLineWidth: 1,
      lineWidthMinPixels: 2,
      filled: true,
      stroked: true,
    };
  }
  return null;
}


export function parseGeoJsonGeometry(raw) {
  const text = String(raw || "").trim();
  if (!text) return { ok: true, empty: true };
  let parsed = null;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    return {
      ok: false,
      title: "Invalid GeoJSON",
      message: "Geo fields must be valid GeoJSON geometry JSON.",
      example: '{"type":"Point","coordinates":[144.9631,-37.8136]}',
    };
  }
  const validTypes = new Set(["Point", "LineString", "Polygon", "MultiPoint", "MultiLineString", "MultiPolygon", "GeometryCollection"]);
  if (!parsed || typeof parsed !== "object" || !validTypes.has(parsed.type)) {
    return {
      ok: false,
      title: "Invalid GeoJSON Geometry",
      message: "Geo fields need a GeoJSON geometry object with a supported type.",
      example: '{"type":"Point","coordinates":[144.9631,-37.8136]}',
    };
  }
  if (parsed.type === "GeometryCollection") {
    if (!Array.isArray(parsed.geometries)) {
      return {
        ok: false,
        title: "Invalid GeometryCollection",
        message: "GeometryCollection values must include a geometries array.",
        example: '{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[144.9631,-37.8136]}]}',
      };
    }
  } else if (!Array.isArray(parsed.coordinates)) {
    return {
      ok: false,
      title: "Missing Coordinates",
      message: "GeoJSON geometries except GeometryCollection must include a coordinates array.",
      example: '{"type":"Point","coordinates":[144.9631,-37.8136]}',
    };
  }
  return { ok: true, parsed };
}


export default function GeoPreviewMap({ geometry }) {
  if (!geometry || !geometry.coordinates) return null;
  const viewState = geoViewState(geometry);
  const overlay = buildGeoOverlayLayer(geometry);
  const layers = [
    {
      type: "TileLayer",
      id: "osm",
      data: "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
      minZoom: 0,
      maxZoom: 19,
      tileSize: 256,
      renderSubLayers: "bitmap",
    },
  ];
  if (overlay) layers.push(overlay);
  return (
    <div className="space-y-1">
      <div className="overflow-hidden rounded-md border border-border/70">
        <DeckMap
          id="geo-cell-preview"
          height="180px"
          initialViewState={viewState}
          controller={true}
          layers={layers}
        />
      </div>
      <p className="text-[0.68rem] text-muted-foreground">{geoLabel(geometry)}</p>
    </div>
  );
}

