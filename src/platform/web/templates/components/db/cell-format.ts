/**
 * Cell formatting shared by every database engine.
 *
 * These are pure functions over a returned cell value. Nothing here knows
 * which engine produced the row, so the same grid renders sekejap,
 * PostgreSQL, MySQL and SQLite results identically.
 */

export function isGeoJsonPoint(val) {
  return val && typeof val === "object" && val.type === "Point" && Array.isArray(val.coordinates) && val.coordinates.length >= 2;
}

export function formatGeoValue(val) {
  if (isGeoJsonPoint(val)) {
    const [lon, lat] = val.coordinates;
    return `${lat.toFixed(4)}, ${lon.toFixed(4)}`;
  }
  if (val && typeof val === "object" && val.type && val.coordinates) {
    return val.type;
  }
  return null;
}

export function stringifyCell(cell) {
  if (cell === null || typeof cell === "undefined") return "";
  if (typeof cell === "string") return cell;
  if (typeof cell === "number" || typeof cell === "boolean") return String(cell);
  const geo = formatGeoValue(cell);
  if (geo) return geo;
  try {
    return JSON.stringify(cell);
  } catch (_) {
    return String(cell);
  }
}

export function rawCellValue(cell) {
  if (cell === null || typeof cell === "undefined") return "";
  if (typeof cell === "string") return cell;
  if (typeof cell === "number" || typeof cell === "boolean") return String(cell);
  try { return JSON.stringify(cell); } catch (_) { return String(cell); }
}

export function isVectorColumn(vectorFields, colName) {
  return Array.isArray(vectorFields) && vectorFields.includes(colName);
}

export function isGeoColumn(geoFields, colName) {
  return Array.isArray(geoFields) && geoFields.includes(colName);
}

export function vectorArrayFromCell(cell) {
  if (Array.isArray(cell)) return cell;
  if (cell && typeof cell === "object") {
    if (Array.isArray(cell.vector)) return cell.vector;
    if (Array.isArray(cell.embedding)) return cell.embedding;
    if (Array.isArray(cell.values)) return cell.values;
  }
  if (typeof cell === "string") {
    const trimmed = cell.trim();
    if (!trimmed.startsWith("[") || !trimmed.endsWith("]")) return null;
    try {
      const parsed = JSON.parse(trimmed);
      return Array.isArray(parsed) ? parsed : null;
    } catch (_) {
      return null;
    }
  }
  return null;
}

export function isNumericVectorArray(values) {
  return Array.isArray(values) && values.every((item) => typeof item === "number" && Number.isFinite(item));
}

export function formatVectorNumber(value) {
  if (typeof value !== "number" || !Number.isFinite(value)) return String(value);
  if (value === 0) return "0";
  const abs = Math.abs(value);
  if (abs < 0.000001 || abs >= 1000000) {
    return value.toExponential(6).replace(/\.?0+e/, "e");
  }
  return value.toFixed(6).replace(/\.?0+$/, "");
}

export function formatVectorPreview(cell) {
  const values = vectorArrayFromCell(cell);
  if (!isNumericVectorArray(values)) return "";
  if (!values.length) return "[]";
  if (values.length === 1) return `[${formatVectorNumber(values[0])}]`;
  if (values.length === 2) return `[${formatVectorNumber(values[0])}, ${formatVectorNumber(values[1])}]`;
  return `[${formatVectorNumber(values[0])}, ..., ${formatVectorNumber(values[values.length - 1])}]`;
}

export function shouldCompactVectorCell(cell, colName, vectorFields) {
  if (isVectorColumn(vectorFields, colName)) return !!formatVectorPreview(cell);
  const values = vectorArrayFromCell(cell);
  return isNumericVectorArray(values) && values.length >= 8;
}

export function displayCellText(cell, colName, vectorFields) {
  if (isVectorColumn(vectorFields, colName) && (cell === null || typeof cell === "undefined" || cell === "")) return "vector";
  if (shouldCompactVectorCell(cell, colName, vectorFields)) return formatVectorPreview(cell);
  return stringifyCell(cell);
}

export function cellTitleText(cell, colName, vectorFields) {
  if (shouldCompactVectorCell(cell, colName, vectorFields)) return displayCellText(cell, colName, vectorFields);
  return stringifyCell(cell);
}

export function defaultColumnWidth(colName) {
  const name = String(colName || "");
  if (name === "_collection") return 120;
  if (name === "_id") return 160;
  if (name === "_key") return 140;
  if (name === "_created_unix" || name === "_updated_unix") return 170;
  if (name === "position" || name === "location" || name === "coordinates" || name === "geom" || name === "geometry") return 170;
  if (name.startsWith("_")) return 150;
  if (name.length <= 4) return 100;
  if (name.length <= 8) return 140;
  return Math.min(240, 80 + name.length * 10);
}

export function autoSizeColumns(columns, rows, vectorFields) {
  const widths = {};
  columns.forEach((col, colIdx) => {
    // Measure header length
    let maxLen = col.length;
    // Sample first 30 rows for content width
    const sampleCount = Math.min(rows.length, 30);
    for (let i = 0; i < sampleCount; i++) {
      const cell = Array.isArray(rows[i]) ? rows[i][colIdx] : undefined;
      const text = displayCellText(cell, col, vectorFields);
      if (text.length > maxLen) maxLen = text.length;
    }
    // Estimate width: ~8px per char + padding, clamped
    const estimated = Math.max(70, Math.min(360, maxLen * 8.2 + 28));
    // Use the larger of default or estimated
    widths[col] = Math.max(defaultColumnWidth(col), estimated);
  });
  return widths;
}
