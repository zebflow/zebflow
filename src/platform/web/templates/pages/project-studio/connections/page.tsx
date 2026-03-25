import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { initProjectDbConnectionsBehavior } from "@/pages/project-studio/connections/connections-behavior";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-connections.css" },
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


export default function Page(input) {
  initProjectDbConnectionsBehavior();
  const connections = Array.isArray(input?.connections) ? input.connections : [];
  const dbConnectionsRuntime = {
    owner: input?.owner ?? "",
    project: input?.project ?? "",
    api: {
      list: input?.db_connections?.api?.list ?? "",
      item_base: input?.db_connections?.api?.item_base ?? "",
      test: input?.db_connections?.api?.test ?? "",
      credentials_list: input?.db_connections?.api?.credentials_list ?? "",
    },
  };
  return (
<Page>
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu="Databases / Connections"
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={input.nav?.links?.db_connections ?? "#"} active>Connections</StudioTabLink>
        </StudioTabNav>
        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg">
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
            <section className="project-content-section" data-project-db-connections="true">
              <script id="project-db-connections-runtime" type="application/json">
                {JSON.stringify(dbConnectionsRuntime)}
              </script>
              <div className="project-content-body">
                <StudioTable>
                  <StudioThead>
                    <tr>
                      <StudioTh>Connection</StudioTh>
                      <StudioTh>Label</StudioTh>
                      <StudioTh>Kind</StudioTh>
                      <StudioTh>Credential</StudioTh>
                      <StudioTh>Updated</StudioTh>
                      <StudioTh>Action</StudioTh>
                    </tr>
                  </StudioThead>
                  <tbody data-db-connection-rows="true">
                    {connections.map((item, index) => (
                      <tr key={`${item?.slug ?? "conn"}-${index}`}>
                        <StudioTd>
                          <span className="db-connection-name">
                            <i className={`zf-devicon ${item.icon_class || ""}`} aria-hidden="true"></i>
                            <span>{item.slug}</span>
                          </span>
                        </StudioTd>
                        <StudioTd>{item.name}</StudioTd>
                        <StudioTd>
                          <span className="db-connection-kind">
                            <i className={`zf-devicon ${item.icon_class || ""}`} aria-hidden="true"></i>
                            <span>{item.kind}</span>
                          </span>
                        </StudioTd>
                        <StudioTd>{item.credential_id || "-"}</StudioTd>
                        <StudioTd>{item.updated_at || "-"}</StudioTd>
                        <StudioTd>
                          <a href={item.path} className="project-inline-chip">Open</a>
                        </StudioTd>
                      </tr>
                    ))}
                  </tbody>
                </StudioTable>
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
                      <option value="sekejap">sekejap</option>
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
                    <textarea name="config_json" rows="6" placeholder="JSON config" data-db-connection-config-json="true"></textarea>
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
                    <span>Credential ID</span>
                    <input name="credential_id" type="text" readOnly data-db-credential-id="true" />
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
