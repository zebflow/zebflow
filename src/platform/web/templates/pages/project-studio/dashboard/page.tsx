import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { initDashboardBehavior } from "@/pages/project-studio/dashboard/dashboard-behavior";
import { Link } from "zeb/react";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import MigrationNotice from "@/components/ui/migration-notice";
import SystemPanel from "@/pages/project-studio/dashboard/components/system-panel";

export const page = {
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}


export default function Page(input) {
  initDashboardBehavior();
  // Present only for a superadmin: the system API answers nobody else.
  const apiUrl = input?.api?.system_info ?? "";
  return (
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu="Dashboard"
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={input?.nav?.links?.dashboard ?? "#"} active>System</StudioTabLink>
        </StudioTabNav>
        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-background">
          <div className="project-content-wrap">

            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">System</p>
                  <p className="project-content-copy">System health, resource usage, and installed capabilities.</p>
                </div>
                <span data-dash-status className="project-inline-chip"></span>
              </div>
              <MigrationNotice notice={input?.migration} className="mt-3" />
            </section>

            {apiUrl ? (
              <SystemPanel apiUrl={apiUrl} />
            ) : (
              <section className="project-content-section">
                <div className="project-content-body">
                  <p className="project-content-copy">
                    System health is shown to the instance administrator only.
                  </p>
                </div>
              </section>
            )}
          </div>
        </section>
      </div>
    </ProjectStudioShell>
  );
}
