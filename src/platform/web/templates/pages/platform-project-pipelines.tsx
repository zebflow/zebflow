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
  // Pipeline registry keeps separators explicit in markup, not CSS pseudo-content.
  return (
<Page>
    <ProjectStudioShell
      projectHref="{input.project_href}"
      projectLabel="{input.title}"
      currentMenu="{input.current_menu}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href="{input.nav.links.pipelines_registry}" className="project-tab-link {input.nav.classes.pipeline_registry}">Registry</a>
          <a href="{input.nav.links.pipelines_webhooks}" className="project-tab-link {input.nav.classes.pipeline_webhooks}">Webhooks</a>
          <a href="{input.nav.links.pipelines_schedules}" className="project-tab-link {input.nav.classes.pipeline_schedules}">Schedules</a>
          <a href="{input.nav.links.pipelines_functions}" className="project-tab-link {input.nav.classes.pipeline_functions}">Functions</a>
        </nav>

        <section className="project-workspace-body">
          <div zShow="input.is_registry" className="project-registry-shell">
            <div className="project-surface-toolbar">
              <div className="project-inline-path">
                <span className="project-inline-path-label">Path</span>
                {/* Explicit separators keep breadcrumb rendering deterministic in SSR and runtime output. */}
                {/* <span className="project-inline-path-divider">/</span> */}
                <span zFor="crumb in input.registry.breadcrumbs" className="project-inline-path-item">
                  <span zShow="crumb.show_divider" className="project-inline-path-divider">/</span>
                  <a href="{crumb.path}" className="project-inline-path-link">{crumb.name}</a>
                </span>
              </div>
              <details className="project-action-menu">
                <summary className="project-inline-chip project-inline-chip-accent">+New</summary>
                <div className="project-action-menu-panel">
                  <a href="{input.nav.links.pipelines_registry}" className="project-action-menu-item">Pipeline</a>
                  <a href="{input.nav.links.pipelines_webhooks}" className="project-action-menu-item">Webhook</a>
                  <a href="{input.nav.links.pipelines_schedules}" className="project-action-menu-item">Schedule</a>
                  <a href="{input.nav.links.pipelines_functions}" className="project-action-menu-item">Function</a>
                </div>
              </details>
            </div>

            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">{input.page_title}</p>
                  <p className="project-content-copy">{input.page_subtitle}</p>
                </div>
              </div>
              <div className="project-content-body">
                <div className="project-entry-grid">
                  <a zFor="folder in input.registry.folders" href="{folder.path}" className="project-entry-card">
                    <span className="project-entry-icon">
                      <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
                        <path d="M3 7.5A1.5 1.5 0 014.5 6h4l1.5 2h9A1.5 1.5 0 0120.5 9.5v7A1.5 1.5 0 0119 18H4.5A1.5 1.5 0 013 16.5v-9z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                      </svg>
                    </span>
                    <div className="project-entry-content">
                      <span className="project-list-title">{folder.name}</span>
                      <p className="project-card-meta">Folder</p>
                    </div>
                  </a>

                  <article zFor="item in input.registry.pipelines" className="project-entry-card">
                    <span className="project-entry-icon">
                      <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
                        <circle cx="6" cy="12" r="2" stroke="currentColor" stroke-width="1.7"/>
                        <circle cx="18" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                        <circle cx="18" cy="18" r="2" stroke="currentColor" stroke-width="1.7"/>
                        <path d="M8 12h4l4-6M12 12h4l-4 6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
                      </svg>
                    </span>
                    <div className="project-entry-content">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <p className="project-list-title">{item.title}</p>
                          <p className="project-card-meta">{item.name}</p>
                        </div>
                        <span className="project-inline-chip">{item.trigger_kind}</span>
                      </div>
                      <p className="project-card-copy">{item.description}</p>
                      <p className="project-card-meta">{item.file_rel_path}</p>
                    </div>
                  </article>
                </div>
              </div>
            </section>
          </div>

          <div zShow="input.is_non_registry" className="project-flat-list">
            <div className="project-surface-panel-head">{input.page_title}</div>
            <div className="project-list">
              <article zFor="item in input.pipeline_items" className="project-list-item">
                <p className="project-list-title">{item.name}</p>
                <p className="project-card-copy">{item.description}</p>
              </article>
            </div>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
