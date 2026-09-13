import ResizableDataGrid from "@/components/db/data-grid";
import StructureTable from "@/components/db/structure-table";
import GridToolbar from "@/components/db/grid-toolbar";
import { mapRowToObject } from "@/components/db/table-data";

/** A table that exists but holds nothing, with its declared structure below. */
function EmptyTable({ activeTable, describe, previewError, onAddRow }) {
  return (
    <div className="flex min-h-full flex-col">
      <div className="border-b border-border/70 px-3 py-4">
        <p className="text-sm font-medium text-foreground">
          {previewError ? "Preview unavailable" : "No rows yet"}
        </p>
        <p className="mt-1 text-sm text-muted-foreground">
          {previewError
            ? `Failed to load preview: ${previewError}`
            : "This table exists, but it does not have stored rows yet. The declared structure is still available below."}
        </p>
        {!previewError ? (
          <button
            type="button"
            className="mt-3 inline-flex items-center gap-1 rounded border border-border bg-popover px-2.5 py-1.5 text-xs font-medium text-foreground hover:bg-accent"
            onClick={onAddRow}
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5">
              <path d="M8 3v10M3 8h10" />
            </svg>
            Add First Row
          </button>
        ) : null}
      </div>
      <div className="min-h-0 flex-1 px-3 pt-4">
        <p className="mb-3 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
          Structure
        </p>
        <StructureTable
          activeTable={activeTable}
          schemaColumns={describe.columns}
          schemaRows={describe.rows}
          schemaError={describe.error}
        />
      </div>
    </div>
  );
}

/**
 * The rows of the open table, and the toolbar that acts on them.
 *
 * The editing state lives in `useDataGridEditor` above; this draws it. What
 * capabilities the engine declared arrive as `caps`, so a grid on an engine
 * that cannot edit in place is simply not given the handlers.
 */
export default function DataTabPanel({ activeTable, grid, editor, selection, caps, describe }) {
  return (
    <div className="db-suite-grid-wrap db-suite-grid-editor-wrap">
      {/* Failures from the grid's own actions. Without this an insert the
          database refused said nothing at all. */}
      {editor.notice ? (
        <div className="shrink-0 border-b border-red-500/40 bg-red-500/10 px-3 py-1.5 text-[11px] text-red-400">
          {editor.notice}
        </div>
      ) : null}

      {activeTable ? (
        <GridToolbar
          editor={editor}
          canDeleteRow={!!selection.data}
          csv={{ href: grid.csvHref, filename: `${activeTable?.table || "export"}.csv` }}
          loadedCount={grid.rows.length}
          onRefresh={grid.onRefresh}
        />
      ) : null}

      <div className="db-suite-grid-scroll">
        {!activeTable ? (
          <div className="flex h-full min-h-[14rem] items-center justify-center text-sm text-muted-foreground">
            Select a table to inspect its data and structure.
          </div>
        ) : grid.rows.length ? (
          <ResizableDataGrid
            columns={grid.columns}
            rows={editor.displayRows()}
            columnMeta={grid.columnMeta}
            identityColumn={caps.rowIdentity}
            selectedRowKey={selection.key}
            onRowSelect={(key, record) => {
              selection.setKey(key);
              selection.setData(record);
            }}
            onCellInspect={grid.onCellInspect}
            mapRowToObject={mapRowToObject}
            editingCell={caps.inlineEdit ? editor.editingCell : null}
            pendingEdits={caps.inlineEdit ? editor.pendingEdits : null}
            onEditingCellChange={caps.inlineEdit ? editor.setEditingCell : undefined}
            onCellEdit={caps.inlineEdit ? editor.editCell : undefined}
            vectorFields={activeTable?.vectorFields}
            geoFields={caps.geo ? activeTable?.spatialFields : []}
            onGeoPick={caps.geo ? editor.mapPicker.openFor : undefined}
          />
        ) : (
          <EmptyTable
            activeTable={activeTable}
            describe={describe}
            previewError={grid.previewError}
            onAddRow={editor.addRow}
          />
        )}
      </div>
    </div>
  );
}
