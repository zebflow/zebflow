import { cx } from "zeb/react";

/** The one status line the explorer and the upload dialog share. */
export function FilesMessage({ text, tone, className = "" }) {
  if (!text) return null;
  return (
    <div className={cx(
      "rounded border px-3 py-2 text-[0.76rem]",
      tone === "error" && "border-red-500/40 bg-red-500/10 text-red-300",
      tone === "ok" && "border-emerald-500/40 bg-emerald-500/10 text-emerald-300",
      tone === "muted" && "border-border bg-muted text-muted-foreground",
      className,
    )}>
      {text}
    </div>
  );
}
