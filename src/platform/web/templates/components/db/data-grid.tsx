import { useState, useEffect, useRef, cx } from "zeb";
import {
  isGeoJsonPoint,
  formatGeoValue,
  stringifyCell,
  rawCellValue,
  isVectorColumn,
  isGeoColumn,
  formatVectorPreview,
  shouldCompactVectorCell,
  displayCellText,
  cellTitleText,
  defaultColumnWidth,
  autoSizeColumns,
} from "@/components/db/cell-format";

/**
 * The one data grid for every database engine.
 *
 * Resizable and sortable columns, optional inline editing, and cell
 * inspection. Engine differences arrive as props (which columns are
 * geometry, which are vectors) rather than as branches on a driver name.
 */
export default function ResizableDataGrid({ columns, rows, selectedRowKey, onRowSelect, onCellInspect, mapRowToObject, editingCell, pendingEdits, onEditingCellChange, onCellEdit, vectorFields, geoFields, onGeoPick }) {
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
