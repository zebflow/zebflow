import { cx } from "zeb/react";

/**
 * Flips one path between private and public read. A real form underneath, so
 * the toggle still works before the page hydrates; once it has, the click is
 * handled in place.
 */
export function AccessToggleButton({ item, scope, ctx, title, className, onToggle, children }) {
  const nextAccess = item?.access && item.access !== "private" ? "private" : "public_read";
  return (
    <form method="post" action={ctx.accessAction || ""} className="contents">
      <input type="hidden" name="path" value={item?.path ?? ""} />
      <input type="hidden" name="access" value={nextAccess} />
      <input type="hidden" name="scope" value={scope} />
      <input type="hidden" name="return_to" value={ctx.returnTo ?? ""} />
      <button
        type="submit"
        className={className}
        title={title}
        aria-label={title}
        disabled={ctx.busy === `access:${item?.path}`}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onToggle?.();
        }}
      >
        {children}
      </button>
    </form>
  );
}

export function RowIconButton({ children, title, tone = "default", onClick, disabled = false }) {
  return (
    <button
      type="button"
      className={cx(
        "flex items-center justify-center w-6 h-6 rounded shrink-0 text-muted-foreground transition-colors",
        tone === "danger" && "hover:text-red-400 hover:bg-red-400/10",
        tone !== "danger" && "hover:text-primary hover:bg-primary/10",
        disabled && "opacity-50 pointer-events-none",
      )}
      title={title}
      aria-label={title}
      onClick={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onClick?.();
      }}
      disabled={disabled}
    >
      {children}
    </button>
  );
}
