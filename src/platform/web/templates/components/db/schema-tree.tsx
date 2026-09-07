import { cx } from "zeb/react";

/**
 * The schemas and tables of one connection.
 *
 * Owns which schemas are collapsed and nothing else; choosing a table is the
 * page's business, so it is handed up rather than decided here.
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
            const items = (grouped.get(schemaName) || []).sort((a, b) => a.key.localeCompare(b.key));
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
                  {items.map((item, itemIndex) => (
                    <button
                      key={`${item.key}-${itemIndex}`}
                      type="button"
                      className={cx("db-suite-object-item", item.key === selectedTable ? "is-active" : "")}
                      onClick={() => onSelectTable(item.key)}
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
  );
}
