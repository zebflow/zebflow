import CreateTablePanel from "@/components/db/create-table-panel";
import MapPicker from "@/components/ui/map-picker";
import ValidationNoticeDialog from "@/components/db/validation-notice-dialog";
import ConfirmDialog from "@/components/ui/confirm-dialog";

/**
 * Every modal the connection page can open, gathered in one place so `Page`
 * stays a layout, not a dialog inventory. All of it reads off `workspace` —
 * the composition root `useConnectionWorkspace` already builds — so this
 * takes one prop instead of the half-dozen individual pieces each dialog
 * touches.
 */
export default function ConnectionDialogs({ workspace }) {
  const { caps, api, dbTypes, catalog, selection, facts, gridEditor, maintenance } = workspace;

  return (
    <>
      {caps.createTable ? (
        <CreateTablePanel
          open={workspace.createOpen}
          onOpenChange={workspace.setCreateOpen}
          tablesApi={api.tables}
          types={dbTypes}
          onCreated={(table) => catalog.reload(table)}
        />
      ) : null}
      <MapPicker
        open={gridEditor.mapPicker.open}
        onOpenChange={gridEditor.mapPicker.close}
        value={gridEditor.mapPicker.target?.value}
        title={gridEditor.mapPicker.target?.colName ? `Pick Geometry · ${gridEditor.mapPicker.target.colName}` : "Pick Geometry"}
        onSave={gridEditor.mapPicker.save}
        onClear={gridEditor.mapPicker.clear}
      />
      <ValidationNoticeDialog
        notice={workspace.validationNotice}
        onClose={() => workspace.setValidationNotice(null)}
      />
      <ConfirmDialog
        open={gridEditor.confirmSave.open}
        onClose={() => gridEditor.confirmSave.setOpen(false)}
        onConfirm={gridEditor.save}
        title="Write changes"
        message={`Write ${facts.pendingEditCount} cell change${facts.pendingEditCount === 1 ? "" : "s"} and ${gridEditor.draftRows.length} new row${gridEditor.draftRows.length === 1 ? "" : "s"} to ${catalog.tableRef}?`}
        confirmLabel="Write"
      />
      <ConfirmDialog
        open={gridEditor.confirmDeleteRow.open}
        onClose={() => gridEditor.confirmDeleteRow.setOpen(false)}
        onConfirm={gridEditor.deleteSelectedRow}
        title="Delete row"
        message={`Delete the row where ${caps.rowIdentity} is ${String(selection.data?.[caps.rowIdentity] ?? "")} from ${catalog.tableRef}? This cannot be undone.`}
        confirmLabel="Delete"
        variant="destructive"
      />
      <ConfirmDialog
        open={maintenance.pendingAction === "compact"}
        onClose={() => maintenance.setPendingAction("")}
        onConfirm={() => maintenance.runOperation("compact")}
        title="Compact Sekejap Store"
        message="Compact the project-local Sekejap store now? This checkpoints the snapshot and truncates WAL replay data. Run it during low-traffic windows for large stores."
        confirmLabel="Compact"
      />
    </>
  );
}
