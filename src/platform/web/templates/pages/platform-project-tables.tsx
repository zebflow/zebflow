import ProjectStudioShell from "@/components/layout/project-studio-shell";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-connections.css" },
      { rel: "stylesheet", href: "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css" },
    ],
    scripts: [{ type: "module", src: "/assets/platform/project-db-connections.mjs" }],
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
      currentMenu="Databases / Connections"
      owner="{input.owner}"
      project="{input.project}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href="{input.nav.links.db_connections}" className="project-tab-link is-active">Connections</a>
        </nav>
        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Database Connections</p>
                  <p className="project-content-copy">Manage project database connections and open DB suite.</p>
                </div>
                <button type="button" className="project-inline-chip project-inline-chip-accent" data-db-connection-create="true">+ New Database Connection</button>
              </div>
            </section>
            <section className="project-content-section" data-project-db-connections="true"
              data-owner="{input.owner}"
              data-project="{input.project}"
              data-api-list="{input.db_connections.api.list}"
              data-api-item="{input.db_connections.api.item_base}"
              data-api-test="{input.db_connections.api.test}"
              data-api-credentials="{input.db_connections.api.credentials_list}"
            >
              <div className="project-content-body">
                <table className="project-table">
                  <thead>
                    <tr>
                      <th>Connection</th>
                      <th>Label</th>
                      <th>Kind</th>
                      <th>Credential</th>
                      <th>Updated</th>
                      <th>Action</th>
                    </tr>
                  </thead>
                  <tbody data-db-connection-rows="true">
                    <tr zFor="item in input.connections">
                      <td>
                        <span className="db-connection-name">
                          <i className="zf-devicon {item.icon_class}" aria-hidden="true"></i>
                          <span>{item.slug}</span>
                        </span>
                      </td>
                      <td>{item.name}</td>
                      <td>
                        <span className="db-connection-kind">
                          <i className="zf-devicon {item.icon_class}" aria-hidden="true"></i>
                          <span>{item.kind}</span>
                        </span>
                      </td>
                      <td>{item.credential_id || "-"}</td>
                      <td>{item.updated_at || "-"}</td>
                      <td>
                        <a href="{item.path}" className="project-inline-chip">Open</a>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <dialog className="pipeline-editor-dialog" data-db-connection-dialog="true">
                <form method="dialog" className="pipeline-editor-dialog-form" data-db-connection-form="true">
                  <h3 className="pipeline-editor-dialog-title" data-db-connection-title="true">Database Connection</h3>
                  <p className="pipeline-editor-subtitle" data-db-connection-status="true">Ready.</p>

                  <label className="pipeline-editor-field">
                    <span>Connection Slug</span>
                    <input name="connection_slug" type="text" placeholder="analytics-pg" required data-db-connection-slug="true" />
                  </label>

                  <label className="pipeline-editor-field">
                    <span>Connection Label</span>
                    <input name="connection_label" type="text" placeholder="Analytics Postgres" required />
                  </label>

                  <label className="pipeline-editor-field">
                    <span>Database Kind</span>
                    <select name="database_kind" data-db-connection-kind="true" required>
                      <option value="sjtable">sjtable (sekejap)</option>
                      <option value="postgresql">postgresql</option>
                      <option value="mysql">mysql</option>
                      <option value="sqlite">sqlite</option>
                      <option value="mongodb">mongodb</option>
                      <option value="redis">redis</option>
                      <option value="qdrant">qdrant</option>
                      <option value="pinecone">pinecone</option>
                      <option value="chromadb">chromadb</option>
                      <option value="elasticsearch">elasticsearch</option>
                    </select>
                  </label>

                  <div className="pipeline-editor-field">
                    <span>Credential</span>
                    <select name="credential_id" data-db-connection-credential-id="true">
                      <option value="">None</option>
                    </select>
                    <small className="pipeline-editor-field-help" data-db-connection-credential-help="true">
                      Select an existing credential for this database kind.
                    </small>
                  </div>

                  <div className="pipeline-editor-dialog-actions">
                    <button type="button" data-db-credential-create-inline="true">Create Credential</button>
                    <button type="button" data-db-credential-refresh-inline="true">Refresh Credentials</button>
                  </div>

                  <label className="pipeline-editor-field">
                    <span>Config JSON</span>
                    <textarea name="config_json" rows="6" placeholder="{ }" data-db-connection-config-json="true"></textarea>
                  </label>

                  <div className="pipeline-editor-dialog-actions">
                    <button type="button" data-db-connection-test="true">Test</button>
                    <button type="button" data-db-connection-delete="true">Delete</button>
                    <button type="button" data-db-connection-cancel="true">Cancel</button>
                    <button type="submit" data-db-connection-save="true">Save</button>
                  </div>
                </form>
              </dialog>

              <dialog className="pipeline-editor-dialog" data-db-credential-dialog="true">
                <form method="dialog" className="pipeline-editor-dialog-form" data-db-credential-form="true">
                  <h3 className="pipeline-editor-dialog-title">Create Credential</h3>
                  <p className="pipeline-editor-subtitle" data-db-credential-status="true">Create a credential and bind it to this connection.</p>

                  <label className="pipeline-editor-field">
                    <span>Credential ID (UUID v4)</span>
                    <input name="credential_id" type="text" readonly data-db-credential-id="true" />
                  </label>

                  <label className="pipeline-editor-field">
                    <span>Title</span>
                    <input name="title" type="text" placeholder="Primary Postgres" required data-db-credential-title="true" />
                  </label>

                  <label className="pipeline-editor-field">
                    <span>Kind</span>
                    <select name="kind" required data-db-credential-kind="true">
                      <option value="postgres">postgres</option>
                      <option value="mysql">mysql</option>
                      <option value="custom">custom</option>
                    </select>
                  </label>

                  <label className="pipeline-editor-field">
                    <span>Secret JSON</span>
                    <textarea name="secret_json" rows="8" data-db-credential-secret-json="true">{"{}"}</textarea>
                  </label>

                  <div className="pipeline-editor-dialog-actions">
                    <button type="button" data-db-credential-cancel="true">Cancel</button>
                    <button type="submit" data-db-credential-save="true">Create</button>
                  </div>
                </form>
              </dialog>
            </section>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
