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

export default function Page(input) {
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
        <div className="flex min-h-full flex-col gap-3 px-3.5 py-3">
          <header className="border-b border-border pb-3">
            <p className="font-mono text-[0.66rem] font-semibold uppercase tracking-[0.16em] text-body-muted">
              Infrastructure
            </p>
            <h1 className="mt-1 text-[1.05rem] font-semibold leading-tight text-body">
              Controller and office topology
            </h1>
            <p className="mt-1 max-w-3xl text-[0.78rem] leading-5 text-body-soft">
              Current office inventory, project runtime placement, and controller
              registration state for this project.
            </p>
          </header>
          <section className="grid gap-3 lg:grid-cols-[minmax(0,1.35fr)_minmax(22rem,0.8fr)]">
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
