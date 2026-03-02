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
      currentMenu="Pipelines / Registry"
      owner="{input.owner}"
      project="{input.project}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip"></nav>
        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Project Shell</p>
                  <p className="project-content-copy">The project workspace now behaves like a studio shell, not a landing page.</p>
                </div>
              </div>
            </section>
            <section className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-2">
                  <article className="project-card">
                    <h3 className="project-card-title">Owner</h3>
                    <p className="project-card-copy">{input.owner}</p>
                  </article>
                  <article className="project-card">
                    <h3 className="project-card-title">Project</h3>
                    <p className="project-card-copy">{input.project}</p>
                  </article>
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
