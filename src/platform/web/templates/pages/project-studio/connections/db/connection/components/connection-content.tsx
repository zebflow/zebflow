import QueryTabPanel from "@/components/db/query-tab-panel";
import GraphTabPanel from "@/components/db/graph-tab-panel";
import SchemaTabPanel from "@/components/db/schema-tab-panel";
import MartTabPanel from "@/components/db/mart-tab-panel";
import MaintenancePanel from "@/components/db/maintenance-panel";
import ConnectionTablesView from "@/pages/project-studio/connections/db/connection/components/connection-tables-view";

/**
 * The DB suite's own section: header plus whichever tab `tabFlags` gates in
 * — never the engine's name. Tables is the heaviest tab, so it owns its own
 * file; the rest stay inline here since none of them approach that size.
 */
export default function ConnectionContent({ input, workspace }) {
  const { tabFlags, connection, caps, preview, catalog, maintenance, valueInspector } = workspace;
  const { selectedTable, activeTable } = catalog;
  // Every engine with edit_table_properties can reach this download, not
  // only sekejap, so the filename it saves as should not claim otherwise.
  const schemaExportFilename = `${input?.project || "project"}-db-schema.json`;

  return (
    <section
      className="db-suite-page flex min-h-0 flex-1 flex-col overflow-auto bg-bg"
      data-db-suite="true"
      data-owner={input.owner}
      data-project={input.project}
      data-db-kind={connection.kind ?? ""}
      data-connection-slug={connection.slug ?? ""}
      data-connection-id={connection.id ?? ""}
      data-api-describe={workspace.dbApi.describe ?? ""}
      data-api-schemas={workspace.dbApi.schemas ?? ""}
      data-api-tables={workspace.dbApi.tables ?? ""}
      data-api-functions={workspace.dbApi.functions ?? ""}
      data-api-preview={workspace.dbApi.preview ?? ""}
      data-api-query={workspace.dbApi.query ?? ""}
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
            <ConnectionTablesView workspace={workspace} schemaExportFilename={schemaExportFilename} />
          ) : null}

          {tabFlags?.query ? (
            <QueryTabPanel
              queryUrl={workspace.dbApi.query}
              initialSql={input?.query_example || ""}
              selectedTable={selectedTable}
              vectorFields={activeTable?.vectorFields}
              onCellInspect={valueInspector.inspectCell}
            />
          ) : null}

          {tabFlags?.graph ? <GraphTabPanel /> : null}

          {tabFlags?.schema ? (
            <SchemaTabPanel
              activeTable={activeTable}
              describe={{ columns: preview.schemaColumns, rows: preview.schemaRows, error: preview.schemaError }}
              canCreateTable={caps.createTable}
              onCreateTable={() => workspace.setCreateOpen(true)}
            />
          ) : null}

          {tabFlags?.mart ? <MartTabPanel /> : null}

          {tabFlags?.maintenance && caps.maintenance ? (
            <section className="db-suite-panel db-suite-panel-fill">
              <MaintenancePanel
                health={maintenance.health}
                report={maintenance.report}
                busy={maintenance.busy}
                status={maintenance.status}
                onRefresh={() => maintenance.reloadHealth()}
                onSync={() => maintenance.runOperation("sync")}
                onCompact={() => maintenance.setPendingAction("compact")}
              />
            </section>
          ) : null}
        </div>
      </section>
    </section>
  );
}
