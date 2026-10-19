/**
 * Reading a GeoJSON geometry: whether it is one, and what to call it.
 *
 * Lived inside the data grid, which meant the preview panel beside it used
 * these by borrowing them out of the flat bundle rather than importing them —
 * fine until the grid stopped being on the same page. The check lived in the
 * geo preview, which put the map library into every page that only parses a
 * cell; it is here now. Where a map looks is `geometryBox`
 * (`components/lib/geometry-box`).
 */

export function geoLabel(geometry) {
  if (!geometry) return "";
  if (geometry.type === "Point") {
    const [lon, lat] = geometry.coordinates;
    return `Point · ${lat.toFixed(4)}, ${lon.toFixed(4)}`;
  }
  if (geometry.type === "LineString") return `LineString · ${geometry.coordinates.length} vertices`;
  if (geometry.type === "Polygon") return `Polygon · ${geometry.coordinates[0]?.length || 0} vertices`;
  if (geometry.type === "MultiPoint") return `MultiPoint · ${geometry.coordinates.length} points`;
  if (geometry.type === "MultiLineString") return `MultiLineString · ${geometry.coordinates.length} lines`;
  if (geometry.type === "MultiPolygon") return `MultiPolygon · ${geometry.coordinates.length} polygons`;
  return geometry.type;
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
