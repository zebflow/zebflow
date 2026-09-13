import { cx } from "zeb/react";

/**
 * Empty — shadcn/ui's empty-state block, on Zebflow's engine.
 *
 * `EmptyMedia`'s icon-sizing (`[&_svg]:…`, `[&_svg:not(…)]:size-6`) and
 * `EmptyDescription`'s link styling (`[&>a]:underline …`) are arbitrary
 * descendant selectors this engine doesn't compile — size an icon child
 * yourself (`className="size-6"`) and style a link child yourself
 * (`className="underline underline-offset-4 hover:text-primary"`).
 */

export function Empty({ className, children, ...rest }) {
  return (
    <div
      data-slot="empty"
      className={cx(
        "flex min-w-0 flex-1 flex-col items-center justify-center gap-6 rounded-lg border-dashed p-6 text-center text-balance md:p-12",
        className
      )}
      {...rest}
    >
      {children}
    </div>
  );
}

export function EmptyHeader({ className, children, ...rest }) {
  return (
    <div data-slot="empty-header" className={cx("flex max-w-sm flex-col items-center gap-2 text-center", className)} {...rest}>
      {children}
    </div>
  );
}

const EMPTY_MEDIA_VARIANTS = {
  default: "bg-transparent",
  icon: "flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-foreground",
};

export function EmptyMedia({ className, variant = "default", children, ...rest }) {
  return (
    <div
      data-slot="empty-icon"
      data-variant={variant}
      className={cx("mb-2 flex shrink-0 items-center justify-center", EMPTY_MEDIA_VARIANTS[variant] ?? EMPTY_MEDIA_VARIANTS.default, className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export function EmptyTitle({ className, children, ...rest }) {
  return (
    <div data-slot="empty-title" className={cx("text-lg font-medium tracking-tight", className)} {...rest}>
      {children}
    </div>
  );
}

export function EmptyDescription({ className, children, ...rest }) {
  return (
    <div data-slot="empty-description" className={cx("text-sm leading-relaxed text-muted-foreground", className)} {...rest}>
      {children}
    </div>
  );
}

export function EmptyContent({ className, children, ...rest }) {
  return (
    <div
      data-slot="empty-content"
      className={cx("flex w-full max-w-sm min-w-0 flex-col items-center gap-4 text-sm text-balance", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export default Empty;
