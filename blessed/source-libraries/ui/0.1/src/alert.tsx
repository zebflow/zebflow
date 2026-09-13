import { cx } from "zeb/react";

/**
 * Alert — shadcn/ui's alert, on Zebflow's engine.
 *
 * Upstream auto-shifts its grid columns when an `<svg>` child is present
 * (`has-[>svg]:grid-cols-…`, `[&>svg]:size-4`) — arbitrary-selector variants
 * this engine does not compile. Instead the base grid is always
 * `grid-cols-[auto_1fr]`: with no icon the first column collapses to zero
 * width on its own, and an icon that IS passed needs
 * `className="col-start-1 row-span-2 size-4"` set on it explicitly. The
 * destructive variant's tinted description (upstream's
 * `*:data-[slot=alert-description]:text-destructive/90`, a child-selector)
 * is dropped too — pass `className="text-destructive/90"` to
 * `AlertDescription` directly when you want it.
 */

const VARIANTS = {
  default: "bg-card text-card-foreground",
  destructive: "bg-card text-destructive",
};

export function Alert({ className, variant = "default", children, ...rest }) {
  return (
    <div
      data-slot="alert"
      data-variant={variant}
      role="alert"
      className={cx(
        "relative grid w-full grid-cols-[auto_1fr] items-start gap-x-3 gap-y-0.5 rounded-lg border px-4 py-3 text-sm",
        VARIANTS[variant] ?? VARIANTS.default,
        className
      )}
      {...rest}
    >
      {children}
    </div>
  );
}

export function AlertTitle({ className, children, ...rest }) {
  return (
    <div
      data-slot="alert-title"
      className={cx("col-start-2 line-clamp-1 min-h-4 font-medium tracking-tight", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export function AlertDescription({ className, children, ...rest }) {
  return (
    <div
      data-slot="alert-description"
      className={cx("col-start-2 grid justify-items-start gap-1 text-sm text-muted-foreground", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export default Alert;
