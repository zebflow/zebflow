import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initDashboardBehavior } from "@/components/behavior/project-dashboard";
import { Link } from "zeb";

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
  initDashboardBehavior();
  const apiUrl = (input?.api && input.api.system_info) || "/api/system/info";
  return (
<Page>
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu="Dashboard"
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <Link href={input?.nav?.links?.dashboard ?? "#"} className="project-tab-link is-active">System</Link>
        </nav>
        <section className="project-workspace-body">
          <div className="project-content-wrap">

            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">System</p>
                  <p className="project-content-copy">System health, resource usage, and installed capabilities.</p>
                </div>
                <span data-dash-status className="project-inline-chip"></span>
              </div>
            </section>

            <div
              data-dashboard-root
              data-api-system-info={apiUrl}
            >

              <section className="project-content-section">
                <div className="project-content-body">
                <p className="project-content-subtitle">System</p>
                <div className="project-card-grid cols-2">

                  <div className="project-card">
                    <h3 className="project-card-title">CPU</h3>
                    <p data-cpu-brand className="project-card-copy"></p>
                    <div className="dash-meta-row">
                      <span data-cpu-cores className="dash-meta-label"></span>
                      <span data-cpu-pct className="dash-metric-value"></span>
                    </div>
                    <div className="dash-bar-track">
                      <div data-cpu-bar className="dash-bar-fill"></div>
                    </div>
                  </div>

                  <div className="project-card">
                    <h3 className="project-card-title">Memory</h3>
                    <div className="dash-meta-row">
                      <span data-mem-used className="dash-meta-label"></span>
                      <span className="dash-meta-sep"> / </span>
                      <span data-mem-total className="dash-meta-label"></span>
                      <span data-mem-pct className="dash-metric-value"></span>
                    </div>
                    <div className="dash-bar-track">
                      <div data-mem-bar className="dash-bar-fill"></div>
                    </div>
                  </div>

                  <div className="project-card">
                    <h3 className="project-card-title">Disk</h3>
                    <div className="dash-meta-row">
                      <span data-disk-used className="dash-meta-label"></span>
                      <span className="dash-meta-sep"> / </span>
                      <span data-disk-total className="dash-meta-label"></span>
                      <span data-disk-pct className="dash-metric-value"></span>
                    </div>
                    <div className="dash-bar-track">
                      <div data-disk-bar className="dash-bar-fill"></div>
                    </div>
                  </div>

                  <div className="project-card">
                    <h3 className="project-card-title">OS</h3>
                    <p data-os-name className="project-card-copy"></p>
                    <div className="dash-meta-row">
                      <span data-os-variant className="project-inline-chip"></span>
                      <span data-os-arch className="dash-meta-label"></span>
                    </div>
                    <p data-os-hostname className="dash-meta-label"></p>
                    <p data-env-container className="dash-meta-label"></p>
                  </div>

                </div>
                </div>
              </section>

              <section className="project-content-section">
                <div className="project-content-body">
                <p className="project-content-subtitle">Process</p>
                <div className="project-card-grid cols-2">

                  <div className="project-card">
                    <h3 className="project-card-title">Server Process</h3>
                    <div className="dash-stat-list">
                      <div className="dash-stat-row">
                        <span className="dash-stat-key">PID</span>
                        <span data-proc-pid className="dash-stat-val"></span>
                      </div>
                      <div className="dash-stat-row">
                        <span className="dash-stat-key">CPU</span>
                        <span data-proc-cpu className="dash-stat-val"></span>
                      </div>
                      <div className="dash-stat-row">
                        <span className="dash-stat-key">Memory</span>
                        <span data-proc-mem className="dash-stat-val"></span>
                      </div>
                      <div className="dash-stat-row">
                        <span className="dash-stat-key">Threads</span>
                        <span data-proc-threads className="dash-stat-val"></span>
                      </div>
                      <div className="dash-stat-row">
                        <span className="dash-stat-key">Uptime</span>
                        <span data-proc-uptime className="dash-stat-val"></span>
                      </div>
                    </div>
                  </div>

                </div>
                </div>
              </section>

              <section className="project-content-section">
                <div className="project-content-body">
                <p className="project-content-subtitle">Capabilities</p>
                <div className="project-card">
                  <div data-caps-grid className="dash-cap-grid"></div>
                </div>
                </div>
              </section>

            </div>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
