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
    >
      <div className="project-workspace">
        <div className="project-tab-strip"></div>
        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">{input.page_title}</p>
                  <p className="project-content-copy">{input.page_subtitle}</p>
                </div>
              </div>
            </section>
            <section className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-2">
                  <a zFor="item in input.cards" href="{item.href}" className="project-card block">
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <h3 className="project-card-title">{item.title}</h3>
                        <p className="project-card-copy">{item.description}</p>
                      </div>
                      <span zShow="item.tag" className="project-inline-chip">{item.tag}</span>
                    </div>
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
