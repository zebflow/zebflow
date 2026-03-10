import ProjectStudioShell from "@/components/layout/project-studio-shell";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
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
  const cards = Array.isArray(input?.cards) ? input.cards : [];
  return (
<Page>
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu={input.current_menu}
      owner={input.owner}
      project={input.project}
      nav={input.nav}
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
                  {cards.map((item, index) => (
                    <a key={`${item?.href ?? "card"}-${index}`} href={item?.href ?? "#"} className="project-card block">
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <h3 className="project-card-title">{item?.title}</h3>
                          <p className="project-card-copy">{item?.description}</p>
                        </div>
                        {item?.tag ? <span className="project-inline-chip">{item.tag}</span> : null}
                      </div>
                    </a>
                  ))}
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
