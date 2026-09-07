import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import {
  DRAFT_ROW_PREFIX,
  jsonValueForCell,
  readableDbError,
  sqlStringLiteral,
  validateCellEditValue,
} from "@/components/db/table-data";

/**
 * Editing the rows of the open table.
 *
 * Owns everything the reader is part-way through: cells typed but not written,
 * rows drafted but not inserted, which cell is open, and the map picker. A
 * draft row exists because a table with a NOT NULL column has no valid empty
 * row — it is collected here and inserted on save rather than the moment the
 * button is pressed.
 *
 * Nothing here reloads the page's data itself. `onRowsChanged` and
 * `onTableChanged` say what happened; what to re-read is the page's business,
 * since it is the page that holds the preview and the catalog.
 */
export function useDataGridEditor({
  table,
  sql,
  grid,
  selection,
  runDbQuery,
  onInvalidInput,
}) {
  const { tableRef, rowIdentity, selectedTable, rowsApi, queryEnabled, onRowsChanged, onTableChanged } = sql;

  const [draftRows, setDraftRows] = useState([]);
  const [pendingEdits, setPendingEdits] = useState({});
  const [editingCell, setEditingCell] = useState(null);
  const [notice, setNotice] = useState("");
  const [totalRowCount, setTotalRowCount] = useState(null);
  const [countBusy, setCountBusy] = useState(false);
  const [mapPickerOpen, setMapPickerOpen] = useState(false);
  const [mapPickerTarget, setMapPickerTarget] = useState(null);
  // Both change data the reader cannot get back by pressing undo, so both ask.
  const [confirmSaveOpen, setConfirmSaveOpen] = useState(false);
  const [confirmDeleteRowOpen, setConfirmDeleteRowOpen] = useState(false);

  const hasPendingEdits = Object.keys(pendingEdits).length > 0 || draftRows.length > 0;

  // A count belongs to the table it was taken from.
  useEffect(() => {
    setTotalRowCount(null);
  }, [selectedTable]);

function handleCellEdit(rowKey, colName, newValue) {
  const key = String(rowKey ?? "");
  if (key.startsWith(DRAFT_ROW_PREFIX)) {
    const index = Number(key.slice(DRAFT_ROW_PREFIX.length));
    setDraftRows((prev) =>
      prev.map((draft, i) => (i === index ? { ...draft, [colName]: newValue } : draft)),
    );
    return;
  }
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
  if (!table || !queryEnabled || !hasPendingEdits) return;
  try {
    setNotice("");
    // Drafts first: each becomes one insert carrying the values typed into it.
    for (const draft of draftRows) {
      const values = {};
      for (const [col, val] of Object.entries(draft || {})) {
        if (col === rowIdentity) continue;
        values[col] = jsonValueForCell(val);
      }
      await requestJson(`${rowsApi}/${encodeURIComponent(selectedTable)}/rows`, {
        method: "POST",
        body: JSON.stringify(values),
      });
    }
    setDraftRows([]);
    for (const edits of Object.values(pendingEdits)) {
      for (const [col, val] of Object.entries(edits || {})) {
        const warning = validateCellEditValue(table, col, val);
        if (warning) {
          onInvalidInput(warning);
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
        `UPDATE ${tableRef} SET ${setClauses} WHERE ${rowIdentity} = '${sqlStringLiteral(rowKey)}'`,
        { readOnly: false, tableName: tableRef, limit: 0 },
      );
    }
    setPendingEdits({});
    setEditingCell(null);
    await onRowsChanged();
  } catch (error) {
    setNotice(`Save failed · ${readableDbError(error)}`);
  }
}

function handleCancelEdits() {
  setPendingEdits({});
  setDraftRows([]);
  setNotice("");
  setEditingCell(null);
}

function handleAddRow() {
  if (!table) return;
  if (!rowsApi) {
    setNotice("This engine cannot add rows.");
    return;
  }
  // A blank draft, not an insert: the database rejects an empty row on any
  // table with a column that cannot be null, which is most of them.
  setDraftRows((prev) => prev.concat([{}]));
  setNotice("");
}

async function handleDeleteSelectedRow() {
  if (!table || !selection.data) return;
  const key = String(selection.data?.[rowIdentity] ?? "").trim();
  if (!key) {
    setNotice(`Cannot delete · row has no ${rowIdentity}`);
    return;
  }
  try {
    await runDbQuery(`DELETE FROM ${tableRef} WHERE ${rowIdentity} = '${sqlStringLiteral(key)}'`, { readOnly: false, tableName: tableRef, limit: 0 });
    selection.setKey("");
    selection.setData(null);
    await onRowsChanged();
    await onTableChanged();
  } catch (error) {
    setNotice(`Delete failed · ${readableDbError(error)}`);
  }
}

async function handleCountRows() {
  if (!table || !queryEnabled) return;
  setCountBusy(true);
  try {
    const res = await runDbQuery(`SELECT COUNT(*) AS cnt FROM ${tableRef}`, { readOnly: true, tableName: tableRef, limit: 1 });
    const cnt = Number(res.objects?.[0]?.cnt ?? res.rows?.[0]?.[0] ?? 0);
    setTotalRowCount(cnt);
  } catch {
    setTotalRowCount(null);
  } finally {
    setCountBusy(false);
  }
}

  /** The loaded rows with the drafts after them, keyed apart. */
  function displayRows() {
    const draftKeyFor = (index) => `${DRAFT_ROW_PREFIX}${index}`;
    return grid.rows.concat(
      draftRows.map((draft, index) =>
        grid.columns.map((col) => (col === rowIdentity ? draftKeyFor(index) : draft?.[col] ?? null)),
      ),
    );
  }

  return {
    draftRows,
    pendingEdits,
    editingCell,
    setEditingCell,
    notice,
    setNotice,
    hasPendingEdits,
    displayRows,
    totalRowCount,
    countBusy,
    mapPicker: {
      open: mapPickerOpen,
      target: mapPickerTarget,
      close: () => setMapPickerOpen(false),
      openFor: openMapPicker,
      save: handleMapPickerSave,
      clear: handleMapPickerClear,
    },
    confirmSave: { open: confirmSaveOpen, setOpen: setConfirmSaveOpen },
    confirmDeleteRow: { open: confirmDeleteRowOpen, setOpen: setConfirmDeleteRowOpen },
    editCell: handleCellEdit,
    save: handleSaveEdits,
    cancel: handleCancelEdits,
    addRow: handleAddRow,
    deleteSelectedRow: handleDeleteSelectedRow,
    countRows: handleCountRows,
  };
}
