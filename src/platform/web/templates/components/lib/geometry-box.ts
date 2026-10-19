/**
 * The box [west, south, east, north] round GeoJSON geometries: what a map
 * fits its view to.
 *
 * Shared by the database geo preview and the MapServer layer preview. Walks
 * every coordinate of every geometry, GeometryCollection included, and
 * answers null when there is none, so a caller never fits to nothing.
 */

function walk(geometry, add) {
  if (!geometry || typeof geometry !== "object") return;
  if (geometry.type === "GeometryCollection") {
    for (const part of geometry.geometries || []) walk(part, add);
    return;
  }
  const visit = (value) => {
    if (!Array.isArray(value)) return;
    if (typeof value[0] === "number") {
      add(value[0], value[1]);
      return;
    }
    for (const item of value) visit(item);
  };
  visit(geometry.coordinates);
}

/** The box round one geometry or a list of them; null when there is no coordinate. */
export function geometryBox(geometries) {
  let west = Infinity, south = Infinity, east = -Infinity, north = -Infinity;
  const add = (lon, lat) => {
    if (!Number.isFinite(lon) || !Number.isFinite(lat)) return;
    if (lon < west) west = lon;
    if (lon > east) east = lon;
    if (lat < south) south = lat;
    if (lat > north) north = lat;
  };
  for (const geometry of Array.isArray(geometries) ? geometries : [geometries]) walk(geometry, add);
  return west <= east && south <= north ? [west, south, east, north] : null;
}
