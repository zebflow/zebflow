import { useState } from "zeb/react";
import OfficeMeta from "@/pages/home/components/office-meta";
import ProjectRow from "@/pages/home/components/project-row";
import OpenRemoteDialog from "@/pages/home/components/open-remote-dialog";
import HiddenProjects from "@/pages/home/components/hidden-projects";

/**
 * The controller's one directory (`offices.md` §3a): each office on the left,
 * the projects it holds on the right. On an office that has not joined
 * anything, it is simply this office and its projects. Projects hidden from
 * home are not in an office's list; they wait under the hidden section below.
 */
export default function OfficeDirectory({ offices, hiddenProjects, canOpenRemote }) {
  const [target, setTarget] = useState(null);
  const rows = Array.isArray(offices) ? offices : [];

  return (
    <>
      <section className="overflow-hidden rounded-[14px] border border-border bg-popover shadow-sm">
        {rows.map((office, index) => {
          const projects = Array.isArray(office?.projects) ? office.projects : [];
          const remote = !office?.local;
          const openRemote = remote && canOpenRemote ? (project) => setTarget({ office, project }) : null;
          return (
            <div
              key={`${office?.id ?? "office"}-${index}`}
              className={`grid gap-4 px-5 py-4 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] ${index > 0 ? "border-t border-border" : ""}`}
            >
              <OfficeMeta office={office} />
              <div className="min-w-0 divide-y divide-border">
                {projects.length > 0 ? (
                  projects.map((item, projectIndex) => (
                    <ProjectRow
                      key={`${item?.owner ?? ""}/${item?.project ?? projectIndex}`}
                      item={item}
                      remote={remote}
                      onOpenRemote={openRemote}
                    />
                  ))
                ) : (
                  <p className="py-2 text-[13px] text-muted-foreground">No projects yet.</p>
                )}
              </div>
            </div>
          );
        })}
      </section>
      <HiddenProjects items={hiddenProjects} />
      <OpenRemoteDialog target={target} onClose={() => setTarget(null)} />
    </>
  );
}
