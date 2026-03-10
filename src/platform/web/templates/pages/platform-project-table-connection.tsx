import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initProjectDbSuiteBehavior } from "@/components/behavior/project-db-suite";
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
  initProjectDbSuiteBehavior();
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
                      <p className="db-suite-side-title">Object Tree</p>
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
                  <div className="db-suite-empty"></div>
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
