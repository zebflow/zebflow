import { cx } from "zeb/react";
import SchemaTree from "@/components/db/schema-tree";
import DataTabHeader from "@/components/db/data-tab-header";
import RelationsSummaryPanel from "@/components/db/relations-summary-panel";
import TablePropertiesPanel from "@/components/db/table-properties-panel";
import DataTabPanel from "@/components/db/data-tab-panel";
import StructureTable from "@/components/db/structure-table";
import RowInspectorPanel from "@/components/db/row-inspector-panel";
import RelationCreatePanel from "@/components/db/relation-create-panel";

/**
 * The Tables tab's own content: the schema tree, the Data/Relations/
 * Properties split, and the row inspector. Pulled out of `ConnectionContent`
 * because it was the single heaviest tab on its own. Everything here reads
 * off `workspace` — the composition root `useConnectionWorkspace` builds —
 * so one prop carries the dozen fields it touches instead of naming each one
 * at this seam.
 */
export default function ConnectionTablesView({ workspace, schemaExportFilename }) {
  const {
    caps,
    api,
    dbTypes,
    catalog,
    preview,
    grid,
    selection,
    valueInspector,
    relationStats,
    nodeRelations,
    gridEditor,
    properties,
    facts,
    connection,
    suiteTabs,
  } = workspace;
  const { tables, selectedTable, activeTable } = catalog;

  return (
    <section className="db-suite-panel db-suite-panel-fill">
      <div className="db-suite-table-split">
        <SchemaTree
          schemaNames={catalog.schemaNames}
          grouped={catalog.grouped}
          selectedTable={selectedTable}
          collapsedSchemas={catalog.collapsedSchemas}
          treeError={catalog.treeError}
          canCreateTable={caps.createTable}
          onToggleSchema={(name) => catalog.toggleSchema(name)}
          onSelectTable={(key) => {
            catalog.setSelectedTable(key);
            valueInspector.reset();
          }}
          onCreateTable={() => workspace.setCreateOpen(true)}
        />

        {/* The split reserves a column for the row inspector,
            which belongs to Data. Properties takes the whole
            width instead, so a table's columns are readable. */}
        <div
          className={cx(
            "db-suite-data-split",
            workspace.contentTab === "properties" ? "!grid-cols-[minmax(0,1fr)]" : "",
          )}
        >
          <div className="flex min-h-0 flex-col">
            <div className="flex min-h-0 flex-1 flex-col">
              <DataTabHeader
                table={{ name: facts.tableName, active: activeTable }}
                facts={{
                  fieldCount: Math.max(preview.schemaRows.length, activeTable?.attributes.length || 0),
                  indexCount: facts.indexCount,
                }}
                tabs={{
                  current: workspace.contentTab,
                  onSelect: workspace.setContentTab,
                  showRelations: caps.graphRelations,
                  showProperties: caps.editProperties,
                }}
                schema={{
                  busy: properties.sync.busy || properties.busy,
                  status: properties.sync.status,
                  // The route only exists when the driver declared it, so its
                  // presence is the gate — not whether a table happens to be
                  // open, which was true on every engine regardless.
                  canSync: Boolean(api.schemaSync),
                  onSync: properties.sync.run,
                  exportHref: api.schemaExport,
                  exportFilename: schemaExportFilename,
                }}
              />

              {workspace.contentTab === "relations" && activeTable && caps.graphRelations ? (
                <RelationsSummaryPanel
                  tableName={activeTable.table}
                  stats={relationStats}
                />
              ) : workspace.contentTab === "properties" && activeTable && caps.editProperties ? (
                <TablePropertiesPanel
                  activeTable={activeTable}
                  engine={connection.kind}
                  types={dbTypes}
                  properties={properties}
                  canDropTable={caps.dropTable}
                  onDeleted={() => catalog.setSelectedTable("")}
                />
              ) : (
              <DataTabPanel
                activeTable={activeTable}
                grid={{
                  columns: grid.mergedColumns,
                  rows: grid.mergedRows,
                  columnMeta: grid.columnMeta,
                  csvHref: grid.csvHref,
                  previewError: preview.previewError,
                  onCellInspect: valueInspector.inspectCell,
                  onRefresh: workspace.refreshAll,
                }}
                editor={gridEditor}
                selection={selection}
                caps={{
                  rowIdentity: caps.rowIdentity,
                  inlineEdit: caps.inlineEdit,
                  geo: caps.geo,
                }}
                describe={{
                  columns: preview.schemaColumns,
                  rows: preview.schemaRows,
                  error: preview.schemaError,
                }}
              />
              )}
            </div>

            {workspace.contentTab === "data" && activeTable && grid.mergedRows.length ? (
              <div className="border-t border-border/70 bg-accent/15 px-3 py-3">
                <div className="mb-3 flex items-center justify-between gap-3">
                  <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
                    Structure
                  </p>
                  <a
                    href={suiteTabs.find((item) => item?.label === "Schema")?.href ?? "#"}
                    className="text-xs text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
                  >
                    Open full schema
                  </a>
                </div>
                <div className="max-h-56 overflow-auto">
                  <StructureTable
                    activeTable={activeTable}
                    schemaColumns={preview.schemaColumns}
                    schemaRows={preview.schemaRows}
                    schemaError={preview.schemaError}
                  />
                </div>
              </div>
            ) : null}
          </div>

          {workspace.contentTab === "properties" ? null : (
          <RowInspectorPanel
            node={{
              record: selection.data,
              slug: facts.nodeSlug,
              label: facts.nodeLabel,
              activeTable,
            }}
            value={valueInspector}
            relations={{
              enabled: caps.graphRelations,
              outgoing: nodeRelations.outgoing,
              incoming: nodeRelations.incoming,
              busy: nodeRelations.busy,
              error: nodeRelations.error,
              pendingDelete: workspace.pendingRelationDelete,
              setPendingDelete: workspace.setPendingRelationDelete,
              onDelete: nodeRelations.deleteRelation,
            }}
            facts={{
              fieldCount: Math.max(preview.schemaRows.length, activeTable?.attributes.length || 0),
              indexCount: facts.indexCount,
              fieldNames: facts.fieldNames,
            }}
            hasGeo={caps.geo}
          >
            {caps.graphRelations ? (
              <RelationCreatePanel
                runDbQuery={workspace.runDbQuery}
                tables={tables}
                current={{ table: activeTable, record: selection.data }}
                typeOptions={workspace.relationTypeOptions}
                onCreated={() => nodeRelations.reload()}
                onInvalidInput={workspace.setValidationNotice}
              />
            ) : null}
          </RowInspectorPanel>
          )}
        </div>
      </div>
    </section>
  );
}
