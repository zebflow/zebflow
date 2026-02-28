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
      currentMenu="Tables / Connections"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href="{input.nav.links.tables_connections}" className="project-tab-link is-active">Connections</a>
        </nav>
        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Connections</p>
                  <p className="project-content-copy">Select a connection, then inspect tables like a lightweight database browser.</p>
                </div>
              </div>
            </section>
            <section className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-2">
                  <a jFor="item in input.connections" href="{item.path}" className="project-card">
                    <div className="flex items-start justify-between gap-3">
                      <h3 className="project-card-title">{item.name}</h3>
                      <span className="project-inline-chip">{item.driver}</span>
                    </div>
                    <p className="project-card-copy">Open table browser for this connection.</p>
                  </a>
                </div>
              </div>
            </section>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
