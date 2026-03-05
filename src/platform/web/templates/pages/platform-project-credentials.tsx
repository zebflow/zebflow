import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initProjectCredentialsBehavior } from "@/components/behavior/project-credentials";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
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
  initProjectCredentialsBehavior();
  const credentialsApi = input?.credentials?.api ?? {};
  const credentialsRuntime = {
    owner: input?.owner ?? "",
    project: input?.project ?? "",
    api: {
      list: credentialsApi?.list ?? "",
      item_base: credentialsApi?.item_base ?? "",
    },
  };
  return (
<Page>
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu="Credentials"
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <span className="project-tab-link is-active">Credentials</span>
        </nav>
        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Credentials</p>
                  <p className="project-content-copy">Create and manage project credential records used by database and service nodes.</p>
                </div>
                <button type="button" className="project-inline-chip project-inline-chip-accent" data-credential-create="true">+ New Credential</button>
              </div>
            </section>

            <section className="project-content-section" data-project-credentials="true">
              <script id="project-credentials-runtime" type="application/json">
                {JSON.stringify(credentialsRuntime)}
              </script>
              <div className="project-content-body">
                <table className="project-table">
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>Title</th>
                      <th>Kind</th>
                      <th>Secret</th>
                      <th>Updated</th>
                      <th>Action</th>
                    </tr>
                  </thead>
                  <tbody data-credential-rows="true"></tbody>
                </table>
              </div>

              <dialog className="pipeline-editor-dialog" data-credential-dialog="true">
                <form method="dialog" className="pipeline-editor-dialog-form" data-credential-form="true">
                  <h3 className="pipeline-editor-dialog-title" data-credential-title="true">Credential</h3>
                  <p className="pipeline-editor-subtitle" data-credential-status="true">Ready.</p>

                  <label className="pipeline-editor-field">
                    <span>Credential ID</span>
                    <input name="credential_id" type="text" placeholder="pg-main" required data-credential-id="true" />
                    <small className="pipeline-editor-field-help">Stable slug used by nodes, for example `n.pg.query`.</small>
                  </label>

                  <label className="pipeline-editor-field">
                    <span>Title</span>
                    <input name="title" type="text" placeholder="Main Postgres" required />
                    <small className="pipeline-editor-field-help">Human-readable label shown in admin views.</small>
                  </label>

                  <label className="pipeline-editor-field">
                    <span>Kind</span>
                    <select name="kind" data-credential-kind="true" required>
                      <option value="postgres">postgres</option>
                      <option value="mysql">mysql</option>
                      <option value="openai">openai</option>
                      <option value="http">http</option>
                      <option value="custom">custom</option>
                    </select>
                    <small className="pipeline-editor-field-help">Determines required secret fields in this dialog.</small>
                  </label>

                  <div className="pipeline-editor-node-fields" data-credential-secret-fields="true"></div>

                  <label className="pipeline-editor-field">
                    <span>Notes</span>
                    <textarea name="notes" rows="3" placeholder="Optional notes"></textarea>
                    <small className="pipeline-editor-field-help">Operational notes only. Do not put secrets here.</small>
                  </label>

                  <div className="pipeline-editor-dialog-actions">
                    <button type="button" data-credential-delete="true">Delete</button>
                    <button type="button" data-credential-cancel="true">Cancel</button>
                    <button type="submit" data-credential-save="true">Save</button>
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
