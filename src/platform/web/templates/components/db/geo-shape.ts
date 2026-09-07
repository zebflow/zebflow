/**
 * Reading a GeoJSON geometry: where a map should look, and what to call it.
 *
 * Lived inside the data grid, which meant the preview panel beside it used
 * these by borrowing them out of the flat bundle rather than importing them —
 * fine until the grid stopped being on the same page.
 */

export function flattenCoordinates(geometry) {
  const coords = [];
  function walk(arr) {
    if (typeof arr[0] === "number") { coords.push(arr); return; }
    for (const item of arr) walk(item);
  }
  if (geometry.coordinates) walk(geometry.coordinates);
  return coords;
}

export function geoViewState(geometry) {
  if (!geometry || !geometry.coordinates) return { longitude: 0, latitude: 0, zoom: 2 };
  if (geometry.type === "Point") {
    return { longitude: geometry.coordinates[0], latitude: geometry.coordinates[1], zoom: 13 };
  }
  const coords = flattenCoordinates(geometry);
  if (!coords.length) return { longitude: 0, latitude: 0, zoom: 2 };
  let minLon = Infinity, maxLon = -Infinity, minLat = Infinity, maxLat = -Infinity;
  for (const [lon, lat] of coords) {
    if (lon < minLon) minLon = lon;
    if (lon > maxLon) maxLon = lon;
    if (lat < minLat) minLat = lat;
    if (lat > maxLat) maxLat = lat;
  }
  const span = Math.max(maxLon - minLon, maxLat - minLat);
  const zoom = span > 10 ? 3 : span > 1 ? 7 : span > 0.1 ? 10 : 13;
  return { longitude: (minLon + maxLon) / 2, latitude: (minLat + maxLat) / 2, zoom };
}

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
