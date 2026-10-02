import { cx } from "zeb/react";
import Badge from "@/components/ui/badge";
import { formatBytes } from "@/components/lib/format";
import { LockIcon, LockOpenIcon } from "@/pages/project-studio/components/icons";
import { FsFileIcon, FsImageIcon, FsTrashIcon } from "@/pages/project-studio/files/components/file-icons";
import { AccessToggleButton, RowIconButton } from "@/pages/project-studio/files/components/access-controls";
import { ExposureMark, ownRule } from "@/pages/project-studio/files/components/exposure-mark";

const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "gif", "webp", "svg", "avif", "bmp"];

/** One file in the explorer. `ctx` is what every row shares; `actions` is what this row can do. */
export function FileRow({ file, ctx, actions }) {
  const ext = (file.name?.split(".").pop() ?? "").toLowerCase();
  const exposed = file.access && file.access !== "private";
  const rule = ownRule(ctx.rules, file.path);
  const accessTitle = exposed ? "Make file private" : "Make file public";
  return (
    <div
      className={cx(
        "group flex items-center gap-2 min-h-[2.1rem] px-2 py-1.5 rounded-md border bg-muted transition-colors",
        file.access === "public_execute" ? "border-red-500/50" : exposed ? "border-amber-400/50" : "border-border",
      )}
    >
      {IMAGE_EXTENSIONS.includes(ext) ? <FsImageIcon /> : <FsFileIcon />}
      <a
        className="flex-1 min-w-0 truncate font-medium text-[0.78rem] text-foreground hover:text-primary hover:underline"
        href={file.url}
        target="_blank"
        rel="noopener"
      >
        {file.name}
      </a>
      <span className="text-[0.7rem] text-muted-foreground whitespace-nowrap shrink-0">
        {formatBytes(file.size)}
        {file.modified ? ` · ${new Date(file.modified * 1000).toLocaleDateString()}` : ""}
      </span>
      <ExposureMark access={file.access} inherited={exposed && !rule} serve={rule?.serve ?? []} />
      {exposed ? null : (
        <Badge variant="outline" className="text-[0.65rem] shrink-0">private</Badge>
      )}
      <AccessToggleButton
        item={file}
        scope="object"
        ctx={ctx}
        title={accessTitle}
        className="flex items-center justify-center w-6 h-6 rounded shrink-0 text-muted-foreground transition-colors hover:text-primary hover:bg-primary/10"
        onToggle={actions.toggleAccess}
      >
        {exposed ? <LockOpenIcon className="w-3.5 h-3.5" /> : <LockIcon className="w-3.5 h-3.5" />}
      </AccessToggleButton>
      <RowIconButton title="Delete file" tone="danger" onClick={actions.remove}>
        <FsTrashIcon />
      </RowIconButton>
    </div>
  );
}
