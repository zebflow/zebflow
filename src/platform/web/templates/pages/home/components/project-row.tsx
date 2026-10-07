import { Link } from "zeb/react";
import Button from "@/components/ui/button";
import MigrationNotice from "@/components/ui/migration-notice";
import ProjectRowMenu from "@/pages/home/components/project-row-menu";

/**
 * One project in an office's list.
 *
 * A project on this office opens here. A project on another office opens
 * through `onOpenRemote`, which asks where to reach that office — or offers
 * nothing when the viewer cannot be vouched in (`offices.md` §3a).
 */
export default function ProjectRow({ item, remote, onOpenRemote }) {
  const title = item?.title || item?.project;

  return (
    <div className="flex items-center justify-between gap-3 py-2">
      <div className="min-w-0">
        <p className="truncate text-[14px] font-medium text-foreground">{title}</p>
        <p className="truncate font-mono text-[11px] text-muted-foreground">
          {item?.owner}/{item?.project}
          {remote && item?.office_label ? ` · ${item.office_label}` : ""}
        </p>
        <MigrationNotice notice={item?.migration} className="mt-1 px-2 py-0.5 text-[12px]" />
      </div>
      <div className="flex shrink-0 gap-2">
        {remote ? (
          onOpenRemote ? (
            <Button type="button" variant="outline" size="sm" onClick={() => onOpenRemote(item)}>
              Open
            </Button>
          ) : null
        ) : (
          <>
            {item?.open_app_path ? (
              <Link href={item.open_app_path} className="inline-flex hover:no-underline">
                <Button as="span" variant="primary" size="sm">
                  Play
                </Button>
              </Link>
            ) : null}
            <Link href={item?.edit_path || item?.path || "#"} className="inline-flex hover:no-underline">
              <Button as="span" variant="outline" size="sm">
                Edit
              </Button>
            </Link>
            <ProjectRowMenu item={item} />
          </>
        )}
      </div>
    </div>
  );
}
