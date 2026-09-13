import { cx } from "zeb/react";

/**
 * Breadcrumb — shadcn/ui's breadcrumb, on Zebflow's engine.
 *
 * `BreadcrumbLink` always renders an `<a>` — no `asChild`/`Slot` here, so
 * there is nothing to swap it for. Icons are inline `<svg>` (no lucide);
 * `BreadcrumbSeparator`'s auto-sizing of a custom child icon
 * (`[&>svg]:size-3.5`, a bracket selector) is dropped — size a custom
 * separator icon yourself.
 */

export function Breadcrumb({ children, ...rest }) {
  return (
    <nav data-slot="breadcrumb" aria-label="breadcrumb" {...rest}>
      {children}
    </nav>
  );
}

export function BreadcrumbList({ className, children, ...rest }) {
  return (
    <ol
      data-slot="breadcrumb-list"
      className={cx("flex flex-wrap items-center gap-1.5 text-sm break-words text-muted-foreground sm:gap-2.5", className)}
      {...rest}
    >
      {children}
    </ol>
  );
}

export function BreadcrumbItem({ className, children, ...rest }) {
  return (
    <li data-slot="breadcrumb-item" className={cx("inline-flex items-center gap-1.5", className)} {...rest}>
      {children}
    </li>
  );
}

export function BreadcrumbLink({ className, children, ...rest }) {
  return (
    <a data-slot="breadcrumb-link" className={cx("transition-colors hover:text-foreground", className)} {...rest}>
      {children}
    </a>
  );
}

export function BreadcrumbPage({ className, children, ...rest }) {
  return (
    <span
      data-slot="breadcrumb-page"
      role="link"
      aria-disabled="true"
      aria-current="page"
      className={cx("font-normal text-foreground", className)}
      {...rest}
    >
      {children}
    </span>
  );
}

export function BreadcrumbSeparator({ className, children, ...rest }) {
  return (
    <li data-slot="breadcrumb-separator" role="presentation" aria-hidden="true" className={cx(className)} {...rest}>
      {children ?? (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-3.5">
          <path d="m9 18 6-6-6-6" />
        </svg>
      )}
    </li>
  );
}

export function BreadcrumbEllipsis({ className, ...rest }) {
  return (
    <span data-slot="breadcrumb-ellipsis" role="presentation" aria-hidden="true" className={cx("flex size-9 items-center justify-center", className)} {...rest}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
        <circle cx="12" cy="12" r="1" />
        <circle cx="19" cy="12" r="1" />
        <circle cx="5" cy="12" r="1" />
      </svg>
      <span className="sr-only">More</span>
    </span>
  );
}

export default Breadcrumb;
