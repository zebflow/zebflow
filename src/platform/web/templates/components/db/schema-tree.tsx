import { cx } from "zeb/react";

/**
 * The schemas and tables of one connection.
 *
 * Owns which schemas are collapsed and nothing else; choosing a table is the
 * page's business, so it is handed up rather than decided here.
 *
 * Within a schema, tables of rows and edge tables are listed apart, because
 * they are different things to work with: an edge table holds the edges
 * between two tables. The graphs those edge tables are declared in close the
 * list. An engine with no edge tables shows its schemas and tables only.
 */
export default function SchemaTree({
  schemaNames,
  grouped,
  selectedTable,
  collapsedSchemas,
  treeError,
  canCreateTable,
  onToggleSchema,
  onSelectTable,
  onCreateTable,
}) {
  const everyTable = schemaNames.flatMap((name) => grouped.get(name) || []);
  const graphs = graphsOf(everyTable);

  return (
      <aside className="db-suite-table-list" data-db-suite-object-tree="true">
        <div className="db-suite-side-actions">
          <p className="db-suite-side-title">Schemas</p>
          {canCreateTable ? (
            <button type="button" className="project-inline-chip project-inline-chip-action" onClick={onCreateTable}>
              Create Table
            </button>
          ) : null}
        </div>

        {treeError ? (
          <div className="db-suite-empty">{treeError}</div>
        ) : schemaNames.length === 0 ? (
          <div className="db-suite-empty">No tables available yet.</div>
        ) : (
          schemaNames.map((schemaName, index) => {
            const collapsed = !!collapsedSchemas[schemaName];
            const items = (grouped.get(schemaName) || []).slice().sort((a, b) => a.key.localeCompare(b.key));
            const rows = items.filter((item) => !item.edge);
            const edges = items.filter((item) => item.edge);
            return (
              <section key={`${schemaName}-${index}`} className="db-suite-object-group">
                <p className="db-suite-object-group-title">
                  <button
                    type="button"
                    className="db-suite-schema-toggle"
                    onClick={() => onToggleSchema(schemaName)}
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
                  {items.length === 0 ? <p className="px-3 py-1 text-xs text-muted-foreground">No tables yet.</p> : null}
                  {edges.length ? <p className="px-3 pb-1 pt-2 text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Tables</p> : null}
                  {rows.map((item) => (
                    <TableItem key={item.key} item={item} active={item.key === selectedTable} onSelect={onSelectTable} />
                  ))}
                  {edges.length ? <p className="px-3 pb-1 pt-2 text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Edge tables</p> : null}
                  {edges.map((item) => (
                    <TableItem key={item.key} item={item} active={item.key === selectedTable} onSelect={onSelectTable} />
                  ))}
                </div>
              </section>
            );
          })
        )}

        {graphs.length ? (
          <section className="db-suite-object-group">
            <p className="px-3 pb-1 pt-2 text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Graphs</p>
            {graphs.map((graph) => (
              <p key={graph.name} className="px-3 py-1 text-xs text-muted-foreground" title={graph.edgeTables.join(", ")}>
                <span className="font-medium text-foreground">{graph.name}</span>
                {" · "}
                {graph.edgeTables.length} edge {graph.edgeTables.length === 1 ? "table" : "tables"}
              </p>
            ))}
          </section>
        ) : null}
      </aside>
  );
}

/** One table in the list: a table of rows, or an edge table with its ends. */
function TableItem({ item, active, onSelect }) {
  const edge = item.edge;
  return (
    <button
      type="button"
      className={cx("db-suite-object-item", active ? "is-active" : "")}
      onClick={() => onSelect(item.key)}
    >
      <span className="db-suite-object-row min-w-0">
        {edge ? <EdgeTableIcon /> : <i className="zf-devicon zf-icon-sjtable" aria-hidden="true"></i>}
        <span className="min-w-0">
          <span className="block truncate">{item.table}</span>
          {edge ? (
            <span className="block truncate text-[11px] text-muted-foreground">
              {edge.source_table} → {edge.destination_table}
            </span>
          ) : null}
        </span>
      </span>
      <span>{edge ? "" : item.rowCount || ""}</span>
    </button>
  );
}

/** Two rows and the edge between them, for an edge table. */
function EdgeTableIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true" className="shrink-0 text-muted-foreground">
      <circle cx="3.5" cy="8" r="2" />
      <circle cx="12.5" cy="8" r="2" />
      <path d="M5.5 8h5" strokeLinecap="round" />
    </svg>
  );
}

/**
 * The property graphs the edge tables are declared in. A graph is not listed
 * by the engine on its own, so it is read off the edge tables that name it.
 */
function graphsOf(tables) {
  const byName = new Map();
  tables.forEach((table) => {
    const name = table?.edge?.graph;
    if (!name) return;
    if (!byName.has(name)) byName.set(name, []);
    byName.get(name).push(table.table);
  });
  return Array.from(byName.entries())
    .map(([name, edgeTables]) => ({ name, edgeTables }))
    .sort((a, b) => a.name.localeCompare(b.name));
}
