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
      currentMenu="{input.current_menu}"
      owner="{input.owner}"
      project="{input.project}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href="{input.nav.links.build_templates}" className="project-tab-link {input.nav.classes.build_templates}">Templates</a>
          <a href="{input.nav.links.build_assets}" className="project-tab-link {input.nav.classes.build_assets}">Assets</a>
          <a href="{input.nav.links.build_docs}" className="project-tab-link {input.nav.classes.build_docs}">Docs</a>
        </nav>

        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">{input.page_title}</p>
                  <p className="project-content-copy">{input.page_subtitle}</p>
                </div>
                <a href="{input.primary_action.href}" className="project-inline-chip">{input.primary_action.label}</a>
              </div>
            </section>
            <section className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-3">
                  <article zFor="item in input.items" className="project-card">
                    <h3 className="project-card-title">{item.title}</h3>
                    <p className="project-card-copy">{item.description}</p>
                  </article>
                </div>
              </div>
            </section>

            <section zShow="input.docs.enabled" className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Project Docs</p>
                  <p className="project-content-copy">Context source for app design, ERD, and implementation notes.</p>
                </div>
              </div>
              <div className="project-content-body">
                <div className="project-split-panels">
                  <aside className="project-surface-panel">
                    <div className="project-surface-panel-head">Files</div>
                    <div className="project-list">
                      <a zFor="doc in input.docs.items" href="{doc.href}" className="project-list-item">
                        <p className="project-list-title">{doc.name}</p>
                        <p className="project-card-meta">{doc.kind} • {doc.path}</p>
                      </a>
                    </div>
                  </aside>
                  <article className="project-surface-panel">
                    <div className="project-surface-panel-head">Context</div>
                    <p className="project-card-meta">selected: {input.docs.selected_path}</p>
                    <pre className="db-suite-pre">{input.docs.selected_content}</pre>
                  </article>
                </div>
                <article className="project-card">
                  <h3 className="project-card-title">Create Doc Operation</h3>
                  <p className="project-card-copy">Operation id: create_project_doc</p>
                  <p className="project-card-meta">POST {input.docs.api.create} {"{ path, content }"}</p>
                </article>
              </div>
            </section>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
