import Button from "@/components/ui/button";
import StructureTable from "@/components/db/structure-table";

/** What the engine says the open table's columns are. */
export default function SchemaTabPanel({ activeTable, describe, canCreateTable, onCreateTable }) {
  return (
    <section className="db-suite-panel db-suite-panel-fill">
      <div className="flex h-full flex-col gap-4 p-6">
        {!activeTable ? (
          <div className="db-suite-empty">Select a table to inspect its schema.</div>
        ) : (
          <>
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-lg font-semibold text-foreground">{activeTable.table}</p>
                <p className="text-sm text-muted-foreground">{activeTable.rowCount || 0} rows</p>
              </div>
              {canCreateTable ? (
                <Button type="button" variant="outline" size="sm" onClick={onCreateTable}>
                  Create Table
                </Button>
              ) : null}
            </div>

            <StructureTable
              activeTable={activeTable}
              schemaColumns={describe.columns}
              schemaRows={describe.rows}
              schemaError={describe.error}
            />
          </>
        )}
      </div>
    </section>
  );
}
