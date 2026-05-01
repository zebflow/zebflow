/**
 * Infrastructure landing page scaffold.
 *
 * This page is the future home of cluster-aware Studio UX:
 *
 * - office inventory and health
 * - per-project placement policy
 * - join/bootstrap guidance
 * - later runtime drains, tags, and capacity views
 *
 * The route is intentionally not wired yet. The file exists now so the product structure mirrors
 * the cluster/control-plane architecture before the UI behavior is implemented.
 */

import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { ProjectRuntimePanel } from "@/pages/project-studio/infrastructure/components/project-runtime-panel";
import { WorkersPanel } from "@/pages/project-studio/infrastructure/components/workers-panel";

export const page = {
  html: { lang: "en" },
  body: { className: "font-sans" },
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

export default function InfrastructurePage(input) {
  const runtime = input?.runtime ?? {};
  const project = input?.project ?? {};

  return (
    <ProjectStudioShell
      projectHref={input?.project_href ?? `/projects/${project?.owner ?? ""}/${project?.project ?? ""}`}
      projectLabel={project?.title ?? input?.title}
      currentMenu="Infrastructure"
      owner={project?.owner ?? ""}
      project={project?.project ?? ""}
      nav={input?.nav}
    >
      <main className="flex-1 min-h-0 overflow-auto">
        <div className="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-8">
          <header className="space-y-2">
            <p className="text-xs uppercase tracking-[0.24em] text-gray-500">
              Infrastructure
            </p>
            <h1 className="text-3xl font-semibold text-gray-900">
              Controller and office topology
            </h1>
            <p className="max-w-3xl text-sm leading-6 text-gray-600">
              This surface will manage office inventory, project placement, and
              controller bootstrap. The structure exists now so runtime work can land
              behind stable UI module boundaries.
            </p>
          </header>
          <section className="grid gap-6 lg:grid-cols-[1.25fr_0.95fr]">
            <WorkersPanel workers={runtime.workers} />
            <ProjectRuntimePanel
              placement={runtime.placement}
              summary={runtime.summary}
            />
          </section>
        </div>
      </main>
    </ProjectStudioShell>
  );
}
