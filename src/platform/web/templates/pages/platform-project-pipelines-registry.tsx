import ProjectStudioShell from "@/components/layout/project-studio-shell";

function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

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
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const registry = input?.registry ?? {};
  const breadcrumbs = Array.isArray(registry?.breadcrumbs) ? registry.breadcrumbs : [];
  const folders = Array.isArray(registry?.folders) ? registry.folders : [];
  const pipelines = Array.isArray(registry?.pipelines) ? registry.pipelines : [];

  return (
    <>
      <ProjectStudioShell
        projectHref={input?.project_href}
        projectLabel={input?.title}
        currentMenu={input?.current_menu}
        owner={input?.owner}
        project={input?.project}
        nav={input?.nav}
      >
        <div className="project-workspace">
          <nav className="project-tab-strip">
            <a href={navLinks.pipelines_registry ?? "#"} className={cx("project-tab-link", navClasses.pipeline_registry)}>Registry</a>
            <a href={navLinks.pipelines_webhooks ?? "#"} className={cx("project-tab-link", navClasses.pipeline_webhooks)}>Webhooks</a>
            <a href={navLinks.pipelines_schedules ?? "#"} className={cx("project-tab-link", navClasses.pipeline_schedules)}>Schedules</a>
            <a href={navLinks.pipelines_manual ?? "#"} className={cx("project-tab-link", navClasses.pipeline_manual)}>Manual</a>
            <a href={navLinks.pipelines_functions ?? "#"} className={cx("project-tab-link", navClasses.pipeline_functions)}>Functions</a>
          </nav>

          <section className="project-workspace-body">
            <div className="project-registry-shell">
              <div className="project-surface-toolbar">
                <div className="project-inline-path">
                  <span className="project-inline-path-label">Path</span>
                  {breadcrumbs.map((crumb, index) => (
                    <span key={`${crumb?.path ?? "root"}-${index}`} className="project-inline-path-item">
                      {crumb?.show_divider ? <span className="project-inline-path-divider">/</span> : null}
                      <a href={crumb?.path ?? "#"} className="project-inline-path-link">{crumb?.name ?? "/"}</a>
                    </span>
                  ))}
                </div>
                <a href={registry?.editor_href ?? "#"} className="project-inline-chip project-inline-chip-accent">Open Editor</a>
              </div>

              <section className="project-content-section">
                <div className="project-content-head">
                  <div>
                    <p className="project-content-title">{input?.page_title}</p>
                    <p className="project-content-copy">{input?.page_subtitle}</p>
                  </div>
                </div>
                <div className="project-content-body">
                  <div className="project-entry-grid">
                    {folders.map((folder, index) => (
                      <a key={`${folder?.path ?? "folder"}-${index}`} href={folder?.path ?? "#"} className="project-entry-card">
                        <span className="project-entry-icon">
                          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
                            <path d="M3 7.5A1.5 1.5 0 014.5 6h4l1.5 2h9A1.5 1.5 0 0120.5 9.5v7A1.5 1.5 0 0119 18H4.5A1.5 1.5 0 013 16.5v-9z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/>
                          </svg>
                        </span>
                        <div className="project-entry-content">
                          <span className="project-list-title">{folder?.name}</span>
                          <p className="project-card-meta">Folder</p>
                        </div>
                      </a>
                    ))}

                    {pipelines.map((item, index) => (
                      <a key={`${item?.id ?? item?.name ?? "pipeline"}-${index}`} href={item?.edit_href ?? "#"} className="project-entry-card">
                        <span className="project-entry-icon">
                          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
                            <circle cx="6" cy="12" r="2" stroke="currentColor" strokeWidth="1.7"/>
                            <circle cx="18" cy="6" r="2" stroke="currentColor" strokeWidth="1.7"/>
                            <circle cx="18" cy="18" r="2" stroke="currentColor" strokeWidth="1.7"/>
                            <path d="M8 12h4l4-6M12 12h4l-4 6" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"/>
                          </svg>
                        </span>
                        <div className="project-entry-content">
                          <div className="flex items-start justify-between gap-3">
                            <div className="min-w-0">
                              <p className="project-list-title">{item?.title}</p>
                              <p className="project-card-meta">{item?.name}</p>
                            </div>
                            <span className="project-inline-chip">{item?.trigger_kind}</span>
                          </div>
                          <p className="project-card-copy">{item?.description}</p>
                          <p className="project-card-meta">{item?.file_rel_path}</p>
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
    </>
  );
}
