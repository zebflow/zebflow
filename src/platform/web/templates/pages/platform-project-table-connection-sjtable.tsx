import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initProjectDbSuiteSjtableBehavior } from "@/components/behavior/project-db-suite-sjtable";
import { cx } from "rwe";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-suite.css" },
      { rel: "stylesheet", href: "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css" },
    ],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

export default function Page(input) {
  initProjectDbSuiteSjtableBehavior();
  const navLinks = input?.nav?.links ?? {};
  const suiteTabs = Array.isArray(input?.suite_tabs) ? input.suite_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const objectGroups = Array.isArray(input?.object_groups) ? input.object_groups : [];
  const preview = input?.preview ?? {};
  const previewColumns = Array.isArray(preview?.columns) ? preview.columns : [];
  const previewRows = Array.isArray(preview?.rows) ? preview.rows : [];
  const connection = input?.connection ?? {};
  const dbApi = input?.db_runtime_api ?? {};

  return (
<Page>
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu={`Databases / ${connection.slug || "connection"}`}
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href={navLinks.db_connections ?? "#"} className="project-tab-link">Connections</a>
          {suiteTabs.map((item, index) => (
            <a key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} className={cx("project-tab-link", item?.classes)}>{item?.label}</a>
          ))}
        </nav>
        <section className="project-workspace-body db-suite-page" data-db-suite="true"
          data-owner={input.owner}
          data-project={input.project}
          data-db-kind={connection.kind ?? ""}
          data-connection-slug={connection.slug ?? ""}
          data-connection-id={connection.id ?? ""}
          data-api-describe={dbApi.describe ?? ""}
          data-api-schemas={dbApi.schemas ?? ""}
          data-api-tables={dbApi.tables ?? ""}
          data-api-functions={dbApi.functions ?? ""}
          data-api-preview={dbApi.preview ?? ""}
          data-api-query={dbApi.query ?? ""}
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
                <section className="db-suite-panel db-suite-panel-fill">
                  <div className="db-suite-table-split">
                    <aside className="db-suite-table-list" data-db-suite-object-tree="true">
                      <div className="db-suite-side-actions">
                        <p className="db-suite-side-title">Object Tree</p>
                        <button type="button" className="project-inline-chip project-inline-chip-action" data-db-suite-create-open="true">
                          + Create Table
                        </button>
                      </div>
                      {objectGroups.map((group, index) => (
                        <section key={`${group?.label ?? "group"}-${index}`} className="db-suite-object-group">
                          <p className="db-suite-object-group-title">
                            <i className={`zf-devicon ${group?.icon_class || ""}`} aria-hidden="true"></i>
                            <span>{group?.label}</span>
                          </p>
                          {(Array.isArray(group?.items) ? group.items : []).map((item, itemIndex) => (
                            <a key={`${item?.href ?? "item"}-${itemIndex}`} href={item?.href ?? "#"} className={cx("db-suite-object-item", item?.classes)}>
                              <span className="db-suite-object-row">
                                <i className={`zf-devicon ${item?.icon_class || ""}`} aria-hidden="true"></i>
                                <span>{item?.label}</span>
                              </span>
                              <span>({item?.row_count})</span>
                            </a>
                          ))}
                        </section>
                      ))}
                    </aside>
                    <div className="db-suite-data-split">
                      <div className="db-suite-grid-wrap">
                        <table className="project-table" data-db-suite-table-preview-table="true">
                          <thead>
                            <tr data-db-suite-table-preview-head="true">
                              {previewColumns.map((col, index) => <th key={`${col}-${index}`}>{col}</th>)}
                            </tr>
                          </thead>
                          <tbody data-db-suite-table-preview-body="true">
                            {previewRows.map((row, rowIndex) => (
                              <tr key={`row-${rowIndex}`}>
                                {(Array.isArray(row) ? row : []).map((cell, cellIndex) => <td key={`cell-${rowIndex}-${cellIndex}`}>{cell}</td>)}
                              </tr>
                            ))}
                            {preview?.empty ? (
                              <tr>
                                <td colSpan={8}>No rows available</td>
                              </tr>
                            ) : null}
                          </tbody>
                        </table>
                      </div>
                      <aside className="db-suite-value-panel">
                        <div className="db-suite-value-head">Value</div>
                        <div className="db-suite-value-meta" data-db-suite-value-meta="true">Click a cell to inspect value</div>
                        <pre className="db-suite-value-body" data-db-suite-value-body="true"></pre>
                      </aside>
                    </div>
                  </div>
                </section>
              ) : null}

              <div className="db-suite-modal-backdrop is-hidden" data-db-suite-create-modal="true" role="dialog" aria-modal="true" aria-labelledby="db-suite-create-title">
                <section className="db-suite-modal-card">
                  <header className="db-suite-modal-head">
                    <h2 id="db-suite-create-title">Create Table</h2>
                    <button type="button" className="db-suite-modal-close" data-db-suite-create-cancel="true" aria-label="Close">×</button>
                  </header>
                  <form className="db-suite-create-form" data-db-suite-create-form="true">
                    <label className="db-suite-form-field">
                      <span>Table Slug</span>
                      <input type="text" name="table" placeholder="posts" required />
                    </label>
                    <label className="db-suite-form-field">
                      <span>Title (Optional)</span>
                      <input type="text" name="title" placeholder="Blog Posts" />
                    </label>
                    <div className="db-suite-form-field">
                      <div className="db-suite-attrs-header">
                        <span>Attributes</span>
                        <button type="button" className="project-inline-chip project-inline-chip-action db-suite-attrs-add-btn" data-db-suite-attr-add="true">+ Add</button>
                      </div>
                      <div className="db-suite-attrs-list" data-db-suite-attrs="true"></div>
                      <p className="db-suite-attrs-hint">Index options appear based on the selected kind.</p>
                    </div>
                    <p className="db-suite-form-status" data-db-suite-create-status="true"></p>
                    <div className="db-suite-form-actions">
                      <button type="button" className="project-inline-chip" data-db-suite-create-cancel="true">Cancel</button>
                      <button type="submit" className="project-inline-chip project-inline-chip-action" data-db-suite-create-submit="true">Create</button>
                    </div>
                  </form>
                </section>
              </div>

              {tabFlags?.query ? (
                <section className="db-suite-panel db-suite-panel-fill">
                  <div className="db-suite-query-split">
                    <div className="db-suite-query-top">
                      <div className="db-suite-query-toolbar">
                        <button type="button" className="project-inline-chip project-inline-chip-action" data-db-suite-query-run="true">Run Query</button>
                        <p className="db-suite-query-status" data-db-suite-query-status="true">Ready</p>
                      </div>
                      <div className="db-suite-query-editor-host" data-db-suite-query-editor="true">{input.query_example}</div>
                    </div>
                    <div className="db-suite-query-bottom">
                      <div className="db-suite-grid-wrap">
                        <table className="project-table">
                          <thead>
                            <tr data-db-suite-query-head="true">
                              {previewColumns.map((col, index) => <th key={`qcol-${col}-${index}`}>{col}</th>)}
                            </tr>
                          </thead>
                          <tbody data-db-suite-query-body="true">
                            {previewRows.map((row, rowIndex) => (
                              <tr key={`qrow-${rowIndex}`}>
                                {(Array.isArray(row) ? row : []).map((cell, cellIndex) => <td key={`qcell-${rowIndex}-${cellIndex}`}>{cell}</td>)}
                              </tr>
                            ))}
                            {preview?.empty ? (
                              <tr>
                                <td colSpan={8}>No rows available</td>
                              </tr>
                            ) : null}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.schema ? (
                <section className="db-suite-panel db-suite-panel-fill">
                  <div className="db-suite-schema-view" data-db-suite-schema-panel="true">
                    <div className="db-suite-schema-hint">Select a table from the tree to view its schema.</div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.mart ? (
                <section className="db-suite-panel db-suite-panel-fill">
                  <div className="db-suite-mart-full">
                    <table className="project-table">
                      <thead>
                        <tr>
                          <th>Name</th>
                          <th>Description</th>
                          <th>Status</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <td>mart_sales_daily</td>
                          <td>Daily aggregated sales mart</td>
                          <td>draft</td>
                        </tr>
                        <tr>
                          <td>mart_retention_cohort</td>
                          <td>User retention cohort mart</td>
                          <td>draft</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </section>
              ) : null}
            </div>
          </section>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
