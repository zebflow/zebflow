import ProjectStudioShell from "@/components/layout/project-studio-shell";

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
  return (
<Page>
    <ProjectStudioShell
      projectHref="{input.project_href}"
      projectLabel="{input.title}"
      currentMenu="Tables / {input.connection}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href="{input.nav.links.tables_connections}" className="project-tab-link">Connections</a>
          <span className="project-tab-link is-active">{input.connection}</span>
        </nav>
        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Tables</p>
                  <p className="project-content-copy">Read-only preview for available tables in this connection.</p>
                </div>
              </div>
            </section>
            <section className="project-content-section">
              <div className="project-content-body">
                <table className="project-table">
                  <thead>
                    <tr>
                      <th>Table</th>
                      <th>Rows</th>
                      <th>Updated</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr jFor="item in input.tables">
                      <td>{item.name}</td>
                      <td>{item.rows}</td>
                      <td>{item.updated}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </section>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
