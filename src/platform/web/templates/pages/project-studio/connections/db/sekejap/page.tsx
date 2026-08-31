import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { useEffect, useState, useRef, cx } from "zeb";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import { Select, SelectOption } from "@/components/ui/select";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import MapPicker from "@/components/ui/map-picker";
import DeckMap from "zeb/deckgl";

export const page = {
  head: {
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-suite.css" },
      { rel: "stylesheet", href: "/assets/libraries/zeb/icons/0.1/runtime/devicons.css" },
    ],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

const INDEX_OPTIONS_BY_KIND = {
  string: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
    { id: "fulltext", label: "Fulltext" },
  ],
  number: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  boolean: [{ id: "hash", label: "Exact" }],
  json: [],
  vector: [{ id: "vector", label: "Index" }],
  geo: [{ id: "spatial", label: "Index" }],
};

const DEFAULT_ATTRIBUTE = {
  name: "",
  kind: "string",
  index_types: [],
};

function requestJson(url, options = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (response) => {
    if (response.status === 401) {
      window.location.href = "/login";
      return null;
    }
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const message =
        payload?.error?.message || payload?.message || `${response.status} ${response.statusText}`;
      throw new Error(message);
    }
    return payload;
  });
}

function normalizeSchemaNodes(nodes) {
  return (Array.isArray(nodes) ? nodes : [])
    .map((node) => String(node?.name || ""))
    .filter((name) => name && !name.startsWith("_"))
    .sort((a, b) => a.localeCompare(b));
}

function normalizeTableNodes(nodes) {
  return (Array.isArray(nodes) ? nodes : [])
    .filter((node) => String(node?.kind || "") === "table")
    .map((node) => {
      const schema = String(node?.schema || "default");
      const table = String(node?.name || "");
      const key = schema === "default" ? table : `${schema}.${table}`;
      return {
        schema,
        table,
        key,
        rowCount: Number(node?.meta?.row_count || 0),
        attributes: Array.isArray(node?.meta?.attributes) ? node.meta.attributes : [],
        hashIndexed: Array.isArray(node?.meta?.hash_indexed_fields) ? node.meta.hash_indexed_fields : [],
        rangeIndexed: Array.isArray(node?.meta?.range_indexed_fields) ? node.meta.range_indexed_fields : [],
        fulltextFields: Array.isArray(node?.meta?.fulltext_fields) ? node.meta.fulltext_fields : [],
        vectorFields: Array.isArray(node?.meta?.vector_fields) ? node.meta.vector_fields : [],
        spatialFields: Array.isArray(node?.meta?.spatial_fields) ? node.meta.spatial_fields : [],
      };
    })
    .filter((item) => item.schema && item.table && !item.schema.startsWith("_"))
    .sort((a, b) => a.key.localeCompare(b.key));
}

function isGeoJsonPoint(val) {
  return val && typeof val === "object" && val.type === "Point" && Array.isArray(val.coordinates) && val.coordinates.length >= 2;
}

function formatGeoValue(val) {
  if (isGeoJsonPoint(val)) {
    const [lon, lat] = val.coordinates;
    return `${lat.toFixed(4)}, ${lon.toFixed(4)}`;
  }
  if (val && typeof val === "object" && val.type && val.coordinates) {
    return val.type;
  }
  return null;
}

function stringifyCell(cell) {
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

function rawCellValue(cell) {
  if (cell === null || typeof cell === "undefined") return "";
  if (typeof cell === "string") return cell;
  if (typeof cell === "number" || typeof cell === "boolean") return String(cell);
  try { return JSON.stringify(cell); } catch (_) { return String(cell); }
}

function isVectorColumn(vectorFields, colName) {
  return Array.isArray(vectorFields) && vectorFields.includes(colName);
}

function isGeoColumn(geoFields, colName) {
  return Array.isArray(geoFields) && geoFields.includes(colName);
}

function vectorArrayFromCell(cell) {
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

function isNumericVectorArray(values) {
  return Array.isArray(values) && values.every((item) => typeof item === "number" && Number.isFinite(item));
}

function formatVectorNumber(value) {
  if (typeof value !== "number" || !Number.isFinite(value)) return String(value);
  if (value === 0) return "0";
  const abs = Math.abs(value);
  if (abs < 0.000001 || abs >= 1000000) {
    return value.toExponential(6).replace(/\.?0+e/, "e");
  }
  return value.toFixed(6).replace(/\.?0+$/, "");
}

function formatVectorPreview(cell) {
  const values = vectorArrayFromCell(cell);
  if (!isNumericVectorArray(values)) return "";
  if (!values.length) return "[]";
  if (values.length === 1) return `[${formatVectorNumber(values[0])}]`;
  if (values.length === 2) return `[${formatVectorNumber(values[0])}, ${formatVectorNumber(values[1])}]`;
  return `[${formatVectorNumber(values[0])}, ..., ${formatVectorNumber(values[values.length - 1])}]`;
}

function shouldCompactVectorCell(cell, colName, vectorFields) {
  if (isVectorColumn(vectorFields, colName)) return !!formatVectorPreview(cell);
  const values = vectorArrayFromCell(cell);
  return isNumericVectorArray(values) && values.length >= 8;
}

function displayCellText(cell, colName, vectorFields) {
  if (isVectorColumn(vectorFields, colName) && (cell === null || typeof cell === "undefined" || cell === "")) return "vector";
  if (shouldCompactVectorCell(cell, colName, vectorFields)) return formatVectorPreview(cell);
  return stringifyCell(cell);
}

function cellTitleText(cell, colName, vectorFields) {
  if (shouldCompactVectorCell(cell, colName, vectorFields)) return displayCellText(cell, colName, vectorFields);
  return stringifyCell(cell);
}

function defaultColumnWidth(colName) {
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

function autoSizeColumns(columns, rows, vectorFields) {
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

function ResizableDataGrid({ columns, rows, selectedRowKey, onRowSelect, onCellInspect, mapRowToObject, editingCell, pendingEdits, onEditingCellChange, onCellEdit, vectorFields, geoFields, onGeoPick }) {
  const [colWidths, setColWidths] = useState({});
  const [sortCol, setSortCol] = useState(null);
  const [sortDir, setSortDir] = useState("asc");
  const dragRef = useRef(null);
  const editInputRef = useRef(null);

  // Auto-size widths when columns change
  useEffect(() => {
    setColWidths(autoSizeColumns(columns, rows, vectorFields));
  }, [columns.join(","), rows.length, (vectorFields || []).join(",")]);

  // Reset sort when columns change
  useEffect(() => { setSortCol(null); }, [columns.join(",")]);

  useEffect(() => {
    if (editingCell && editInputRef.current) {
      editInputRef.current.focus();
      editInputRef.current.select();
    }
  }, [editingCell?.rowIndex, editingCell?.colIndex]);

  function onResizeStart(e, colIndex) {
    e.preventDefault();
    e.stopPropagation();
    const col = columns[colIndex];
    const startX = e.clientX;
    const startW = colWidths[col] || defaultColumnWidth(col);

    function onMove(ev) {
      const delta = ev.clientX - startX;
      const nextW = Math.max(48, startW + delta);
      setColWidths((prev) => ({ ...prev, [col]: nextW }));
    }
    function onUp() {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      dragRef.current = null;
    }
    dragRef.current = col;
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }

  function onHeaderClick(colIndex) {
    if (dragRef.current) return;
    const col = columns[colIndex];
    if (sortCol === col) {
      setSortDir((prev) => prev === "asc" ? "desc" : "asc");
    } else {
      setSortCol(col);
      setSortDir("asc");
    }
  }

  // Sort rows
  const sortedRows = (() => {
    if (sortCol === null) return rows;
    const colIdx = columns.indexOf(sortCol);
    if (colIdx < 0) return rows;
    const copy = [...rows];
    copy.sort((a, b) => {
      const av = Array.isArray(a) ? a[colIdx] : undefined;
      const bv = Array.isArray(b) ? b[colIdx] : undefined;
      const as = displayCellText(av, sortCol, vectorFields);
      const bs = displayCellText(bv, sortCol, vectorFields);
      // Try numeric comparison first
      const an = Number(as);
      const bn = Number(bs);
      if (!isNaN(an) && !isNaN(bn)) {
        return sortDir === "asc" ? an - bn : bn - an;
      }
      const cmp = as.localeCompare(bs);
      return sortDir === "asc" ? cmp : -cmp;
    });
    return copy;
  })();

  if (!columns.length) return null;

  return (
    <table className="w-full border-collapse project-table" style={{ width: "max-content", minWidth: "100%" }}>
      <thead className="bg-surface-2">
        <tr>
          {columns.map((col, index) => {
            const isSorted = sortCol === col;
            return (
              <th
                key={`${col}-${index}`}
                className="relative px-[0.65rem] py-[0.4rem] border-b border-border-soft text-left text-[0.68rem] font-mono uppercase tracking-[0.12em] text-body-soft select-none cursor-pointer hover:text-body"
                style={{ width: colWidths[col] || defaultColumnWidth(col), minWidth: 48, maxWidth: 600 }}
                onClick={() => onHeaderClick(index)}
              >
                <span className="flex items-center gap-1 overflow-hidden whitespace-nowrap">
                  <span className="overflow-hidden text-ellipsis">{col}</span>
                  {isSorted && (
                    <svg viewBox="0 0 10 10" fill="currentColor" className="w-2.5 h-2.5 shrink-0 opacity-70">
                      {sortDir === "asc"
                        ? <path d="M5 2L9 8H1Z" />
                        : <path d="M5 8L1 2H9Z" />
                      }
                    </svg>
                  )}
                </span>
                <div
                  className="absolute top-0 right-0 w-[5px] h-full cursor-col-resize group"
                  onMouseDown={(ev) => onResizeStart(ev, index)}
                >
                  <div className="absolute top-1 bottom-1 right-[2px] w-[1px] bg-border-soft opacity-0 hover:opacity-100 transition-opacity" />
                </div>
              </th>
            );
          })}
        </tr>
      </thead>
      <tbody>
        {sortedRows.map((row, rowIndex) => {
          const record = mapRowToObject(columns, row);
          const rowKey = String(record?._key || "");
          const isSelected = selectedRowKey && rowKey === selectedRowKey;
          const rowPending = pendingEdits?.[rowKey] || {};
          return (
            <tr key={`row-${rowIndex}`} className={isSelected ? "is-row-selected" : ""}>
              {(Array.isArray(row) ? row : []).map((cell, cellIndex) => {
                const colName = columns[cellIndex] || `column_${cellIndex + 1}`;
                const isSystemCol = colName.startsWith("_");
                const isEditing = editingCell && editingCell.rowIndex === rowIndex && editingCell.colIndex === cellIndex;
                const hasPending = colName in rowPending;
                const displayValue = hasPending ? rowPending[colName] : cell;
                const isVectorCol = isVectorColumn(vectorFields, colName);
                const isGeoCol = isGeoColumn(geoFields, colName);
                const compactVector = shouldCompactVectorCell(displayValue, colName, vectorFields);
                return (
                  <td
                    key={`cell-${rowIndex}-${cellIndex}`}
                    className={`px-[0.65rem] border-b border-border-soft text-left text-[0.78rem] text-body cursor-pointer whitespace-nowrap overflow-hidden text-ellipsis ${isEditing ? "p-0" : "py-[0.35rem]"} ${hasPending ? "bg-amber-500/10" : ""}`}
                    style={{ maxWidth: colWidths[colName] || defaultColumnWidth(colName) }}
                    title={isEditing ? undefined : cellTitleText(displayValue, colName, vectorFields)}
                    onClick={() => {
                      onRowSelect(rowKey, record);
                      onCellInspect(colName, rowIndex, cell);
                    }}
                    onDoubleClick={() => {
                      if (!isSystemCol && !isVectorCol && rowKey && onEditingCellChange) {
                        onEditingCellChange({ rowIndex, colIndex: cellIndex, rowKey, colName });
                      }
                    }}
                  >
                    {isEditing ? (
                      <input
                        ref={editInputRef}
                        type="text"
                        className="w-full border-0 bg-ui-bg px-[0.65rem] py-[0.35rem] text-[0.78rem] text-body outline-none ring-1 ring-inset ring-blue-500"
                        defaultValue={rawCellValue(displayValue)}
                        onBlur={(e) => {
                          const val = e.target.value;
                          if (val !== rawCellValue(cell)) {
                            onCellEdit(rowKey, colName, val);
                          }
                          onEditingCellChange(null);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") e.target.blur();
                          if (e.key === "Escape") {
                            e.target.value = rawCellValue(cell);
                            onEditingCellChange(null);
                          }
                          if (e.key === "Tab") {
                            e.preventDefault();
                            const val = e.target.value;
                            if (val !== rawCellValue(cell)) {
                              onCellEdit(rowKey, colName, val);
                            }
                            let nextCol = cellIndex + (e.shiftKey ? -1 : 1);
                            while (nextCol >= 0 && nextCol < columns.length && columns[nextCol]?.startsWith("_")) {
                              nextCol += e.shiftKey ? -1 : 1;
                            }
                            if (nextCol >= 0 && nextCol < columns.length) {
                              onEditingCellChange({ rowIndex, colIndex: nextCol, rowKey, colName: columns[nextCol] });
                            } else {
                              onEditingCellChange(null);
                            }
                          }
                        }}
                      />
                    ) : (
                      <span className={cx("inline-flex max-w-full items-center gap-1", isGeoCol ? "w-full" : "")}>
                        <span className="min-w-0 overflow-hidden text-ellipsis">
                          {isGeoJsonPoint(displayValue) ? (
                            <span className="inline-flex items-center gap-1">
                              <svg viewBox="0 0 12 12" fill="none" className="w-3 h-3 shrink-0 opacity-50">
                                <path d="M6 1C4.067 1 2.5 2.567 2.5 4.5C2.5 7.25 6 11 6 11s3.5-3.75 3.5-6.5C9.5 2.567 7.933 1 6 1Zm0 4.75a1.25 1.25 0 110-2.5 1.25 1.25 0 010 2.5Z" fill="currentColor"/>
                              </svg>
                              <span>{formatGeoValue(displayValue)}</span>
                            </span>
                          ) : compactVector ? (
                            <span className="font-mono text-[0.73rem] text-[#ebbe7d]">
                              {formatVectorPreview(displayValue)}
                            </span>
                          ) : (displayValue == null || displayValue === "") && isVectorCol ? (
                            <span className="inline-flex items-center gap-1 text-[#e9904e]">
                              <svg viewBox="0 0 12 12" fill="none" className="w-3 h-3 shrink-0">
                                <circle cx="3" cy="6" r="1.5" fill="currentColor" opacity="0.6"/>
                                <circle cx="6" cy="3" r="1.5" fill="currentColor" opacity="0.8"/>
                                <circle cx="9" cy="6" r="1.5" fill="currentColor" opacity="0.6"/>
                                <circle cx="6" cy="9" r="1.5" fill="currentColor" opacity="0.4"/>
                              </svg>
                              <span className="text-[11px]">vector</span>
                            </span>
                          ) : (displayValue == null || displayValue === "") && isGeoCol ? (
                            <span className="text-[11px] text-[#e9904e]">geo</span>
                          ) : stringifyCell(displayValue)}
                        </span>
                        {isGeoCol && !isSystemCol ? (
                          <button
                            type="button"
                            title="Pick geometry on map"
                            className="ml-auto inline-flex h-5 w-5 shrink-0 items-center justify-center rounded border border-ui-border/70 bg-ui-bg text-ui-text-soft hover:border-[#f6863c] hover:text-[#f6863c]"
                            onClick={(event) => {
                              event.preventDefault();
                              event.stopPropagation();
                              onRowSelect(rowKey, record);
                              onCellInspect(colName, rowIndex, cell);
                              onGeoPick?.({ rowKey, colName, value: displayValue, rowIndex, colIndex: cellIndex, record });
                            }}
                          >
                            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5">
                              <path d="M8 1.75c-2.07 0-3.75 1.68-3.75 3.75 0 2.95 3.75 8.75 3.75 8.75s3.75-5.8 3.75-8.75c0-2.07-1.68-3.75-3.75-3.75Z"/>
                              <circle cx="8" cy="5.5" r="1.25"/>
                            </svg>
                          </button>
                        ) : null}
                      </span>
                    )}
                  </td>
                );
              })}
            </tr>
          );
        })}
      </tbody>
      <tfoot>
        <tr>
          <td
            colSpan={columns.length}
            className="px-[0.65rem] py-[0.3rem] text-[0.68rem] text-body-muted border-t border-border-soft bg-surface-2/50"
          >
            {sortedRows.length} rows{sortCol ? ` · sorted by ${sortCol} ${sortDir}` : ""}
          </td>
        </tr>
      </tfoot>
    </table>
  );
}

function prettyValue(raw) {
  const text = String(raw || "").trim();
  if (!text) return "";
  if (!text.startsWith("{") && !text.startsWith("[")) return text;
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch (_) {
    return text;
  }
}

function isGeoJsonGeometry(val) {
  return val && typeof val === "object" && val.type && val.coordinates;
}

function flattenCoordinates(geometry) {
  const coords = [];
  function walk(arr) {
    if (typeof arr[0] === "number") { coords.push(arr); return; }
    for (const item of arr) walk(item);
  }
  if (geometry.coordinates) walk(geometry.coordinates);
  return coords;
}

function geoViewState(geometry) {
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

function geoLabel(geometry) {
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

function GeoPreviewMap({ geometry }) {
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
      <div className="overflow-hidden rounded-md border border-ui-border/70">
        <DeckMap
          id="geo-cell-preview"
          height="180px"
          initialViewState={viewState}
          controller={true}
          layers={layers}
        />
      </div>
      <p className="text-[0.68rem] text-ui-text-soft">{geoLabel(geometry)}</p>
    </div>
  );
}

function sqlStringLiteral(value) {
  return String(value || "").replace(/'/g, "''");
}

function mapRowToObject(columns, row) {
  const output = {};
  (Array.isArray(columns) ? columns : []).forEach((column, index) => {
    output[String(column || `column_${index + 1}`)] = Array.isArray(row) ? row[index] : undefined;
  });
  return output;
}

function relationNodeSlug(record, fallbackCollection = "") {
  const explicit = String(record?._id || record?.slug || "").trim();
  if (explicit) return explicit;
  const collection = String(record?._collection || fallbackCollection || "").trim();
  const key = String(record?._key || "").trim();
  return collection && key ? `${collection}/${key}` : "";
}

function relationNodeLabel(record, fallbackCollection = "") {
  return (
    String(record?.title || "").trim() ||
    String(record?.fullname || "").trim() ||
    String(record?.full_name || "").trim() ||
    String(record?.name || "").trim() ||
    String(record?.label || "").trim() ||
    String(record?.post_id || "").trim() ||
    String(record?._key || "").trim() ||
    relationNodeSlug(record, fallbackCollection)
  );
}

function normalizeRelationType(value) {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9_]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function orderedDataColumns(columns) {
  const trailing = ["_created_unix", "_updated_unix"];
  const seen = new Set();
  const unique = (columns || []).map(String).filter((name) => {
    if (!name || seen.has(name)) return false;
    seen.add(name);
    return true;
  });
  return [
    ...unique.filter((name) => !trailing.includes(name)),
    ...trailing.filter((name) => unique.includes(name)),
  ];
}

function reorderRowsForColumns(sourceColumns, rows, targetColumns) {
  const indexByName = new Map((sourceColumns || []).map((name, index) => [String(name), index]));
  return (rows || []).map((row) =>
    targetColumns.map((name) => {
      const sourceIndex = indexByName.get(name);
      return Array.isArray(row) && sourceIndex !== undefined ? row[sourceIndex] : null;
    })
  );
}

function fieldKindForColumn(table, colName) {
  const name = String(colName || "");
  const attr = (table?.attributes || []).find((item) => String(item?.name || "") === name);
  if (attr?.kind) return String(attr.kind);
  if ((table?.spatialFields || []).includes(name)) return "geo";
  if ((table?.vectorFields || []).includes(name)) return "vector";
  return "";
}

function parseGeoJsonGeometry(raw) {
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

function validateCellEditValue(table, colName, value) {
  const kind = fieldKindForColumn(table, colName);
  const text = String(value ?? "").trim();
  if (!text) return null;
  if (kind === "geo") {
    const geo = parseGeoJsonGeometry(text);
    if (!geo.ok) {
      return {
        title: geo.title,
        message: `${geo.message} Column: ${colName}. Coordinates use [longitude, latitude].`,
        example: geo.example,
      };
    }
  }
  if (kind === "number" && Number.isNaN(Number(text))) {
    return {
      title: "Invalid Number",
      message: `Column ${colName} expects a number. Use plain numeric values such as 12, 12.5, or -3.`,
      example: "12.5",
    };
  }
  if (kind === "json") {
    try {
      JSON.parse(text);
    } catch (_) {
      return {
        title: "Invalid JSON",
        message: `Column ${colName} expects valid JSON.`,
        example: '{"status":"active","tags":["demo"]}',
      };
    }
  }
  if (kind === "vector") {
    try {
      const parsed = JSON.parse(text);
      if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === "number" && Number.isFinite(item))) {
        throw new Error("vector must be numeric array");
      }
    } catch (_) {
      return {
        title: "Invalid Vector",
        message: `Column ${colName} expects a JSON array of numbers.`,
        example: "[0.12, 0.34, 0.56]",
      };
    }
  }
  return null;
}

function relationSlugParts(slug) {
  const text = String(slug || "").trim();
  const [collection, key, ...rest] = text.split("/");
  if (!collection || !key || rest.length) return null;
  return { collection, key };
}

function uniqueRelationDefs(defs) {
  const seen = new Set();
  return (defs || [])
    .map((item) => ({
      from: String(item?.from || "").trim(),
      to: String(item?.to || "").trim(),
      type: String(item?.type || "").trim(),
    }))
    .filter((item) => item.from && item.to && item.type)
    .filter((item) => {
      const key = `${item.from}:${item.type}:${item.to}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

function relationCountFromRows(rows) {
  const first = Array.isArray(rows) && Array.isArray(rows[0]) ? rows[0][0] : null;
  const count = Number(first);
  return Number.isFinite(count) ? count : null;
}

function groupTablesBySchema(tables) {
  const map = new Map();
  (tables || []).forEach((item) => {
    if (!map.has(item.schema)) {
      map.set(item.schema, []);
    }
    map.get(item.schema).push(item);
  });
  return map;
}

function selectedTableDefinition(tables, selectedTable) {
  return (tables || []).find((item) => item.key === selectedTable) || null;
}

function formatBytes(value) {
  const bytes = Number(value || 0);
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let amount = bytes;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  const fixed = amount >= 100 || unit === 0 ? 0 : amount >= 10 ? 1 : 2;
  return `${amount.toFixed(fixed)} ${units[unit]}`;
}

function maintenanceDelta(report, key) {
  if (!report?.before || !report?.after) return "";
  const before = Number(report.before?.[key] || 0);
  const after = Number(report.after?.[key] || 0);
  const diff = after - before;
  if (!diff) return "";
  return `${diff > 0 ? "+" : "-"}${formatBytes(Math.abs(diff))}`;
}

function indexBadgesForAttribute(tableItem, attrName) {
  const badges = [];
  if ((tableItem?.hashIndexed || []).includes(attrName)) badges.push({ key: "hash", label: "exact" });
  if ((tableItem?.rangeIndexed || []).includes(attrName)) badges.push({ key: "range", label: "range" });
  if ((tableItem?.fulltextFields || []).includes(attrName)) badges.push({ key: "fulltext", label: "fulltext" });
  if ((tableItem?.vectorFields || []).includes(attrName)) badges.push({ key: "vector", label: "vector" });
  if ((tableItem?.spatialFields || []).includes(attrName)) badges.push({ key: "spatial", label: "geo" });
  return badges;
}

function SekejapMaintenancePanel({ health, report, busy, status, onRefresh, onSync, onCompact }) {
  const walBytes = Number(health?.wal_bytes || 0);
  const shouldCompact = walBytes >= 64 * 1024 * 1024;
  const statItems = [
    { label: "Nodes", value: Number(health?.node_count || 0).toLocaleString() },
    { label: "Edges", value: Number(health?.edge_count || 0).toLocaleString() },
    { label: "WAL", value: formatBytes(health?.wal_bytes), delta: maintenanceDelta(report, "wal_bytes") },
    { label: "Snapshot", value: formatBytes(health?.snapshot_bytes), delta: maintenanceDelta(report, "snapshot_bytes") },
    { label: "Payload", value: formatBytes(health?.payload_bytes), delta: maintenanceDelta(report, "payload_bytes") },
    { label: "Indexes", value: formatBytes(health?.sidecar_bytes), delta: maintenanceDelta(report, "sidecar_bytes") },
    { label: "Check Time", value: `${Number(health?.duration_ms || 0)} ms` },
    { label: "Root", value: health?.root ? String(health.root) : "Not loaded", mono: true },
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-4">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-ui-text">Sekejap Store Maintenance</p>
          <p className="mt-1 max-w-2xl text-xs text-ui-text-soft">
            Inspect the project-local store, flush pending WAL writes, and compact the snapshot during low-traffic windows.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onRefresh}>
            Refresh
          </Button>
          <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onSync}>
            Sync WAL
          </Button>
          <Button type="button" size="sm" disabled={busy} onClick={onCompact}>
            Compact
          </Button>
        </div>
      </div>

      {status ? (
        <div
          className={cx(
            "mb-4 rounded-md border px-3 py-2 text-xs",
            status.startsWith("Error")
              ? "border-red-300/70 bg-red-50/50 text-red-700 dark:border-red-800/60 dark:bg-red-950/20 dark:text-red-400"
              : "border-ui-border/80 bg-ui-bg-muted/30 text-ui-text-soft"
          )}
        >
          {status}
        </div>
      ) : null}

      {shouldCompact ? (
        <div className="mb-4 rounded-md border border-amber-300/70 bg-amber-50/50 px-3 py-2 text-xs text-amber-800 dark:border-amber-800/60 dark:bg-amber-950/20 dark:text-amber-300">
          WAL is above 64 MB. Compacting will checkpoint the store into a clean snapshot and truncate replay data.
        </div>
      ) : null}

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        {statItems.map((item) => (
          <div key={item.label} className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/20 p-3">
            <p className="text-[0.68rem] font-medium uppercase tracking-[0.14em] text-ui-text-soft">{item.label}</p>
            <p className={cx("mt-1 truncate text-sm font-medium text-ui-text", item.mono ? "font-mono text-[0.72rem]" : "")} title={item.value}>
              {item.value}
            </p>
            {item.delta ? (
              <p className="mt-1 text-[0.68rem] tabular-nums text-ui-text-soft">{item.delta}</p>
            ) : null}
          </div>
        ))}
      </div>

      {report ? (
        <div className="mt-4 rounded-lg border border-ui-border/80 bg-ui-bg-muted/10 p-3">
          <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Last Operation</p>
          <p className="mt-2 text-sm text-ui-text">
            {report.operation} completed in {Number(report.duration_ms || 0)} ms.
          </p>
        </div>
      ) : null}
    </div>
  );
}

function AttributeEditorRow({ item, onChange, onRemove }) {
  const options = INDEX_OPTIONS_BY_KIND[item?.kind] || [];

  return (
    <div className="flex min-w-0 flex-col gap-3 rounded-lg border border-ui-border/80 bg-ui-bg-muted/40 p-3">
      <div className="grid min-w-0 gap-3 md:grid-cols-[minmax(0,1fr)_11rem]">
        <Input
          className="min-w-0"
          value={item?.name || ""}
          onInput={(event) => onChange({ ...item, name: event?.target?.value || "" })}
          placeholder="field_name"
        />
        <Select className="min-w-0" value={item?.kind || "string"} onChange={(event) => onChange({ ...item, kind: event?.target?.value || "string", index_types: [] })}>
          {Object.keys(INDEX_OPTIONS_BY_KIND).map((kind) => (
            <SelectOption key={kind} value={kind} label={kind} />
          ))}
        </Select>
      </div>
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <div className="flex min-h-9 min-w-0 flex-1 flex-wrap items-center gap-3 rounded-md border border-dashed border-ui-border px-3 py-2">
          {options.length ? (
            options.map((option) => {
              const checked = Array.isArray(item?.index_types) && item.index_types.includes(option.id);
              return (
                <label key={option.id} className="inline-flex items-center gap-2 text-xs text-ui-text-soft">
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={(event) => {
                      const next = new Set(Array.isArray(item?.index_types) ? item.index_types : []);
                      if (event?.target?.checked) {
                        next.add(option.id);
                      } else {
                        next.delete(option.id);
                      }
                      onChange({ ...item, index_types: Array.from(next) });
                    }}
                  />
                  <span>{option.label}</span>
                </label>
              );
            })
          ) : (
            <span className="text-xs text-ui-text-muted">No index</span>
          )}
        </div>
        <Button type="button" variant="ghost" size="sm" className="shrink-0" onClick={onRemove}>
          Remove
        </Button>
      </div>
    </div>
  );
}

function CreateTableDialog({
  open,
  onOpenChange,
  tableSlug,
  setTableSlug,
  attributes,
  setAttributes,
  status,
  busy,
  onSubmit,
}) {
  function updateAttribute(index, nextValue) {
    setAttributes((prev) => prev.map((item, itemIndex) => (itemIndex === index ? nextValue : item)));
  }

  function removeAttribute(index) {
    setAttributes((prev) => prev.filter((_, itemIndex) => itemIndex !== index));
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="wide" className="border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Create Table</DialogTitle>
          <p className="text-sm text-body-soft">
            Define a sekejap table and its attributes. Index options change based on the selected kind.
          </p>
          <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : status.startsWith("Created") ? "text-success" : "text-body-soft")}>
            {status}
          </p>
        </DialogHeader>

        <form
          onSubmit={onSubmit}
          className="flex flex-col gap-4 px-6 py-4"
        >
          <Field label="Table Slug">
            <Input
              value={tableSlug}
              onInput={(event) => setTableSlug(event?.target?.value || "")}
              placeholder="posts"
              required
              disabled={busy}
            />
          </Field>

          <Field label="Attributes">
            <div className="flex flex-col gap-3">
              {attributes.map((item, index) => (
                <AttributeEditorRow
                  key={`attr-${index}`}
                  item={item}
                  onChange={(nextValue) => updateAttribute(index, nextValue)}
                  onRemove={() => removeAttribute(index)}
                />
              ))}
              <div className="flex items-center justify-between gap-3 rounded-lg border border-dashed border-ui-border px-3 py-2">
                <p className="text-xs text-ui-text-soft">Add only the attributes you want to predeclare. Sekejap still accepts dynamic JSON payloads.</p>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setAttributes((prev) => [...prev, { ...DEFAULT_ATTRIBUTE }])}
                  disabled={busy}
                >
                  Add Attribute
                </Button>
              </div>
            </div>
          </Field>

          <DialogFooter>
            <Button type="button" variant="ghost" size="sm" onClick={() => onOpenChange(false)} disabled={busy}>
              Cancel
            </Button>
            <Button type="submit" size="sm" disabled={busy}>
              {busy ? "Creating…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function StructureTable({ activeTable, schemaColumns, schemaRows, schemaError, emptyMessage = "No declared or inferred structure available yet." }) {
  if (schemaRows.length) {
    return (
      <StudioTable>
        <StudioThead>
          <tr>
            {schemaColumns.map((col, index) => (
              <StudioTh key={`scol-${col}-${index}`}>{col}</StudioTh>
            ))}
          </tr>
        </StudioThead>
        <tbody>
          {schemaRows.map((row, rowIndex) => (
            <tr key={`srow-${rowIndex}`}>
              {(Array.isArray(row) ? row : []).map((cell, cellIndex) => (
                <StudioTd key={`scell-${rowIndex}-${cellIndex}`}>{stringifyCell(cell)}</StudioTd>
              ))}
            </tr>
          ))}
        </tbody>
      </StudioTable>
    );
  }

  if (schemaError) {
    return <div className="db-suite-empty">Failed to load structure: {schemaError}</div>;
  }

  if (activeTable?.attributes?.length) {
    return (
      <StudioTable>
        <StudioThead>
          <tr>
            <StudioTh>Name</StudioTh>
            <StudioTh>Kind</StudioTh>
            <StudioTh>Indexes</StudioTh>
          </tr>
        </StudioThead>
        <tbody>
          {activeTable.attributes.map((attr, index) => {
            const badges = indexBadgesForAttribute(activeTable, attr.name);
            return (
              <tr key={`${attr.name}-${index}`}>
                <StudioTd>{attr.name}</StudioTd>
                <StudioTd>{attr.kind || "string"}</StudioTd>
                <StudioTd>
                  <div className="flex flex-wrap gap-2">
                    {badges.length ? (
                      badges.map((badge) => (
                        <span key={badge.key} className="inline-flex rounded-full border border-ui-border px-2 py-0.5 text-[11px] text-ui-text-soft">
                          {badge.label}
                        </span>
                      ))
                    ) : (
                      <span className="text-ui-text-muted">—</span>
                    )}
                  </div>
                </StudioTd>
              </tr>
            );
          })}
        </tbody>
      </StudioTable>
    );
  }

  return <div className="db-suite-empty">{emptyMessage}</div>;
}

function RelationDialog({
  open,
  onOpenChange,
  busy,
  status,
  direction,
  setDirection,
  relationType,
  setRelationType,
  relatedNodeSlug,
  setRelatedNodeSlug,
  currentNodeSlug,
  relationTypeOptions,
  relatedSlugWarning,
  onOpenTargetSearch,
  onSubmit,
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Create Relation</DialogTitle>
          <p className="text-sm text-body-soft">
            Link the current node to another node in the Sekejap store.
          </p>
          <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : status.startsWith("Created") ? "text-success" : "text-body-soft")}>
            {status}
          </p>
        </DialogHeader>

        <form onSubmit={onSubmit} className="flex flex-col gap-4 px-6 py-4">
          <Field label="Current Node">
            <Input value={currentNodeSlug} disabled />
          </Field>

          <div className="grid gap-3 md:grid-cols-2">
            <Field label="Direction">
              <Select value={direction} onChange={(event) => setDirection(event?.target?.value || "outgoing")} disabled={busy}>
                <SelectOption value="outgoing" label="Current -> related" />
                <SelectOption value="incoming" label="Related -> current" />
              </Select>
            </Field>
            <Field label="Relation Type">
              <div className="flex flex-col gap-2">
                {relationTypeOptions?.length ? (
                  <Select value={relationTypeOptions.includes(relationType) ? relationType : ""} onChange={(event) => setRelationType(event?.target?.value || "")} disabled={busy}>
                    <SelectOption value="" label="New or custom type" />
                    {relationTypeOptions.map((type) => (
                      <SelectOption key={type} value={type} label={type} />
                    ))}
                  </Select>
                ) : null}
                <Input
                  value={relationType}
                  onInput={(event) => setRelationType(event?.target?.value || "")}
                  placeholder="references"
                  required
                  disabled={busy}
                />
              </div>
            </Field>
          </div>

          <Field label={direction === "outgoing" ? "Target Node Slug" : "Source Node Slug"}>
            <div className="flex gap-2">
              <Input
                value={relatedNodeSlug}
                onInput={(event) => setRelatedNodeSlug(event?.target?.value || "")}
                placeholder="people/alice"
                required
                disabled={busy}
              />
              <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onOpenTargetSearch}>
                Search
              </Button>
            </div>
            {relatedSlugWarning ? (
              <p className="mt-1 text-xs text-amber-500">{relatedSlugWarning}</p>
            ) : null}
          </Field>

          <p className="text-xs text-body-soft">
            Use the Sekejap node slug format: <span className="font-mono">collection/key</span>.
          </p>

          <DialogFooter>
            <Button type="button" variant="ghost" size="sm" onClick={() => onOpenChange(false)} disabled={busy}>
              Cancel
            </Button>
            <Button type="submit" size="sm" disabled={busy}>
              {busy ? "Creating…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function DataWarningDialog({ notice, onClose }) {
  return (
    <Dialog open={!!notice} onOpenChange={(value) => { if (!value) onClose(); }}>
      <DialogContent className="max-w-xl border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>{notice?.title || "Invalid Input"}</DialogTitle>
          <p className="text-sm text-body-soft">{notice?.message || ""}</p>
        </DialogHeader>
        {notice?.example ? (
          <div className="px-6 py-2">
            <p className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-body-soft">Example</p>
            <pre className="overflow-auto rounded-md border border-ui-border/70 bg-ui-bg-muted/30 p-3 text-xs text-body">{notice.example}</pre>
          </div>
        ) : null}
        <DialogFooter className="px-6 pb-6">
          <Button type="button" size="sm" onClick={onClose}>OK</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RelationTargetSearchDialog({ open, onOpenChange, tables, onSearch, onSelect }) {
  const [collection, setCollection] = useState("");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState([]);
  const [status, setStatus] = useState("Choose a collection and search existing nodes.");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    const first = tables?.[0]?.table || "";
    setCollection((current) => current || first);
    setQuery("");
    setResults([]);
    setStatus("Choose a collection and search existing nodes.");
  }, [open, tables?.length]);

  async function runSearch() {
    if (!collection) return;
    setBusy(true);
    setStatus("Searching…");
    try {
      const items = await onSearch(collection, query);
      setResults(items);
      setStatus(`${items.length} node(s) found.`);
    } catch (error) {
      setResults([]);
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="wide" className="border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Search Node</DialogTitle>
          <p className="text-sm text-body-soft">Select an existing node for the relation target.</p>
          <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : "text-body-soft")}>{status}</p>
        </DialogHeader>
        <div className="flex flex-col gap-3 px-6 py-4">
          <div className="grid gap-3 md:grid-cols-[12rem_minmax(0,1fr)_auto]">
            <Select value={collection} onChange={(event) => setCollection(event?.target?.value || "")} disabled={busy}>
              {(tables || []).map((table) => (
                <SelectOption key={table.table} value={table.table} label={table.table} />
              ))}
            </Select>
            <Input value={query} onInput={(event) => setQuery(event?.target?.value || "")} placeholder="Search by _key, title, name, slug…" disabled={busy} />
            <Button type="button" size="sm" disabled={busy || !collection} onClick={runSearch}>Search</Button>
          </div>
          <div className="max-h-96 overflow-auto rounded-md border border-ui-border/70">
            {results.length ? (
              <StudioTable>
                <StudioThead>
                  <tr>
                    <StudioTh>Node</StudioTh>
                    <StudioTh>Label</StudioTh>
                    <StudioTh></StudioTh>
                  </tr>
                </StudioThead>
                <tbody>
                  {results.map((item) => (
                    <tr key={item.slug}>
                      <StudioTd>{item.slug}</StudioTd>
                      <StudioTd>{item.label}</StudioTd>
                      <StudioTd>
                        <Button type="button" variant="outline" size="sm" onClick={() => { onSelect(item.slug); onOpenChange(false); }}>
                          Use
                        </Button>
                      </StudioTd>
                    </tr>
                  ))}
                </tbody>
              </StudioTable>
            ) : (
              <p className="px-3 py-6 text-center text-sm text-ui-text-soft">No nodes loaded yet.</p>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function RelationStatsList({ title, items, emptyText, peerKey }) {
  return (
    <div className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/10 p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">{title}</p>
        <span className="text-xs text-ui-text-soft">{items?.length || 0}</span>
      </div>
      {items?.length ? (
        <div className="space-y-2">
          {items.map((item, index) => (
            <div key={`${title}-${item.type}-${item.from}-${item.to}-${index}`} className="grid grid-cols-[minmax(0,1fr)_auto] gap-3 rounded-md border border-ui-border/70 bg-ui-bg px-3 py-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium text-ui-text">{item.type}</p>
                <p className="truncate text-xs text-ui-text-soft">{peerKey === "to" ? `to ${item.to}` : `from ${item.from}`}</p>
                {item.countError ? (
                  <p className="mt-1 text-[11px] text-amber-500">Count unavailable</p>
                ) : null}
              </div>
              <div className="text-right">
                <p className="font-mono text-sm font-semibold tabular-nums text-ui-text">
                  {item.count === null || item.count === undefined ? "n/a" : Number(item.count).toLocaleString()}
                </p>
                <p className="text-[11px] uppercase tracking-[0.12em] text-ui-text-soft">edges</p>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-ui-text-soft">{emptyText}</p>
      )}
    </div>
  );
}

function RowRelationList({ title, items, emptyText, onDelete }) {
  return (
    <div className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/10 p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">{title}</p>
        <span className="text-xs text-ui-text-soft">{items?.length || 0}</span>
      </div>
      {items?.length ? (
        <div className="space-y-2">
          {items.map((entry, index) => (
            <div key={`${title}-${entry.type}-${entry.otherSlug}-${index}`} className="rounded-md border border-ui-border/70 bg-ui-bg px-3 py-2">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="text-sm font-medium text-ui-text">{entry.type}</p>
                  <p className="truncate text-xs text-ui-text-soft">{entry.otherLabel}</p>
                  <p className="truncate text-[11px] text-ui-text-muted">{entry.otherSlug}</p>
                </div>
                <Button type="button" variant="ghost" size="sm" onClick={() => onDelete(entry)}>
                  Delete
                </Button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-ui-text-soft">{emptyText}</p>
      )}
    </div>
  );
}

export default function Page(input) {
  const navLinks = input?.nav?.links ?? {};
  const suiteTabs = Array.isArray(input?.suite_tabs) ? input.suite_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const preview = input?.preview ?? { columns: [], rows: [], empty: true };
  const connection = input?.connection ?? {};
  const dbApi = input?.db_runtime_api ?? {};
  const projectApiBase = `/api/projects/${encodeURIComponent(input?.owner || "")}/${encodeURIComponent(input?.project || "")}`;
  const simpleTablesApi = `${projectApiBase}/tables`;
  const sekejapSchemaExportApi = `${simpleTablesApi}/schema/export`;
  const sekejapSchemaSyncApi = `${simpleTablesApi}/schema/sync`;
  const sekejapMaintenanceApi = `${projectApiBase}/db/sekejap/maintenance`;
  const initialTable = typeof window !== "undefined" ? new URLSearchParams(window.location.search).get("table") || "" : "";

  const [schemas, setSchemas] = useState([]);
  const [tables, setTables] = useState([]);
  const [selectedTable, setSelectedTable] = useState(initialTable);
  const [collapsedSchemas, setCollapsedSchemas] = useState({});
  const [treeError, setTreeError] = useState("");
  const [previewColumns, setPreviewColumns] = useState(Array.isArray(preview?.columns) ? preview.columns : []);
  const [previewRows, setPreviewRows] = useState(Array.isArray(preview?.rows) ? preview.rows : []);
  const [previewError, setPreviewError] = useState("");
  const [querySql, setQuerySql] = useState(String(input?.query_example || "SHOW TABLES"));
  const [queryStatus, setQueryStatus] = useState("Ready");
  const [queryColumns, setQueryColumns] = useState(Array.isArray(preview?.columns) ? preview.columns : []);
  const [queryRows, setQueryRows] = useState(Array.isArray(preview?.rows) ? preview.rows : []);
  const [schemaColumns, setSchemaColumns] = useState([]);
  const [schemaRows, setSchemaRows] = useState([]);
  const [schemaError, setSchemaError] = useState("");
  const [valueMeta, setValueMeta] = useState("Click a cell to inspect value");
  const [valueBody, setValueBody] = useState("");
  const [inspectedCellRaw, setInspectedCellRaw] = useState(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createBusy, setCreateBusy] = useState(false);
  const [createStatus, setCreateStatus] = useState("Define the table and save it into the project-local sekejap store.");
  const [createTableSlug, setCreateTableSlug] = useState("");
  const [createAttributes, setCreateAttributes] = useState([{ ...DEFAULT_ATTRIBUTE }]);
  const [reloadToken, setReloadToken] = useState(0);
  const [selectedPreviewRowKey, setSelectedPreviewRowKey] = useState("");
  const [selectedPreviewRowData, setSelectedPreviewRowData] = useState(null);
  const [outgoingRelations, setOutgoingRelations] = useState([]);
  const [incomingRelations, setIncomingRelations] = useState([]);
  const [relationsBusy, setRelationsBusy] = useState(false);
  const [relationsError, setRelationsError] = useState("");
  const [outgoingRelationStats, setOutgoingRelationStats] = useState([]);
  const [incomingRelationStats, setIncomingRelationStats] = useState([]);
  const [relationStatsBusy, setRelationStatsBusy] = useState(false);
  const [relationStatsError, setRelationStatsError] = useState("");
  const [relationCreateOpen, setRelationCreateOpen] = useState(false);
  const [relationCreateBusy, setRelationCreateBusy] = useState(false);
  const [relationCreateStatus, setRelationCreateStatus] = useState("Choose the direction, relation type, and related node slug.");
  const [relationDirection, setRelationDirection] = useState("outgoing");
  const [relationType, setRelationType] = useState("");
  const [relatedNodeSlug, setRelatedNodeSlug] = useState("");
  const [relationTypeOptions, setRelationTypeOptions] = useState([]);
  const [relationTargetSearchOpen, setRelationTargetSearchOpen] = useState(false);
  const [validationNotice, setValidationNotice] = useState(null);
  const [pendingRelationDelete, setPendingRelationDelete] = useState(null);
  const [contentTab, setContentTab] = useState("data");
  const [propsAttributes, setPropsAttributes] = useState([]);
  const [propsBusy, setPropsBusy] = useState(false);
  const [propsStatus, setPropsStatus] = useState("");
  const [schemaSyncBusy, setSchemaSyncBusy] = useState(false);
  const [schemaSyncStatus, setSchemaSyncStatus] = useState("");
  const [maintenanceHealth, setMaintenanceHealth] = useState(null);
  const [maintenanceReport, setMaintenanceReport] = useState(null);
  const [maintenanceBusy, setMaintenanceBusy] = useState(false);
  const [maintenanceStatus, setMaintenanceStatus] = useState("");
  const [pendingMaintenanceAction, setPendingMaintenanceAction] = useState("");
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [deleteConfirmInput, setDeleteConfirmInput] = useState("");
  const [deleteBusy, setDeleteBusy] = useState(false);
  const grouped = groupTablesBySchema(tables);
  const schemaNames = (schemas.length ? schemas : Array.from(grouped.keys())).sort((a, b) => a.localeCompare(b));
  const activeTable = selectedTableDefinition(tables, selectedTable);
  const declaredCols = (activeTable?.attributes || []).map((a) => String(a.name || "")).filter(Boolean);
  const extraCols = declaredCols.filter((name) => !previewColumns.includes(name));
  const rawMergedColumns = [...previewColumns, ...extraCols];
  const rawMergedRows = extraCols.length
    ? previewRows.map((row) => [...(Array.isArray(row) ? row : []), ...extraCols.map(() => null)])
    : previewRows;
  const mergedColumns = orderedDataColumns(rawMergedColumns);
  const mergedRows = reorderRowsForColumns(rawMergedColumns, rawMergedRows, mergedColumns);

  async function loadTreeData(preferredTable = "") {
    const [schemasPayload, tablesPayload] = await Promise.all([
      requestJson(dbApi.schemas),
      requestJson(dbApi.tables),
    ]);
    const nextSchemas = normalizeSchemaNodes(schemasPayload?.result?.nodes);
    const nextTables = normalizeTableNodes(tablesPayload?.result?.nodes);
    setSchemas(nextSchemas);
    setTables(nextTables);
    setTreeError("");

    const requested = String(preferredTable || initialTable || "").trim();
    const first = nextTables[0]?.key || "";
    const target = nextTables.some((item) => item.key === requested) ? requested : first;
    setSelectedTable(target);
  }

  useEffect(() => {
    if (!dbApi.schemas || !dbApi.tables) return;
    let active = true;
    loadTreeData()
      .catch((error) => {
        if (!active) return;
        setSchemas([]);
        setTables([]);
        setTreeError(`Failed to load tables: ${String(error?.message || error)}`);
      });
    return () => {
      active = false;
    };
  }, [dbApi.schemas, dbApi.tables, reloadToken]);

  useEffect(() => {
    loadMaintenanceHealth({ silent: true });
  }, [sekejapMaintenanceApi, reloadToken]);

  async function loadPreviewData(table) {
    if (!dbApi.preview || !table) return { columns: [], rows: [] };
    const url = `${dbApi.preview}?table=${encodeURIComponent(table)}&limit=120`;
    const payload = await requestJson(url);
    const result = payload?.result || {};
    const columns = Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : [];
    const rows = Array.isArray(result?.rows) ? result.rows : [];
    setPreviewColumns(columns);
    setPreviewRows(rows);
    setPreviewError("");
    return { columns, rows };
  }

  useEffect(() => {
    if (!dbApi.preview) return;
    if (!selectedTable) {
      setPreviewColumns([]);
      setPreviewRows([]);
      setPreviewError("");
      if (typeof window !== "undefined") {
        const next = new URL(window.location.href);
        next.searchParams.delete("table");
        window.history.replaceState({}, "", next.toString());
      }
      return;
    }
    let active = true;
    loadPreviewData(selectedTable).catch((error) => {
      if (!active) return;
      setPreviewColumns([]);
      setPreviewRows([]);
      setPreviewError(String(error?.message || error));
    });

    if (typeof window !== "undefined") {
      const next = new URL(window.location.href);
      next.searchParams.set("table", selectedTable);
      window.history.replaceState({}, "", next.toString());
    }

    return () => {
      active = false;
    };
  }, [dbApi.preview, selectedTable]);

  useEffect(() => {
    if (!activeTable || !mergedRows.length) {
      setSelectedPreviewRowKey("");
      setSelectedPreviewRowData(null);
      return;
    }

    const records = mergedRows.map((row) => mapRowToObject(mergedColumns, row));
    const existing = records.find((record) => String(record?._key || "") === selectedPreviewRowKey);
    const chosen = existing || records[0] || null;
    if (!chosen) {
      setSelectedPreviewRowKey("");
      setSelectedPreviewRowData(null);
      return;
    }
    setSelectedPreviewRowKey(String(chosen?._key || ""));
    setSelectedPreviewRowData(chosen);
  }, [activeTable, previewColumns, previewRows, selectedPreviewRowKey]);

  useEffect(() => {
    if (!dbApi.query || !selectedTable) return;
    let active = true;
    const tableName = selectedTable.split(".").pop() || selectedTable;
    requestJson(dbApi.query, {
      method: "POST",
      body: JSON.stringify({
        sql: `SHOW ${tableName}`,
        read_only: true,
        limit: 500,
        table: tableName,
      }),
    })
      .then((payload) => {
        if (!active) return;
        const result = payload?.result || {};
        setSchemaColumns(Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : []);
        setSchemaRows(Array.isArray(result?.rows) ? result.rows : []);
        setSchemaError("");
      })
      .catch((error) => {
        if (!active) return;
        setSchemaColumns([]);
        setSchemaRows([]);
        setSchemaError(String(error?.message || error));
      });
    return () => {
      active = false;
    };
  }, [dbApi.query, selectedTable]);

  function onCellInspect(columnName, rowIndex, cellValue) {
    setValueMeta(`${columnName} · row ${rowIndex + 1}`);
    setValueBody(prettyValue(rawCellValue(cellValue)));
    setInspectedCellRaw(cellValue);
  }

  async function runDbQuery(sql, { readOnly = true, tableName = "", limit = 500 } = {}) {
    if (!dbApi.query) {
      throw new Error("Query endpoint is not available");
    }
    const payload = await requestJson(dbApi.query, {
      method: "POST",
      body: JSON.stringify({
        sql,
        read_only: readOnly,
        limit,
        ...(tableName ? { table: tableName } : {}),
      }),
    });
    const result = payload?.result || {};
    const columns = Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : [];
    const rows = Array.isArray(result?.rows) ? result.rows : [];
    return {
      columns,
      rows,
      objects: rows.map((row) => mapRowToObject(columns, row)),
      result,
    };
  }

  async function loadRelationSummaryForTable(tableName) {
    if (!dbApi.query || !tableName) {
      setOutgoingRelationStats([]);
      setIncomingRelationStats([]);
      setRelationStatsError("");
      return;
    }

    setRelationStatsBusy(true);
    setRelationStatsError("");
    try {
      const show = await runDbQuery("SHOW EDGES", { readOnly: true, tableName, limit: 500 });
      const edgeDefs = uniqueRelationDefs(show.objects);
      const allTypes = edgeDefs
        .map((item) => item.type)
        .filter((item, index, arr) => item && arr.indexOf(item) === index)
        .sort((a, b) => a.localeCompare(b));
      setRelationTypeOptions(allTypes);

      const outgoingDefs = edgeDefs.filter((item) => item.from === tableName);
      const incomingDefs = edgeDefs.filter((item) => item.to === tableName);

      async function withCount(def, direction) {
        const query = direction === "outgoing"
          ? `SELECT COUNT(*) AS count FROM MATCH (a:${def.from})-[:${def.type}]->(b:${def.to})`
          : `SELECT COUNT(*) AS count FROM MATCH (a:${def.from})-[:${def.type}]->(b:${def.to})`;
        try {
          const counted = await runDbQuery(query, { readOnly: true, tableName, limit: 1 });
          return { ...def, direction, count: relationCountFromRows(counted.rows), countError: "" };
        } catch (error) {
          return { ...def, direction, count: null, countError: String(error?.message || error) };
        }
      }

      const [outgoing, incoming] = await Promise.all([
        Promise.all(outgoingDefs.map((def) => withCount(def, "outgoing"))),
        Promise.all(incomingDefs.map((def) => withCount(def, "incoming"))),
      ]);
      setOutgoingRelationStats(outgoing.sort((a, b) => `${a.type}:${a.to}`.localeCompare(`${b.type}:${b.to}`)));
      setIncomingRelationStats(incoming.sort((a, b) => `${a.type}:${a.from}`.localeCompare(`${b.type}:${b.from}`)));
    } catch (error) {
      setOutgoingRelationStats([]);
      setIncomingRelationStats([]);
      setRelationStatsError(String(error?.message || error));
    } finally {
      setRelationStatsBusy(false);
    }
  }

  async function loadRelationsForNode(tableName, record) {
    const nodeKey = String(record?._key || "").trim();
    if (!tableName || !nodeKey) {
      setOutgoingRelations([]);
      setIncomingRelations([]);
      setRelationsError("");
      return;
    }

    setRelationsBusy(true);
    setRelationsError("");
    try {
      const show = await runDbQuery("SHOW EDGES", { readOnly: true, tableName, limit: 500 });
      const edgeDefs = uniqueRelationDefs(show.objects);
      setRelationTypeOptions(
        edgeDefs
          .map((item) => String(item?.type || "").trim())
          .filter(Boolean)
          .filter((item, index, arr) => arr.indexOf(item) === index)
          .sort((a, b) => a.localeCompare(b))
      );

      const outgoingTypes = edgeDefs
        .filter((item) => String(item?.from || "") === tableName)
        .map((item) => String(item?.type || "").trim())
        .filter(Boolean)
        .filter((item, index, arr) => arr.indexOf(item) === index);

      const incomingEdges = edgeDefs
        .filter((item) => String(item?.to || "") === tableName)
        .map((item) => ({
          from: String(item?.from || "").trim(),
          type: String(item?.type || "").trim(),
        }))
        .filter((item) => item.from && item.type)
        .filter((item, index, arr) => arr.findIndex((other) => other.from === item.from && other.type === item.type) === index);

      const escapedKey = sqlStringLiteral(nodeKey);

      const outgoingLists = await Promise.all(
        outgoingTypes.map(async (type) => {
          const query = `SELECT b._collection AS _collection, b._key AS _key, b.title AS title, b.name AS name, b.post_id AS post_id, b.slug AS slug FROM MATCH (a:${tableName})-[:${type}]->(b) WHERE a._key = '${escapedKey}'`;
          const response = await runDbQuery(query, { readOnly: true, tableName, limit: 200 });
          return response.objects.map((target) => ({
            direction: "outgoing",
            type,
            other: target,
            otherSlug: relationNodeSlug(target, ""),
            otherLabel: relationNodeLabel(target, ""),
          }));
        })
      );

      const incomingLists = await Promise.all(
        incomingEdges.map(async ({ from, type }) => {
          const query = `SELECT a._collection AS _collection, a._key AS _key, a.title AS title, a.name AS name, a.post_id AS post_id, a.slug AS slug FROM MATCH (a:${from})-[:${type}]->(b:${tableName}) WHERE b._key = '${escapedKey}'`;
          const response = await runDbQuery(query, { readOnly: true, tableName, limit: 200 });
          return response.objects.map((source) => ({
            direction: "incoming",
            type,
            other: source,
            otherSlug: relationNodeSlug(source, ""),
            otherLabel: relationNodeLabel(source, ""),
          }));
        })
      );

      setOutgoingRelations(
        outgoingLists
          .flat()
          .filter((item) => item.otherSlug)
          .sort((a, b) => `${a.type}:${a.otherLabel}`.localeCompare(`${b.type}:${b.otherLabel}`))
      );
      setIncomingRelations(
        incomingLists
          .flat()
          .filter((item) => item.otherSlug)
          .sort((a, b) => `${a.type}:${a.otherLabel}`.localeCompare(`${b.type}:${b.otherLabel}`))
      );
    } catch (error) {
      setOutgoingRelations([]);
      setIncomingRelations([]);
      setRelationsError(String(error?.message || error));
    } finally {
      setRelationsBusy(false);
    }
  }

  async function runQuery() {
    if (!dbApi.query) return;
    const sql = String(querySql || "").trim();
    if (!sql) {
      setQueryStatus("Error · Query is empty");
      return;
    }

    const payload = {
      sql,
      read_only: true,
      limit: 1000,
      ...(selectedTable ? { table: selectedTable.split(".").pop() || selectedTable } : {}),
    };

    setQueryStatus("Running...");
    try {
      const response = await requestJson(dbApi.query, {
        method: "POST",
        body: JSON.stringify(payload),
      });
      const result = response?.result || {};
      setQueryColumns(Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : []);
      setQueryRows(Array.isArray(result?.rows) ? result.rows : []);
      setQueryStatus(`OK · rows ${Number(result?.row_count || 0)} · ${Number(result?.duration_ms || 0)} ms`);
    } catch (error) {
      setQueryColumns([]);
      setQueryRows([]);
      setQueryStatus(`Error · ${String(error?.message || error)}`);
    }
  }

  function resetCreateForm(open) {
    setCreateOpen(open);
    if (open) {
      setCreateStatus("Define the table and save it into the project-local sekejap store.");
      setCreateTableSlug("");
      setCreateAttributes([{ ...DEFAULT_ATTRIBUTE }]);
    }
  }

  async function handleCreateTable(event) {
    event?.preventDefault?.();
    const table = String(createTableSlug || "").trim();
    const attributes = (createAttributes || [])
      .map((item) => ({
        name: String(item?.name || "").trim(),
        kind: String(item?.kind || "string"),
        index_types: Array.isArray(item?.index_types) ? item.index_types : [],
      }))
      .filter((item) => item.name);

    if (!table) {
      setCreateStatus("Error · Table slug is required.");
      return;
    }

    setCreateBusy(true);
    setCreateStatus("Creating table…");
    try {
      const payload = await requestJson(simpleTablesApi, {
        method: "POST",
        body: JSON.stringify({
          table,
          attributes,
        }),
      });
      const createdTable = String(payload?.table?.table || table).trim();
      setCreateStatus(`Created · ${createdTable}`);
      setCreateOpen(false);
      await loadTreeData(createdTable);
      setQueryStatus(`Created table '${createdTable}'.`);
    } catch (error) {
      setCreateStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setCreateBusy(false);
    }
  }

  function openRelationDialog() {
    setRelationCreateStatus("Choose the direction, relation type, and related node slug.");
    setRelationDirection("outgoing");
    setRelationType("");
    setRelatedNodeSlug("");
    setRelationCreateOpen(true);
  }

  async function handleCreateRelation(event) {
    event?.preventDefault?.();
    const currentNodeSlug = relationNodeSlug(selectedPreviewRowData, activeTable?.table || "");
    const currentKey = String(selectedPreviewRowData?._key || "").trim();
    const normalizedType = normalizeRelationType(relationType);
    const otherSlug = String(relatedNodeSlug || "").trim();
    const otherParts = relationSlugParts(otherSlug);

    if (!activeTable?.table || !currentNodeSlug || !currentKey) {
      setRelationCreateStatus("Error · Select a concrete row first.");
      return;
    }
    if (!normalizedType) {
      setRelationCreateStatus("Error · Relation type is required.");
      return;
    }
    if (!otherParts) {
      setValidationNotice({
        title: "Invalid Node Slug",
        message: "Related node slug must use collection/key format.",
        example: "people/alice",
      });
      setRelationCreateStatus("Error · Related node slug must use collection/key.");
      return;
    }
    if (!tables.some((table) => table.table === otherParts.collection)) {
      setValidationNotice({
        title: "Unknown Collection",
        message: `Collection '${otherParts.collection}' does not exist in this Sekejap store. Choose an existing collection before creating the relation.`,
        example: `${activeTable.table}/${currentKey}`,
      });
      setRelationCreateStatus(`Error · Unknown collection '${otherParts.collection}'.`);
      return;
    }

    const fromSlug = relationDirection === "outgoing" ? currentNodeSlug : otherSlug;
    const toSlug = relationDirection === "outgoing" ? otherSlug : currentNodeSlug;
    const sql = `INSERT ('${sqlStringLiteral(fromSlug)}')-[:${normalizedType}]->('${sqlStringLiteral(toSlug)}')`;

    setRelationCreateBusy(true);
    setRelationCreateStatus("Creating relation…");
    try {
      const exists = await runDbQuery(`SELECT _key FROM ${otherParts.collection} WHERE _key = '${sqlStringLiteral(otherParts.key)}'`, {
        readOnly: true,
        tableName: otherParts.collection,
        limit: 1,
      });
      if (!exists.rows.length) {
        setValidationNotice({
          title: "Target Node Not Found",
          message: `Node '${otherSlug}' does not exist. Search and select an existing node, or create the node first.`,
          example: `${otherParts.collection}/existing_key`,
        });
        setRelationCreateStatus(`Error · Node '${otherSlug}' not found.`);
        return;
      }
      await runDbQuery(sql, { readOnly: false, tableName: activeTable.table, limit: 50 });
      setRelationCreateStatus(`Created · ${normalizedType}`);
      setRelationCreateOpen(false);
      await loadRelationsForNode(activeTable.table, selectedPreviewRowData);
    } catch (error) {
      setRelationCreateStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setRelationCreateBusy(false);
    }
  }

  async function handleDeleteRelation(entry) {
    if (!entry || !activeTable?.table || !selectedPreviewRowData) return;
    const currentNodeSlug = relationNodeSlug(selectedPreviewRowData, activeTable.table);
    if (!currentNodeSlug) return;
    const fromSlug = entry.direction === "outgoing" ? currentNodeSlug : entry.otherSlug;
    const toSlug = entry.direction === "outgoing" ? entry.otherSlug : currentNodeSlug;
    const sql = `DELETE ('${sqlStringLiteral(fromSlug)}')-[:${entry.type}]->('${sqlStringLiteral(toSlug)}')`;
    try {
      await runDbQuery(sql, { readOnly: false, tableName: activeTable.table, limit: 50 });
      await loadRelationsForNode(activeTable.table, selectedPreviewRowData);
    } catch (error) {
      setRelationsError(String(error?.message || error));
    }
  }

  // Sync properties form when active table changes
  useEffect(() => {
    if (!activeTable) return;
    setPropsAttributes(
      (activeTable.attributes || []).map((a) => ({
        name: a.name || "",
        kind: a.kind || "string",
        index_types: Array.isArray(a.index_types) ? [...a.index_types] : [],
      }))
    );
    setPropsStatus("");
    setContentTab("data");
  }, [activeTable?.table, activeTable?.updatedAt]);

  function currentPropertiesPayload() {
    const attrs = (propsAttributes || [])
      .map((item) => ({
        name: String(item?.name || "").trim(),
        kind: String(item?.kind || "string"),
        index_types: Array.isArray(item?.index_types) ? item.index_types : [],
      }))
      .filter((item) => item.name);
    return { attributes: attrs };
  }

  async function saveActiveTableProperties(status = "Saving…") {
    if (!activeTable) return null;
    setPropsBusy(true);
    setPropsStatus(status);
    const payload = currentPropertiesPayload();
    const response = await requestJson(`${simpleTablesApi}/${encodeURIComponent(activeTable.table)}`, {
      method: "PUT",
      body: JSON.stringify(payload),
    });
    setPropsStatus("Saved");
    setReloadToken((v) => v + 1);
    return response;
  }

  async function handleUpdateTable(event) {
    event?.preventDefault?.();
    if (!activeTable) return;

    try {
      await saveActiveTableProperties("Saving…");
    } catch (error) {
      setPropsStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setPropsBusy(false);
    }
  }

  async function handleSyncSchemaToRepo() {
    setSchemaSyncBusy(true);
    setSchemaSyncStatus(activeTable ? "Saving table and syncing schema…" : "Syncing schema…");
    try {
      if (activeTable) {
        await saveActiveTableProperties("Saving before schema sync…");
      }
      const payload = await requestJson(sekejapSchemaSyncApi, { method: "POST" });
      const sync = payload?.sync || {};
      setSchemaSyncStatus(`Synced · ${Number(sync?.table_count || 0)} tables · ${Number(sync?.files_written?.length || 0)} files`);
      setReloadToken((v) => v + 1);
    } catch (error) {
      setSchemaSyncStatus(`Sync failed · ${String(error?.message || error)}`);
    } finally {
      setSchemaSyncBusy(false);
      setPropsBusy(false);
    }
  }

  function handleDownloadSchema() {
    const a = Object.assign(document.createElement("a"), {
      href: sekejapSchemaExportApi,
      download: `${input?.project || "project"}-sekejap-schema.json`,
    });
    a.click();
  }

  async function loadMaintenanceHealth({ silent = false } = {}) {
    if (!silent) {
      setMaintenanceBusy(true);
      setMaintenanceStatus("Checking store…");
    }
    try {
      const payload = await requestJson(`${sekejapMaintenanceApi}/health`);
      const health = payload?.health || null;
      setMaintenanceHealth(health);
      setMaintenanceStatus(health ? `Checked · ${Number(health?.duration_ms || 0)} ms` : "No health data returned");
      return health;
    } catch (error) {
      setMaintenanceStatus(`Error · ${String(error?.message || error)}`);
      return null;
    } finally {
      if (!silent) setMaintenanceBusy(false);
    }
  }

  async function runMaintenanceOperation(operation) {
    setMaintenanceBusy(true);
    setMaintenanceReport(null);
    setMaintenanceStatus(operation === "compact" ? "Compacting store…" : "Syncing WAL…");
    try {
      const payload = await requestJson(`${sekejapMaintenanceApi}/${operation}`, { method: "POST" });
      const report = payload?.[operation] || null;
      setMaintenanceReport(report);
      setMaintenanceHealth(report?.after || null);
      setMaintenanceStatus(`${operation === "compact" ? "Compacted" : "Synced"} · ${Number(report?.duration_ms || 0)} ms`);
    } catch (error) {
      setMaintenanceStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setMaintenanceBusy(false);
      setPendingMaintenanceAction("");
    }
  }

  async function handleDeleteTable() {
    if (!activeTable) return;
    setDeleteBusy(true);
    try {
      await requestJson(`${simpleTablesApi}/${encodeURIComponent(activeTable.table)}`, {
        method: "DELETE",
      });
      setDeleteConfirmOpen(false);
      setDeleteConfirmInput("");
      setSelectedTable("");
      setReloadToken((v) => v + 1);
    } catch (error) {
      setPropsStatus(`Delete failed · ${String(error?.message || error)}`);
    } finally {
      setDeleteBusy(false);
    }
  }

  const [totalRowCount, setTotalRowCount] = useState(null);
  const [countBusy, setCountBusy] = useState(false);
  const [pendingEdits, setPendingEdits] = useState({});
  const [editingCell, setEditingCell] = useState(null);
  const [mapPickerOpen, setMapPickerOpen] = useState(false);
  const [mapPickerTarget, setMapPickerTarget] = useState(null);
  const hasPendingEdits = Object.keys(pendingEdits).length > 0;

  async function handleRefreshData() {
    if (activeTable) {
      await loadPreviewData(activeTable.table);
      await loadTreeData(activeTable.table);
    } else {
      setReloadToken((v) => v + 1);
    }
    await loadMaintenanceHealth({ silent: true });
  }

  function handleCellEdit(rowKey, colName, newValue) {
    setPendingEdits((prev) => {
      const rowEdits = { ...(prev[rowKey] || {}), [colName]: newValue };
      return { ...prev, [rowKey]: rowEdits };
    });
  }

  function openMapPicker(target) {
    if (!target?.rowKey || !target?.colName) return;
    setMapPickerTarget(target);
    setMapPickerOpen(true);
    setEditingCell(null);
  }

  function handleMapPickerSave(geometry) {
    if (!mapPickerTarget?.rowKey || !mapPickerTarget?.colName) return;
    handleCellEdit(mapPickerTarget.rowKey, mapPickerTarget.colName, JSON.stringify(geometry));
  }

  function handleMapPickerClear() {
    if (!mapPickerTarget?.rowKey || !mapPickerTarget?.colName) return;
    handleCellEdit(mapPickerTarget.rowKey, mapPickerTarget.colName, "");
  }

  async function handleSaveEdits() {
    if (!activeTable || !dbApi.query || !hasPendingEdits) return;
    try {
      for (const edits of Object.values(pendingEdits)) {
        for (const [col, val] of Object.entries(edits || {})) {
          const warning = validateCellEditValue(activeTable, col, val);
          if (warning) {
            setValidationNotice(warning);
            return;
          }
        }
      }
      for (const [rowKey, edits] of Object.entries(pendingEdits)) {
        const setClauses = Object.entries(edits)
          .map(([col, val]) => {
            if (val === null || val === "") return `${col} = NULL`;
            const trimmed = typeof val === "string" ? val.trim() : String(val);
            if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
              try {
                const parsed = JSON.parse(trimmed);
                if (parsed && typeof parsed === "object" && parsed.type && (parsed.coordinates || parsed.geometries)) {
                  return `${col} = ST_GeomFromGeoJSON('${sqlStringLiteral(trimmed)}')`;
                }
              } catch (_) {}
            }
            const num = Number(val);
            if (!isNaN(num) && trimmed !== "") return `${col} = ${num}`;
            return `${col} = '${sqlStringLiteral(val)}'`;
          })
          .join(", ");
        await runDbQuery(
          `UPDATE ${activeTable.table} SET ${setClauses} WHERE _key = '${sqlStringLiteral(rowKey)}'`,
          { readOnly: false, tableName: activeTable.table, limit: 0 },
        );
      }
      setPendingEdits({});
      setEditingCell(null);
      await loadPreviewData(activeTable.table);
    } catch (error) {
      setQueryStatus(`Save failed · ${String(error?.message || error)}`);
    }
  }

  function handleCancelEdits() {
    setPendingEdits({});
    setEditingCell(null);
  }

  async function handleAddRow() {
    if (!activeTable || !dbApi.query) return;
    try {
      const uid = crypto.randomUUID();
      await runDbQuery(`INSERT INTO ${activeTable.table} (_key) VALUES ('${uid}')`, { readOnly: false, tableName: activeTable.table, limit: 0 });
      const { rows } = await loadPreviewData(activeTable.table);
      await loadTreeData(activeTable.table);
      if (rows.length) {
        const cols = mergedColumns.length ? mergedColumns : (activeTable.attributes || []).map((a) => a.name);
        const keyIdx = cols.indexOf("_key");
        const match = keyIdx >= 0 ? rows.find((r) => Array.isArray(r) && String(r[keyIdx]) === uid) : rows[rows.length - 1];
        const found = match || rows[rows.length - 1];
        const record = mapRowToObject(cols, Array.isArray(found) ? found : []);
        setSelectedPreviewRowKey(uid);
        setSelectedPreviewRowData(record);
      }
    } catch (error) {
      setQueryStatus(`Insert failed · ${String(error?.message || error)}`);
    }
  }

  async function handleDeleteSelectedRow() {
    if (!activeTable || !selectedPreviewRowData) return;
    const key = String(selectedPreviewRowData?._key || "").trim();
    if (!key) {
      setQueryStatus("Cannot delete · row has no _key");
      return;
    }
    try {
      await runDbQuery(`DELETE FROM ${activeTable.table} WHERE _key = '${sqlStringLiteral(key)}'`, { readOnly: false, tableName: activeTable.table, limit: 0 });
      setSelectedPreviewRowKey("");
      setSelectedPreviewRowData(null);
      await loadPreviewData(activeTable.table);
      await loadTreeData(activeTable.table);
    } catch (error) {
      setQueryStatus(`Delete failed · ${String(error?.message || error)}`);
    }
  }

  async function handleCountRows() {
    if (!activeTable || !dbApi.query) return;
    setCountBusy(true);
    try {
      const res = await runDbQuery(`SELECT COUNT(*) AS cnt FROM ${activeTable.table}`, { readOnly: true, tableName: activeTable.table, limit: 1 });
      const cnt = Number(res.objects?.[0]?.cnt ?? res.rows?.[0]?.[0] ?? 0);
      setTotalRowCount(cnt);
    } catch {
      setTotalRowCount(null);
    } finally {
      setCountBusy(false);
    }
  }

  function handleExportCsv() {
    if (!mergedColumns.length || !mergedRows.length) return;
    const escCsv = (v) => {
      const s = String(v ?? "");
      return s.includes(",") || s.includes('"') || s.includes("\n") ? `"${s.replace(/"/g, '""')}"` : s;
    };
    const header = mergedColumns.map(escCsv).join(",");
    const body = mergedRows.map((row) => (Array.isArray(row) ? row : []).map((cell) => escCsv(typeof cell === "object" ? JSON.stringify(cell) : cell)).join(",")).join("\n");
    const blob = new Blob([header + "\n" + body], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = Object.assign(document.createElement("a"), { href: url, download: `${activeTable?.table || "export"}.csv` });
    a.click();
    URL.revokeObjectURL(url);
  }

  useEffect(() => { setTotalRowCount(null); }, [selectedTable]);

  const activeTableName = activeTable?.table || selectedTable.split(".").pop() || "";
  const indexCount = activeTable
    ? new Set([
        ...(activeTable.hashIndexed || []),
        ...(activeTable.rangeIndexed || []),
        ...(activeTable.fulltextFields || []),
        ...(activeTable.vectorFields || []),
        ...(activeTable.spatialFields || []),
      ]).size
    : 0;
  const hasInspectedValue = !!String(valueBody || "").trim();
  const selectedNodeSlug = relationNodeSlug(selectedPreviewRowData, activeTable?.table || "");
  const selectedNodeLabel = selectedPreviewRowData ? relationNodeLabel(selectedPreviewRowData, activeTable?.table || "") : "";
  const relatedSlugParts = relationSlugParts(relatedNodeSlug);
  const relatedSlugWarning = relatedNodeSlug.trim() && relatedSlugParts && !tables.some((table) => table.table === relatedSlugParts.collection)
    ? `Unknown collection '${relatedSlugParts.collection}'.`
    : "";

  async function searchRelationTargets(collection, query) {
    const q = String(query || "").trim();
    const table = (tables || []).find((item) => String(item?.table || "") === String(collection || ""));
    const candidateCols = [
      "_collection",
      "_key",
      "_id",
      ...(table?.attributes || [])
        .filter((attr) => {
          const kind = String(attr?.kind || "").toLowerCase();
          return kind !== "geo" && kind !== "vector";
        })
        .map((attr) => String(attr?.name || "").trim())
        .filter(Boolean),
    ];
    const seenCols = new Set();
    const columns = candidateCols.filter((col) => {
      if (!col || seenCols.has(col)) return false;
      seenCols.add(col);
      return true;
    });
    const result = await runDbQuery(
      `SELECT ${columns.join(", ")} FROM ${collection}`,
      { readOnly: true, tableName: collection, limit: 200 }
    );
    return result.objects.map((item) => ({
      slug: relationNodeSlug(item, collection),
      label: relationNodeLabel(item, collection),
      searchText: Object.values(item || {}).map((value) => {
        if (value === null || value === undefined) return "";
        if (typeof value === "object") {
          try { return JSON.stringify(value); } catch (_) { return ""; }
        }
        return String(value);
      }).join(" "),
    })).filter((item) => {
      if (!item.slug) return false;
      const needle = q.toLowerCase();
      return !needle || item.slug.toLowerCase().includes(needle) || item.label.toLowerCase().includes(needle) || item.searchText.toLowerCase().includes(needle);
    });
  }

  useEffect(() => {
    if (!activeTable?.table || !selectedPreviewRowData?._key) {
      setOutgoingRelations([]);
      setIncomingRelations([]);
      setRelationTypeOptions([]);
      setRelationsError("");
      return;
    }
    loadRelationsForNode(activeTable.table, selectedPreviewRowData);
  }, [activeTable?.table, selectedPreviewRowData?._key, reloadToken]);

  useEffect(() => {
    if (!activeTable?.table) {
      setOutgoingRelationStats([]);
      setIncomingRelationStats([]);
      setRelationStatsError("");
      return;
    }
    loadRelationSummaryForTable(activeTable.table);
  }, [activeTable?.table, reloadToken]);

  return (
    <>
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu={`Databases / ${connection.slug || "connection"}`}
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={navLinks.db_connections ?? "#"}>Connections</StudioTabLink>
          {suiteTabs.map((item, index) => (
              <StudioTabLink key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} active={item?.classes === "is-active"}>
                {item?.label}
              </StudioTabLink>
            ))}
          </StudioTabNav>

          <section
            className="db-suite-page flex min-h-0 flex-1 flex-col overflow-auto bg-bg"
            data-db-suite="true"
            data-owner={input.owner}
            data-project={input.project}
            data-db-kind={connection.kind ?? ""}
            data-connection-slug={connection.slug ?? ""}
            data-connection-id={connection.id ?? ""}
            data-api-describe={dbApi.describe ?? ""}
            data-api-schemas={dbApi.schemas ?? ""}
            data-api-tables={dbApi.tables ?? ""}
            data-api-functions={dbApi.functions ?? ""}
            data-api-preview={dbApi.preview ?? ""}
            data-api-query={dbApi.query ?? ""}
          >
            <header className="db-suite-header">
              <p className="db-suite-panel-title">{connection.name}</p>
              <span className="project-inline-chip">
                <i className={`zf-devicon ${connection.icon_class || ""}`} aria-hidden="true"></i>
                <span>kind: {connection.kind} | slug: {connection.slug}</span>
              </span>
            </header>
            <section className="db-suite-shell">
              <div className="db-suite-main">
                {tabFlags?.tables ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="db-suite-table-split">
                      <aside className="db-suite-table-list" data-db-suite-object-tree="true">
                        <div className="db-suite-side-actions">
                          <p className="db-suite-side-title">Schemas</p>
                          <button type="button" className="project-inline-chip project-inline-chip-action" onClick={() => resetCreateForm(true)}>
                            Create Table
                          </button>
                        </div>

                        {treeError ? (
                          <div className="db-suite-empty">{treeError}</div>
                        ) : schemaNames.length === 0 ? (
                          <div className="db-suite-empty">No tables available yet.</div>
                        ) : (
                          schemaNames.map((schemaName, index) => {
                            const collapsed = !!collapsedSchemas[schemaName];
                            const items = (grouped.get(schemaName) || []).sort((a, b) => a.key.localeCompare(b.key));
                            return (
                              <section key={`${schemaName}-${index}`} className="db-suite-object-group">
                                <p className="db-suite-object-group-title">
                                  <button
                                    type="button"
                                    className="db-suite-schema-toggle"
                                    onClick={() =>
                                      setCollapsedSchemas((prev) => ({
                                        ...prev,
                                        [schemaName]: !prev[schemaName],
                                      }))
                                    }
                                  >
                                    <span className={cx("db-suite-schema-caret", collapsed ? "is-collapsed" : "")} aria-hidden="true">
                                      <svg viewBox="0 0 12 12" fill="none">
                                        <path d="M2.25 4.5L6 8.25L9.75 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"></path>
                                      </svg>
                                    </span>
                                    <i className="zf-devicon zf-icon-schema" aria-hidden="true"></i>
                                    <span>{schemaName}</span>
                                  </button>
                                </p>
                                <div className={cx("db-suite-object-items", collapsed ? "is-collapsed" : "")}>
                                  {items.map((item, itemIndex) => (
                                    <button
                                      key={`${item.key}-${itemIndex}`}
                                      type="button"
                                      className={cx("db-suite-object-item", item.key === selectedTable ? "is-active" : "")}
                                      onClick={() => {
                                        setSelectedTable(item.key);
                                        setValueMeta("Click a cell to inspect value");
                                        setValueBody("");
                                        setInspectedCellRaw(null);
                                      }}
                                    >
                                      <span className="db-suite-object-row">
                                        <i className="zf-devicon zf-icon-sjtable" aria-hidden="true"></i>
                                        <span>{item.table}</span>
                                      </span>
                                      <span>{item.rowCount || ""}</span>
                                    </button>
                                  ))}
                                </div>
                              </section>
                            );
                          })
                        )}
                      </aside>

                      <div className="db-suite-data-split">
                        <div className="flex min-h-0 flex-col">
                          <div className="flex min-h-0 flex-1 flex-col">
                            <div className="flex items-center justify-between gap-3 border-b border-ui-border/70 bg-ui-bg-muted/30 px-3 py-2">
                              <div className="min-w-0">
                                <p className="truncate text-sm font-medium text-ui-text">
                                  {activeTableName || "No table selected"}
                                </p>
                                <p className="text-xs text-ui-text-soft">
                                  {activeTable ? `${activeTable.schema} schema` : "Choose a table from the left or create a new one."}
                                </p>
                              </div>
                              <div className="flex flex-wrap items-center justify-end gap-2">
                                {activeTable ? (
                                  <div className="flex flex-wrap items-center justify-end gap-2 text-[11px] uppercase tracking-[0.14em] text-ui-text-soft">
                                    <span className="rounded-full border border-ui-border/80 px-2 py-1">{activeTable.rowCount || 0} rows</span>
                                    <span className="rounded-full border border-ui-border/80 px-2 py-1">{Math.max(schemaRows.length, activeTable.attributes.length)} fields</span>
                                    <span className="rounded-full border border-ui-border/80 px-2 py-1">{indexCount} indexed</span>
                                  </div>
                                ) : null}
                                <div className="flex items-center gap-2">
                                  <button
                                    type="button"
                                    className="rounded border border-ui-border px-2 py-1 text-xs font-medium text-ui-text-soft hover:text-ui-text disabled:opacity-50"
                                    disabled={schemaSyncBusy || propsBusy}
                                    onClick={handleSyncSchemaToRepo}
                                  >
                                    Sync schema to repo
                                  </button>
                                  <button
                                    type="button"
                                    className="rounded border border-ui-border px-2 py-1 text-xs font-medium text-ui-text-soft hover:text-ui-text"
                                    onClick={handleDownloadSchema}
                                  >
                                    Download schema
                                  </button>
                                </div>
                              </div>
                            </div>
                            {schemaSyncStatus ? (
                              <div className="border-b border-ui-border/70 px-3 py-1 text-xs text-ui-text-soft">
                                {schemaSyncStatus}
                              </div>
                            ) : null}

                            <div className="flex items-center gap-1 border-b border-ui-border/70 px-3">
                                <button
                                  type="button"
                                  className={cx(
                                    "px-3 py-1.5 text-xs font-medium transition-colors",
                                    contentTab === "data"
                                      ? "border-b-2 border-ui-text text-ui-text"
                                      : "text-ui-text-soft hover:text-ui-text"
                                  )}
                                  onClick={() => setContentTab("data")}
                                >
                                  Data
                                </button>
                                <button
                                  type="button"
                                  disabled={!activeTable}
                                  className={cx(
                                    "px-3 py-1.5 text-xs font-medium transition-colors",
                                    contentTab === "relations"
                                      ? "border-b-2 border-ui-text text-ui-text"
                                      : "text-ui-text-soft hover:text-ui-text",
                                    !activeTable ? "cursor-not-allowed opacity-40" : ""
                                  )}
                                  onClick={() => setContentTab("relations")}
                                >
                                  Relations
                                </button>
                                <button
                                  type="button"
                                  disabled={!activeTable}
                                  className={cx(
                                    "px-3 py-1.5 text-xs font-medium transition-colors",
                                    contentTab === "properties"
                                      ? "border-b-2 border-ui-text text-ui-text"
                                      : "text-ui-text-soft hover:text-ui-text",
                                    !activeTable ? "cursor-not-allowed opacity-40" : ""
                                  )}
                                  onClick={() => setContentTab("properties")}
                                >
                                  Properties
                                </button>
                              </div>

                            {contentTab === "relations" && activeTable ? (
                              <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-4">
                                <div className="flex flex-wrap items-start justify-between gap-3">
                                  <div>
                                    <p className="text-sm font-semibold text-ui-text">Relations</p>
                                    <p className="mt-1 text-xs text-ui-text-soft">
                                      Collection-level relation patterns for {activeTable.table}.
                                    </p>
                                  </div>
                                </div>

                                {relationStatsBusy ? <p className="text-xs text-ui-text-soft">Loading relation statistics…</p> : null}
                                {relationStatsError ? <p className="text-xs text-danger">Failed to load relation statistics: {relationStatsError}</p> : null}

                                <div className="grid gap-3 xl:grid-cols-2">
                                  <RelationStatsList
                                    title="Outbound By Type"
                                    items={outgoingRelationStats}
                                    emptyText="No outbound relation types are declared for this collection."
                                    peerKey="to"
                                  />
                                  <RelationStatsList
                                    title="Inbound By Type"
                                    items={incomingRelationStats}
                                    emptyText="No inbound relation types are declared for this collection."
                                    peerKey="from"
                                  />
                                </div>
                              </div>
                            ) : contentTab === "properties" && activeTable ? (
                              <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-4">
                                <form onSubmit={handleUpdateTable} className="flex flex-col gap-5">
                                  <div className="flex flex-col gap-3">
                                    <div className="flex items-center justify-between">
                                      <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Attributes</p>
                                      <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() =>
                                          setPropsAttributes((prev) => [
                                            ...prev,
                                            { ...DEFAULT_ATTRIBUTE },
                                          ])
                                        }
                                      >
                                        Add Attribute
                                      </Button>
                                    </div>
                                    {propsAttributes.length === 0 ? (
                                      <p className="text-xs text-ui-text-soft">No attributes defined yet.</p>
                                    ) : (
                                      propsAttributes.map((attr, idx) => (
                                        <AttributeEditorRow
                                          key={idx}
                                          item={attr}
                                          onChange={(next) =>
                                            setPropsAttributes((prev) =>
                                              prev.map((a, i) => (i === idx ? next : a))
                                            )
                                          }
                                          onRemove={() =>
                                            setPropsAttributes((prev) =>
                                              prev.filter((_, i) => i !== idx)
                                            )
                                          }
                                        />
                                      ))
                                    )}
                                  </div>

                                  <div className="flex items-center gap-3">
                                    <Button type="submit" size="sm" disabled={propsBusy}>
                                      {propsBusy ? "Saving…" : "Save Changes"}
                                    </Button>
                                    {propsStatus ? (
                                      <span className="text-xs text-ui-text-soft">{propsStatus}</span>
                                    ) : null}
                                  </div>
                                </form>

                                <div className="mt-8 rounded-lg border border-red-300/60 bg-red-50/30 p-4 dark:border-red-800/50 dark:bg-red-950/20">
                                  <p className="text-sm font-medium text-red-700 dark:text-red-400">Danger Zone</p>
                                  <p className="mt-1 text-xs text-red-600/80 dark:text-red-400/70">
                                    Permanently delete this table and all its data. This action cannot be undone.
                                  </p>
                                  <Button
                                    type="button"
                                    variant="outline"
                                    size="sm"
                                    className="mt-3 border-red-300 text-red-700 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-950/40"
                                    onClick={() => {
                                      setDeleteConfirmInput("");
                                      setDeleteConfirmOpen(true);
                                    }}
                                  >
                                    Delete Table
                                  </Button>
                                </div>

                                {deleteConfirmOpen ? (
                                  <Dialog open onOpenChange={(v) => { if (!v) setDeleteConfirmOpen(false); }}>
                                    <DialogContent onKeyDown={(e) => e.stopPropagation()}>
                                      <DialogHeader>
                                        <DialogTitle>Delete Table</DialogTitle>
                                      </DialogHeader>
                                      <div className="flex flex-col gap-3 py-2">
                                        <p className="text-sm text-ui-text-soft">
                                          This will permanently delete <strong>{activeTable.table}</strong> and all its rows. Type the table name to confirm.
                                        </p>
                                        <Input
                                          value={deleteConfirmInput}
                                          onInput={(e) => setDeleteConfirmInput(e.currentTarget.value)}
                                          placeholder={activeTable.table}
                                        />
                                      </div>
                                      <DialogFooter>
                                        <Button
                                          type="button"
                                          variant="outline"
                                          size="sm"
                                          onClick={() => setDeleteConfirmOpen(false)}
                                        >
                                          Cancel
                                        </Button>
                                        <Button
                                          type="button"
                                          size="sm"
                                          disabled={deleteConfirmInput !== activeTable.table || deleteBusy}
                                          className="bg-red-600 text-white hover:bg-red-700 disabled:opacity-40"
                                          onClick={handleDeleteTable}
                                        >
                                          {deleteBusy ? "Deleting…" : "Delete"}
                                        </Button>
                                      </DialogFooter>
                                    </DialogContent>
                                  </Dialog>
                                ) : null}
                              </div>
                            ) : (
                            <div className="db-suite-grid-wrap db-suite-grid-editor-wrap">
                              {activeTable ? (
                                <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-ui-border/70 bg-ui-bg-muted/30 px-2 py-1.5">
                                  <button type="button" title="Save changes" className={`flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium disabled:opacity-30 ${hasPendingEdits ? "bg-blue-600 text-white hover:bg-blue-700" : "text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text"}`} disabled={!hasPendingEdits} onClick={handleSaveEdits}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3 w-3"><path d="M13 14H3a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1h7.586a1 1 0 0 1 .707.293l2.414 2.414a1 1 0 0 1 .293.707V13a1 1 0 0 1-1 1Z"/><path d="M5 14V9h6v5M5 2v3h4"/></svg>
                                    Save
                                  </button>
                                  <button type="button" title="Cancel changes" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text disabled:opacity-30" disabled={!hasPendingEdits} onClick={handleCancelEdits}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3 w-3"><path d="m4 4 8 8M12 4l-8 8"/></svg>
                                    Cancel
                                  </button>
                                  <span className="mx-0.5 h-4 w-px bg-ui-border/60" />
                                  <button type="button" title="Add row" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text hover:bg-ui-bg-muted" onClick={handleAddRow}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M8 3v10M3 8h10"/></svg>
                                    Row
                                  </button>
                                  <button type="button" title="Delete selected row" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-red-500 disabled:opacity-30" disabled={!selectedPreviewRowData} onClick={handleDeleteSelectedRow}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M3 8h10"/></svg>
                                    Delete
                                  </button>
                                  <span className="mx-0.5 h-4 w-px bg-ui-border/60" />
                                  <button type="button" title="Export CSV" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text disabled:opacity-30" disabled={!mergedRows.length} onClick={handleExportCsv}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M8 2v8M4 7l4 4 4-4M2 13h12"/></svg>
                                    CSV
                                  </button>
                                  <button type="button" title="Calculate total row count" className={cx("flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium hover:bg-ui-bg-muted", countBusy ? "animate-pulse text-ui-text" : "text-ui-text-soft hover:text-ui-text")} onClick={handleCountRows}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M13 3H3v10h10V3ZM6 6h4M6 8h4M6 10h2"/></svg>
                                    Count
                                  </button>
                                  <button type="button" title="Refresh" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text" onClick={handleRefreshData}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M13.5 8A5.5 5.5 0 1 1 8 2.5M13.5 2.5v3h-3"/></svg>
                                    Refresh
                                  </button>
                                  <span className="ml-auto text-[10px] tabular-nums text-ui-text-soft">
                                    {totalRowCount !== null ? `${totalRowCount} rows` : `${mergedRows.length} loaded`}
                                  </span>
                                </div>
                              ) : null}
                              <div className="db-suite-grid-scroll">
                              {!activeTable ? (
                                <div className="flex h-full min-h-[14rem] items-center justify-center text-sm text-ui-text-soft">
                                  Select a table to inspect its data and structure.
                                </div>
                              ) : mergedRows.length ? (
                                <ResizableDataGrid
                                  columns={mergedColumns}
                                  rows={mergedRows}
                                  selectedRowKey={selectedPreviewRowKey}
                                  onRowSelect={(key, record) => {
                                    setSelectedPreviewRowKey(key);
                                    setSelectedPreviewRowData(record);
                                  }}
                                  onCellInspect={onCellInspect}
                                  mapRowToObject={mapRowToObject}
                                  editingCell={editingCell}
                                  pendingEdits={pendingEdits}
                                  onEditingCellChange={setEditingCell}
                                  onCellEdit={handleCellEdit}
                                  vectorFields={activeTable?.vectorFields}
                                  geoFields={activeTable?.spatialFields}
                                  onGeoPick={openMapPicker}
                                />
                              ) : (
                                <div className="flex min-h-full flex-col">
                                  <div className="border-b border-ui-border/70 px-3 py-4">
                                    <p className="text-sm font-medium text-ui-text">
                                      {previewError ? "Preview unavailable" : "No rows yet"}
                                    </p>
                                    <p className="mt-1 text-sm text-ui-text-soft">
                                      {previewError
                                        ? `Failed to load preview: ${previewError}`
                                        : "This table exists, but it does not have stored rows yet. The declared structure is still available below."}
                                    </p>
                                    {!previewError ? (
                                      <button type="button" className="mt-3 inline-flex items-center gap-1 rounded border border-ui-border bg-ui-bg px-2.5 py-1.5 text-xs font-medium text-ui-text hover:bg-ui-bg-muted" onClick={handleAddRow}>
                                        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M8 3v10M3 8h10"/></svg>
                                        Add First Row
                                      </button>
                                    ) : null}
                                  </div>
                                  <div className="min-h-0 flex-1 px-3 pt-4">
                                    <p className="mb-3 text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">
                                      Structure
                                    </p>
                                    <StructureTable
                                      activeTable={activeTable}
                                      schemaColumns={schemaColumns}
                                      schemaRows={schemaRows}
                                      schemaError={schemaError}
                                    />
                                  </div>
                                </div>
                              )}
                              </div>
                            </div>
                            )}
                          </div>

                          {contentTab === "data" && activeTable && mergedRows.length ? (
                            <div className="border-t border-ui-border/70 bg-ui-bg-muted/15 px-3 py-3">
                              <div className="mb-3 flex items-center justify-between gap-3">
                                <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">
                                  Structure
                                </p>
                                <a
                                  href={suiteTabs.find((item) => item?.label === "Schema")?.href ?? "#"}
                                  className="text-xs text-ui-text-soft underline-offset-4 hover:text-ui-text hover:underline"
                                >
                                  Open full schema
                                </a>
                              </div>
                              <div className="max-h-56 overflow-auto">
                                <StructureTable
                                  activeTable={activeTable}
                                  schemaColumns={schemaColumns}
                                  schemaRows={schemaRows}
                                  schemaError={schemaError}
                                />
                              </div>
                            </div>
                          ) : null}
                        </div>

                        <aside className="db-suite-value-panel">
                          <div className="db-suite-value-head">{selectedPreviewRowData ? "Node" : hasInspectedValue ? "Value" : "Overview"}</div>
                          <div className="db-suite-value-meta">
                            {selectedPreviewRowData ? selectedNodeSlug || valueMeta : hasInspectedValue ? valueMeta : activeTable ? `${activeTable.schema}.${activeTable.table}` : "Select a table"}
                          </div>
                          {selectedPreviewRowData ? (
                            <div className="flex min-h-0 flex-col gap-4 overflow-y-auto overflow-x-hidden px-3 py-3 text-sm text-ui-text" style={{ wordBreak: "break-word" }}>
                              <div className="flex items-start justify-between gap-3">
                                <div className="min-w-0">
                                  <p className="truncate text-sm font-medium text-ui-text">{selectedNodeLabel || selectedNodeSlug}</p>
                                  <p className="truncate text-xs text-ui-text-soft">{selectedNodeSlug}</p>
                                </div>
                                <Button type="button" variant="outline" size="sm" onClick={openRelationDialog}>
                                  New Relation
                                </Button>
                              </div>

                              {hasInspectedValue ? (
                                <div className="space-y-2">
                                  <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Selected Value</p>
                                  <p className="text-xs text-ui-text-soft">{valueMeta}</p>
                                  {isGeoJsonGeometry(inspectedCellRaw) ? (
                                    <GeoPreviewMap geometry={inspectedCellRaw} />
                                  ) : null}
                                  <pre className="max-h-32 overflow-y-auto overflow-x-hidden rounded-md border border-ui-border/70 bg-ui-bg-muted/20 p-2 text-xs text-ui-text" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
                                    {valueBody}
                                  </pre>
                                </div>
                              ) : null}

                              <div className="space-y-3">
                                <div className="flex flex-wrap items-center justify-between gap-2">
                                  <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Relations</p>
                                  {relationsBusy ? <span className="text-[11px] text-ui-text-soft">Loading…</span> : null}
                                </div>
                                {relationsError ? <p className="text-xs text-danger">Failed to load relations: {relationsError}</p> : null}
                                <RowRelationList
                                  title="Outgoing"
                                  items={outgoingRelations}
                                  emptyText="No outgoing relations."
                                  onDelete={setPendingRelationDelete}
                                />
                                <RowRelationList
                                  title="Incoming"
                                  items={incomingRelations}
                                  emptyText="No incoming relations."
                                  onDelete={setPendingRelationDelete}
                                />
                              </div>

                              {activeTable ? (
                              <div className="grid grid-cols-2 gap-2 text-xs uppercase tracking-[0.12em] text-ui-text-soft">
                                <span>Rows</span>
                                <span className="text-right">{activeTable.rowCount || 0}</span>
                                <span>Fields</span>
                                <span className="text-right">{Math.max(schemaRows.length, activeTable.attributes.length)}</span>
                                <span>Indexes</span>
                                <span className="text-right">{indexCount}</span>
                                <span>Slug</span>
                                <span className="truncate text-right normal-case tracking-normal text-ui-text">{activeTable.table}</span>
                              </div>
                              ) : null}

                              <div className="space-y-2">
                                <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Fields</p>
                                {schemaRows.length || activeTable?.attributes.length ? (
                                  <div className="flex flex-wrap gap-2">
                                    {(schemaRows.length
                                      ? schemaRows.map((row, index) => ({
                                          name: String(Array.isArray(row) ? row[0] ?? `field_${index + 1}` : `field_${index + 1}`),
                                        }))
                                      : (activeTable?.attributes || []).map((attr) => ({ name: String(attr?.name || "") }))
                                    )
                                      .filter((item) => item.name && !item.name.startsWith("_"))
                                      .map((item) => (
                                        <span key={item.name} className="inline-flex rounded-full border border-ui-border/80 px-2 py-1 text-xs text-ui-text-soft">
                                          {item.name}
                                        </span>
                                      ))}
                                  </div>
                                ) : (
                                  <p className="text-sm text-ui-text-soft">No field metadata available yet.</p>
                                )}
                              </div>

                              <p className="text-xs text-ui-text-soft">
                                Open the Relations tab for collection-level relation statistics.
                              </p>
                            </div>
                          ) : hasInspectedValue ? (
                            <div className="flex min-h-0 flex-col gap-3 overflow-y-auto overflow-x-hidden px-3 py-3">
                              {isGeoJsonGeometry(inspectedCellRaw) ? (
                                <GeoPreviewMap geometry={inspectedCellRaw} />
                              ) : null}
                              <pre className="db-suite-value-body" style={{ margin: 0 }}>{valueBody}</pre>
                            </div>
                          ) : (
                            <pre className="db-suite-value-body">Choose a table from the left to inspect its data and structure.</pre>
                          )}
                        </aside>
                      </div>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.query ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="db-suite-query-split">
                      <div className="db-suite-query-top">
                        <div className="db-suite-query-toolbar">
                          <button type="button" className="project-inline-chip project-inline-chip-action" onClick={runQuery}>
                            Run Query
                          </button>
                          <p className="db-suite-query-status">{queryStatus}</p>
                        </div>
                        <Textarea
                          className="db-suite-query-editor-host"
                          value={querySql}
                          onInput={(event) => setQuerySql(event?.target?.value || "")}
                          rows={10}
                        />
                      </div>

                      <div className="db-suite-query-bottom">
                        <div className="db-suite-grid-wrap">
                          <StudioTable variant="dbGrid">
                            <StudioThead>
                              <tr>
                                {queryColumns.map((col, index) => (
                                  <StudioTh key={`qcol-${col}-${index}`}>{col}</StudioTh>
                                ))}
                              </tr>
                            </StudioThead>
                            <tbody>
                              {queryRows.map((row, rowIndex) => (
                                <tr key={`qrow-${rowIndex}`}>
                                  {(Array.isArray(row) ? row : []).map((cell, cellIndex) => {
                                    const colName = queryColumns[cellIndex] || `column_${cellIndex + 1}`;
                                    return (
                                      <StudioTd key={`qcell-${rowIndex}-${cellIndex}`} onClick={() => onCellInspect(colName, rowIndex, cell)}>
                                        {displayCellText(cell, colName, activeTable?.vectorFields)}
                                      </StudioTd>
                                    );
                                  })}
                                </tr>
                              ))}
                              {!queryRows.length ? (
                                <tr>
                                  <StudioTd colSpan={Math.max(queryColumns.length, 1)}>No rows available</StudioTd>
                                </tr>
                              ) : null}
                            </tbody>
                          </StudioTable>
                        </div>
                      </div>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.graph ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-6 py-10 text-center">
                      <p className="text-base font-semibold text-ui-text">Interactive Graph Inserter</p>
                      <p className="max-w-2xl text-sm text-ui-text-soft">
                        This workspace will let users draft multiple Sekejap nodes and native edges on a canvas, validate the graph, then commit it as one transaction.
                      </p>
                      <Button type="button" variant="outline" size="sm" disabled>
                        Insert Graph
                      </Button>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.schema ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="flex h-full flex-col gap-4 p-6">
                      {!activeTable ? (
                        <div className="db-suite-empty">Select a table to inspect its schema.</div>
                      ) : (
                        <>
                          <div className="flex items-center justify-between gap-3">
                            <div>
                              <p className="text-lg font-semibold text-ui-text">{activeTable.table}</p>
                              <p className="text-sm text-ui-text-soft">{activeTable.rowCount || 0} rows</p>
                            </div>
                            <Button type="button" variant="outline" size="sm" onClick={() => resetCreateForm(true)}>
                              Create Table
                            </Button>
                          </div>

                          <StructureTable
                            activeTable={activeTable}
                            schemaColumns={schemaColumns}
                            schemaRows={schemaRows}
                            schemaError={schemaError}
                          />
                        </>
                      )}
                    </div>
                  </section>
                ) : null}

                {tabFlags?.mart ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="db-suite-mart-full">
                      <StudioTable>
                        <StudioThead>
                          <tr>
                            <StudioTh>Name</StudioTh>
                            <StudioTh>Description</StudioTh>
                            <StudioTh>Status</StudioTh>
                          </tr>
                        </StudioThead>
                        <tbody>
                          <tr>
                            <StudioTd>mart_sales_daily</StudioTd>
                            <StudioTd>Daily aggregated sales mart</StudioTd>
                            <StudioTd>draft</StudioTd>
                          </tr>
                          <tr>
                            <StudioTd>mart_retention_cohort</StudioTd>
                            <StudioTd>User retention cohort mart</StudioTd>
                            <StudioTd>draft</StudioTd>
                          </tr>
                        </tbody>
                      </StudioTable>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.maintenance ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <SekejapMaintenancePanel
                      health={maintenanceHealth}
                      report={maintenanceReport}
                      busy={maintenanceBusy}
                      status={maintenanceStatus}
                      onRefresh={() => loadMaintenanceHealth()}
                      onSync={() => runMaintenanceOperation("sync")}
                      onCompact={() => setPendingMaintenanceAction("compact")}
                    />
                  </section>
                ) : null}
              </div>
            </section>
          </section>
        </div>
        <CreateTableDialog
          open={createOpen}
          onOpenChange={setCreateOpen}
          tableSlug={createTableSlug}
          setTableSlug={setCreateTableSlug}
          attributes={createAttributes}
          setAttributes={setCreateAttributes}
          status={createStatus}
          busy={createBusy}
          onSubmit={handleCreateTable}
        />
        <RelationDialog
          open={relationCreateOpen}
          onOpenChange={setRelationCreateOpen}
          busy={relationCreateBusy}
          status={relationCreateStatus}
          direction={relationDirection}
          setDirection={setRelationDirection}
          relationType={relationType}
          setRelationType={setRelationType}
          relatedNodeSlug={relatedNodeSlug}
          setRelatedNodeSlug={setRelatedNodeSlug}
          currentNodeSlug={selectedNodeSlug}
          relationTypeOptions={relationTypeOptions}
          relatedSlugWarning={relatedSlugWarning}
          onOpenTargetSearch={() => setRelationTargetSearchOpen(true)}
          onSubmit={handleCreateRelation}
        />
        <RelationTargetSearchDialog
          open={relationTargetSearchOpen}
          onOpenChange={setRelationTargetSearchOpen}
          tables={tables}
          onSearch={searchRelationTargets}
          onSelect={setRelatedNodeSlug}
        />
        <MapPicker
          open={mapPickerOpen}
          onOpenChange={setMapPickerOpen}
          value={mapPickerTarget?.value}
          title={mapPickerTarget?.colName ? `Pick Geometry · ${mapPickerTarget.colName}` : "Pick Geometry"}
          onSave={handleMapPickerSave}
          onClear={handleMapPickerClear}
        />
        <DataWarningDialog
          notice={validationNotice}
          onClose={() => setValidationNotice(null)}
        />
        <ConfirmDialog
          open={!!pendingRelationDelete}
          onClose={() => setPendingRelationDelete(null)}
          onConfirm={() => {
            if (pendingRelationDelete) {
              handleDeleteRelation(pendingRelationDelete);
            }
          }}
          title="Delete Relation"
          message={
            pendingRelationDelete
              ? `Delete relation ${pendingRelationDelete.type} between ${selectedNodeSlug} and ${pendingRelationDelete.otherSlug}?`
              : ""
          }
          confirmLabel="Delete"
          variant="destructive"
        />
        <ConfirmDialog
          open={pendingMaintenanceAction === "compact"}
          onClose={() => setPendingMaintenanceAction("")}
          onConfirm={() => runMaintenanceOperation("compact")}
          title="Compact Sekejap Store"
          message="Compact the project-local Sekejap store now? This checkpoints the snapshot and truncates WAL replay data. Run it during low-traffic windows for large stores."
          confirmLabel="Compact"
        />
      </ProjectStudioShell>
    </>
  );
}
