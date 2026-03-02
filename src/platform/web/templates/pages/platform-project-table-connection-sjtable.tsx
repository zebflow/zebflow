import ProjectStudioShell from "@/components/layout/project-studio-shell";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-suite.css" },
      { rel: "stylesheet", href: "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css" },
    ],
    scripts: [{ type: "module", src: "/assets/platform/project-db-suite-sjtable.mjs" }],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "h-screen overflow-hidden bg-slate-950 text-slate-100 font-sans",
  },
  navigation: "history",
};

export const app = {};

export default function Page(input) {
  return (
<Page>
    <ProjectStudioShell
      projectHref="{input.project_href}"
      projectLabel="{input.title}"
      currentMenu="Databases / {input.connection.slug}"
      owner="{input.owner}"
      project="{input.project}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href="{input.nav.links.db_connections}" className="project-tab-link">Connections</a>
          <a zFor="item in input.suite_tabs" href="{item.href}" className="project-tab-link {item.classes}">{item.label}</a>
        </nav>
        <section className="project-workspace-body db-suite-page" data-db-suite="true"
          data-owner="{input.owner}"
          data-project="{input.project}"
          data-db-kind="{input.connection.kind}"
          data-connection-slug="{input.connection.slug}"
          data-connection-id="{input.connection.id}"
          data-api-describe="{input.db_runtime_api.describe}"
          data-api-schemas="{input.db_runtime_api.schemas}"
          data-api-tables="{input.db_runtime_api.tables}"
          data-api-functions="{input.db_runtime_api.functions}"
          data-api-preview="{input.db_runtime_api.preview}"
          data-api-query="{input.db_runtime_api.query}"
        >
          <header className="db-suite-header">
            <p className="db-suite-panel-title">{input.connection.name}</p>
            <span className="project-inline-chip">
              <i className="zf-devicon {input.connection.icon_class}" aria-hidden="true"></i>
              <span>kind: {input.connection.kind} | slug: {input.connection.slug}</span>
            </span>
          </header>

          <section className="db-suite-shell">
            <div className="db-suite-main">
              <section zShow="input.tab_flags.tables" className="db-suite-panel db-suite-panel-fill">
                <div className="db-suite-table-split">
                  <aside className="db-suite-table-list" data-db-suite-object-tree="true">
                    <div className="db-suite-side-actions">
                      <p className="db-suite-side-title">Object Tree</p>
                      <button
                        type="button"
                        className="project-inline-chip project-inline-chip-action"
                        data-db-suite-create-open="true"
                      >
                        + Create Table
                      </button>
                    </div>
                    <section zFor="group in input.object_groups" className="db-suite-object-group">
                      <p className="db-suite-object-group-title">
                        <i className="zf-devicon {group.icon_class}" aria-hidden="true"></i>
                        <span>{group.label}</span>
                      </p>
                      <a zFor="item in group.items" href="{item.href}" className="db-suite-object-item {item.classes}">
                        <span className="db-suite-object-row">
                          <i className="zf-devicon {item.icon_class}" aria-hidden="true"></i>
                          <span>{item.label}</span>
                        </span>
                        <span>({item.row_count})</span>
                      </a>
                    </section>
                  </aside>
                  <div className="db-suite-data-split">
                    <div className="db-suite-grid-wrap">
                      <table className="project-table" data-db-suite-table-preview-table="true">
                        <thead>
                          <tr data-db-suite-table-preview-head="true">
                            <th zFor="col in input.preview.columns">{col}</th>
                          </tr>
                        </thead>
                        <tbody data-db-suite-table-preview-body="true">
                          <tr zFor="row in input.preview.rows">
                            <td zFor="cell in row">{cell}</td>
                          </tr>
                          <tr zShow="input.preview.empty">
                            <td colspan="8">No rows available</td>
                          </tr>
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
              <div className="db-suite-modal-backdrop is-hidden" data-db-suite-create-modal="true" role="dialog" aria-modal="true" aria-labelledby="db-suite-create-title">
                <section className="db-suite-modal-card">
                  <header className="db-suite-modal-head">
                    <h2 id="db-suite-create-title">Create Table</h2>
                    <button type="button" className="db-suite-modal-close" data-db-suite-create-cancel="true" aria-label="Close">
                      ×
                    </button>
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
                    <label className="db-suite-form-field">
                      <span>Hash Indexed Attributes</span>
                      <input type="text" name="hash_fields" placeholder="author_id, post_id" />
                    </label>
                    <label className="db-suite-form-field">
                      <span>Range Indexed Attributes</span>
                      <input type="text" name="range_fields" placeholder="created_at, updated_at" />
                    </label>
                    <p className="db-suite-form-note">
                      SekejapDB handles data payload attributes dynamically; indexing here is optional.
                    </p>
                    <p className="db-suite-form-status" data-db-suite-create-status="true"></p>
                    <div className="db-suite-form-actions">
                      <button type="button" className="project-inline-chip" data-db-suite-create-cancel="true">Cancel</button>
                      <button type="submit" className="project-inline-chip project-inline-chip-action" data-db-suite-create-submit="true">
                        Create
                      </button>
                    </div>
                  </form>
                </section>
              </div>

              <section zShow="input.tab_flags.query" className="db-suite-panel db-suite-panel-fill">
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
                            <th zFor="col in input.preview.columns">{col}</th>
                          </tr>
                        </thead>
                        <tbody data-db-suite-query-body="true">
                          <tr zFor="row in input.preview.rows">
                            <td zFor="cell in row">{cell}</td>
                          </tr>
                          <tr zShow="input.preview.empty">
                            <td colspan="8">No rows available</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  </div>
                </div>
              </section>

              <section zShow="input.tab_flags.schema" className="db-suite-panel db-suite-panel-fill">
                <div className="db-suite-empty"></div>
              </section>

              <section zShow="input.tab_flags.mart" className="db-suite-panel db-suite-panel-fill">
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
            </div>
          </section>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
