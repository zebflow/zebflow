import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";

/**
 * The project's sections (`docs/contracts/addressing.md` §2c): each root and
 * its dev host, so a webhook under a root links to the host that answers it.
 * Empty for a project without a root, which is every project but a few.
 */
export function useProjectSections(owner: string, project: string): any[] {
  const [sections, setSections] = useState<any[]>([]);
  useEffect(() => {
    if (!owner || !project) return;
    let live = true;
    requestJson(`/api/projects/${encodeURIComponent(owner)}/${encodeURIComponent(project)}/settings/addressing`)
      .then((body) => {
        const data = body?.data ?? body?.section?.data ?? {};
        if (live && Array.isArray(data?.sections)) setSections(data.sections);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [owner, project]);
  return sections;
}
