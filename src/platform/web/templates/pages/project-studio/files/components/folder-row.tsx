import { cx } from "zeb/react";
import Badge from "@/components/ui/badge";
import { LockIcon, LockOpenIcon } from "@/pages/project-studio/components/icons";
import { FsFolderIcon, FsTrashIcon } from "@/pages/project-studio/files/components/file-icons";
import { AccessToggleButton, RowIconButton } from "@/pages/project-studio/files/components/access-controls";
import { ExposedInsideMark, ExposureMark, ownRule, rulesInside } from "@/pages/project-studio/files/components/exposure-mark";

/** One folder in the explorer. `ctx` is what every row shares; `actions` is what this row can do. */
export function FolderRow({ folder, ctx, actions }) {
  const exposed = folder.access && folder.access !== "private";
  const rule = ownRule(ctx.rules, folder.path);
  const inside = exposed ? [] : rulesInside(ctx.rules, folder.path);
  const accessTitle = exposed ? "Make folder private" : "Make folder public";
  return (
    <div
      className={cx(
        "group flex items-center gap-2 min-h-[2.1rem] px-2 py-1.5 rounded-md border border-dashed text-muted-foreground text-[0.8rem] hover:bg-muted hover:text-foreground transition-colors",
        folder.access === "public_execute" ? "border-red-500/50" : exposed ? "border-amber-400/50" : "border-border",
      )}
    >
      <FsFolderIcon />
      <button
        type="button"
        className="flex-1 min-w-0 truncate text-left font-medium text-[0.78rem] text-foreground bg-transparent border-0 p-0 cursor-pointer"
        onClick={actions.open}
      >
        {folder.name}
      </button>
      <ExposureMark access={folder.access} inherited={exposed && !rule} serve={rule?.serve ?? []} />
      <ExposedInsideMark count={inside.length} execute={inside.some((r) => r.access === "public_execute")} />
      {exposed ? null : (
        <Badge variant="outline" className="text-[0.65rem] shrink-0">private</Badge>
      )}
      <AccessToggleButton
        item={folder}
        scope="prefix"
        ctx={ctx}
        title={accessTitle}
        className="flex items-center justify-center w-6 h-6 rounded shrink-0 text-muted-foreground transition-colors hover:text-primary hover:bg-primary/10"
        onToggle={actions.toggleAccess}
      >
        {exposed ? <LockOpenIcon className="w-3.5 h-3.5" /> : <LockIcon className="w-3.5 h-3.5" />}
      </AccessToggleButton>
      <RowIconButton title="Serve as a site…" onClick={actions.serve}>
        <span className="text-[0.62rem] font-bold">www</span>
      </RowIconButton>
      {folder.protected ? (
        <Badge variant="outline" className="text-[0.65rem] shrink-0">protected</Badge>
      ) : (
        <RowIconButton title="Delete folder" tone="danger" onClick={actions.remove}>
          <FsTrashIcon />
        </RowIconButton>
      )}
    </div>
  );
}
