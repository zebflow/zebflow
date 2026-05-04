import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { initProjectDbSuiteBehavior } from "@/pages/project-studio/connections/db/connection/db-suite-behavior";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { cx } from "zeb";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";

export const page = {
  head: {
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-suite.css" },
      { rel: "stylesheet", href: "/assets/libraries/zeb/icons/0.1/runtime/devicons.css" },
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

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

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
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu={`Databases / ${connection.slug || "connection"}`}
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={navLinks.db_connections ?? "#"}>Connections</StudioTabLink>
          {suiteTabs.map((item, index) => (
            <StudioTabLink key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} active={item?.classes === "is-active"}>{item?.label}</StudioTabLink>
          ))}
        </StudioTabNav>
        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg db-suite-page" data-db-suite="true"
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
                        <StudioTable variant="dbGrid" data-db-suite-table-preview-table="true">
                          <StudioThead>
                            <tr data-db-suite-table-preview-head="true">
                              {previewColumns.map((col, index) => <StudioTh key={`${col}-${index}`}>{col}</StudioTh>)}
                            </tr>
                          </StudioThead>
                          <tbody data-db-suite-table-preview-body="true">
                            {previewRows.map((row, rowIndex) => (
                              <tr key={`row-${rowIndex}`}>
                                {(Array.isArray(row) ? row : []).map((cell, cellIndex) => <StudioTd key={`cell-${rowIndex}-${cellIndex}`}>{cell}</StudioTd>)}
                              </tr>
                            ))}
                            {preview?.empty ? (
                              <tr>
                                <StudioTd colSpan={8}>No rows available</StudioTd>
                              </tr>
                            ) : null}
                          </tbody>
                        </StudioTable>
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
                        <StudioTable variant="dbGrid">
                          <StudioThead>
                            <tr data-db-suite-query-head="true">
                              {previewColumns.map((col, index) => <StudioTh key={`qcol-${col}-${index}`}>{col}</StudioTh>)}
                            </tr>
                          </StudioThead>
                          <tbody data-db-suite-query-body="true">
                            {previewRows.map((row, rowIndex) => (
                              <tr key={`qrow-${rowIndex}`}>
                                {(Array.isArray(row) ? row : []).map((cell, cellIndex) => <StudioTd key={`qcell-${rowIndex}-${cellIndex}`}>{cell}</StudioTd>)}
                              </tr>
                            ))}
                            {preview?.empty ? (
                              <tr>
                                <StudioTd colSpan={8}>No rows available</StudioTd>
                              </tr>
                            ) : null}
                          </tbody>
                        </StudioTable>
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
                    <StudioTable>
                      <StudioThead>
                        <tr>
                          <StudioTh>Name</StudioTh>
                          <StudioTh>Description</StudioTh>
                          <StudioTh>Status</StudioTh>
                        </tr>
                      </StudioThead>
                      <tbody>
                        <tr>
                          <StudioTd>mart_sales_daily</StudioTd>
                          <StudioTd>Daily aggregated sales mart</StudioTd>
                          <StudioTd>draft</StudioTd>
                        </tr>
                        <tr>
                          <StudioTd>mart_retention_cohort</StudioTd>
                          <StudioTd>User retention cohort mart</StudioTd>
                          <StudioTd>draft</StudioTd>
                        </tr>
                      </tbody>
                    </StudioTable>
                  </div>
                </section>
              ) : null}
            </div>
          </section>
        </section>
      </div>
    </ProjectStudioShell>
  );
}
