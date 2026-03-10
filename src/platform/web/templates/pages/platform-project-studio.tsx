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
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const items = Array.isArray(input?.items) ? input.items : [];
  const docs = input?.docs ?? {};
  const docItems = Array.isArray(docs?.items) ? docs.items : [];
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
        <nav className="project-tab-strip">
          <a href={navLinks.build_templates ?? "#"} className={`project-tab-link ${navClasses.build_templates || ""}`}>Templates</a>
          <a href={navLinks.build_assets ?? "#"} className={`project-tab-link ${navClasses.build_assets || ""}`}>Assets</a>
          <a href={navLinks.build_docs ?? "#"} className={`project-tab-link ${navClasses.build_docs || ""}`}>Docs</a>
        </nav>

        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">{input.page_title}</p>
                  <p className="project-content-copy">{input.page_subtitle}</p>
                </div>
                <a href={input?.primary_action?.href ?? "#"} className="project-inline-chip">{input?.primary_action?.label}</a>
              </div>
            </section>
            <section className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-3">
                  {items.map((item, index) => (
                    <article key={`${item?.title ?? "item"}-${index}`} className="project-card">
                      <h3 className="project-card-title">{item?.title}</h3>
                      <p className="project-card-copy">{item?.description}</p>
                    </article>
                  ))}
                </div>
              </div>
            </section>

            {docs?.enabled ? (
            <section className="project-content-section">
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
                      {docItems.map((doc, index) => (
                        <a key={`${doc?.path ?? "doc"}-${index}`} href={doc?.href ?? "#"} className="project-list-item">
                          <p className="project-list-title">{doc?.name}</p>
                          <p className="project-card-meta">{doc?.kind} | {doc?.path}</p>
                        </a>
                      ))}
                    </div>
                  </aside>
                  <article className="project-surface-panel">
                    <div className="project-surface-panel-head">Context</div>
                    <p className="project-card-meta">selected: {docs?.selected_path}</p>
                    <pre className="db-suite-pre">{docs?.selected_content}</pre>
                  </article>
                </div>
                <article className="project-card">
                  <h3 className="project-card-title">Create Doc Operation</h3>
                  <p className="project-card-copy">Operation id: create_project_doc</p>
                  <p className="project-card-meta">POST {docs?.api?.create} {"{ path, content }"}</p>
                </article>
              </div>
            </section>
            ) : null}
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
