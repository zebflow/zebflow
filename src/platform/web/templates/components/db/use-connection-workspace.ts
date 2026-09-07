import { useEffect, useMemo, useState } from "zeb/react";
import { computeMergedGrid, gridAsCsv } from "@/components/db/table-data";
import { relationNodeLabel, relationNodeSlug } from "@/components/db/relations-graph";
import {
  countIndexedColumns,
  readConnectionInput,
  readableFieldNames,
} from "@/components/db/connection-input";
import { useDbQuery } from "@/components/db/use-db-query";
import { useTableCatalog } from "@/components/db/use-table-catalog";
import { useTablePreview } from "@/components/db/use-table-preview";
import { useValueInspector } from "@/components/db/use-value-inspector";
import { useMaintenance } from "@/components/db/use-maintenance";
import { useRelationStats } from "@/components/db/use-relation-stats";
import { useRowSelection } from "@/components/db/use-row-selection";
import { useNodeRelations } from "@/components/db/use-node-relations";
import { useDataGridEditor } from "@/components/db/use-data-grid-editor";
import { useTableProperties } from "@/components/db/use-table-properties";

/**
 * One connection's screen, assembled.
 *
 * Each hook below owns one thing; this is the only place that knows they exist
 * together, which is what makes refreshing — preview, then catalog, then store
 * health — expressible at all. The page it serves is left with the layout and
 * nothing else.
 *
 * A few pieces stay here rather than inside a hook because more than one of
 * them writes or reads them:
 *  - `relationTypeOptions`, discovered by whichever relation query ran last
 *  - `validationNotice`, raised by the grid and by the relation dialog alike
 *  - `contentTab`, which decides which of three panels renders next
 *  - `createOpen`, flipped by two buttons that belong to neither each other
 */
export function useConnectionWorkspace(input) {
  const { dbApi, caps, api, initialTable, ...rest } = readConnectionInput(input);

  const runDbQuery = useDbQuery(dbApi.query);

  const catalog = useTableCatalog({
    schemasUrl: dbApi.schemas,
    tablesUrl: dbApi.tables,
    initialTable,
    qualifySchema: caps.qualifySchema,
  });
  const { selectedTable, activeTable, tableRef, reloadToken } = catalog;

  const preview = useTablePreview({
    previewUrl: dbApi.preview,
    describeUrl: dbApi.describe,
    table: selectedTable,
  });

  const valueInspector = useValueInspector();
  const maintenance = useMaintenance(api.maintenance);

  const [createOpen, setCreateOpen] = useState(false);
  const [relationTypeOptions, setRelationTypeOptions] = useState([]);
  const [validationNotice, setValidationNotice] = useState(null);
  const [pendingRelationDelete, setPendingRelationDelete] = useState(null);
  const [contentTab, setContentTab] = useState("data");

  // Row-selection effects depend on these arrays. Rebuilding them for a
  // selection-only update would repeatedly select a newly allocated record.
  const grid = useMemo(
    () => computeMergedGrid(activeTable, preview.previewColumns, preview.previewRows),
    [activeTable, preview.previewColumns, preview.previewRows],
  );

  const relationStats = useRelationStats({
    runDbQuery,
    enabled: caps.graphRelations && !!dbApi.query,
    tableName: activeTable?.table || "",
    reloadToken,
    onTypeOptions: setRelationTypeOptions,
  });

  const selection = useRowSelection({
    activeTable,
    columns: grid.mergedColumns,
    rows: grid.mergedRows,
    rowIdentity: caps.rowIdentity,
  });

  const nodeRelations = useNodeRelations({
    runDbQuery,
    enabled: caps.graphRelations,
    tableName: activeTable?.table || "",
    record: selection.data,
    reloadToken,
    onTypeOptions: setRelationTypeOptions,
  });

  const gridEditor = useDataGridEditor({
    table: activeTable,
    sql: {
      tableRef,
      rowIdentity: caps.rowIdentity,
      selectedTable,
      rowsApi: api.tables,
      queryEnabled: !!dbApi.query,
      onRowsChanged: () => preview.reloadPreview(selectedTable),
      onTableChanged: () => catalog.reload(selectedTable),
    },
    grid: { columns: grid.mergedColumns, rows: grid.mergedRows },
    selection,
    runDbQuery,
    onInvalidInput: setValidationNotice,
  });

  const properties = useTableProperties({
    propertiesApi: api.properties,
    tablesApi: api.tables,
    schemaSyncApi: api.schemaSync,
    table: activeTable,
    selectedTable,
    onTableChanged: catalog.refresh,
  });

  useEffect(() => {
    maintenance.reloadHealth({ silent: true });
  }, [api.maintenance, reloadToken]);

  // Opening a different table puts the reader back on its data.
  useEffect(() => {
    setContentTab("data");
  }, [activeTable?.table]);

  /** Re-read everything this screen shows about the open table. */
  async function refreshAll() {
    if (activeTable) {
      await preview.reloadPreview(selectedTable);
      await catalog.reload(selectedTable);
    } else {
      catalog.refresh();
    }
    await maintenance.reloadHealth({ silent: true });
  }

  return {
    ...rest,
    dbApi,
    caps,
    api,
    runDbQuery,
    catalog,
    preview,
    grid: {
      ...grid,
      // Computed for the link rather than written into a document element on
      // click: a browser downloads an `<a download>` on its own, and building
      // an anchor to click is the DOM manipulation the templates rule out.
      csvHref: grid.mergedRows.length
        ? `data:text/csv;charset=utf-8,${encodeURIComponent(gridAsCsv(grid.mergedColumns, grid.mergedRows))}`
        : "",
    },
    selection,
    valueInspector,
    maintenance,
    relationStats,
    nodeRelations,
    gridEditor,
    properties,
    contentTab,
    setContentTab,
    createOpen,
    setCreateOpen,
    relationTypeOptions,
    validationNotice,
    setValidationNotice,
    pendingRelationDelete,
    setPendingRelationDelete,
    refreshAll,
    facts: {
      tableName: activeTable?.table || selectedTable.split(".").pop() || "",
      indexCount: countIndexedColumns(activeTable),
      fieldCount: Math.max(preview.schemaRows.length, activeTable?.attributes.length || 0),
      fieldNames: readableFieldNames(preview.schemaRows, activeTable),
      nodeSlug: relationNodeSlug(selection.data, activeTable?.table || ""),
      nodeLabel: selection.data
        ? relationNodeLabel(selection.data, activeTable?.table || "")
        : "",
      pendingEditCount: Object.values(gridEditor.pendingEdits).reduce(
        (total, edits) => total + Object.keys(edits || {}).length,
        0,
      ),
    },
  };
}
