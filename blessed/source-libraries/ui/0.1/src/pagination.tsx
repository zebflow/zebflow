import { cx } from "zeb/react";
import { buttonVariants } from "zeb/ui/button";

/**
 * Pagination — shadcn/ui's pagination, on Zebflow's engine.
 *
 * `PaginationLink` reuses `zeb/ui/button`'s `buttonVariants` exactly like
 * upstream reuses its own `buttonVariants`. Icons are inline `<svg>` (no
 * lucide).
 */

export function Pagination({ className, children, ...rest }) {
  return (
    <nav role="navigation" aria-label="pagination" data-slot="pagination" className={cx("mx-auto flex w-full justify-center", className)} {...rest}>
      {children}
    </nav>
  );
}

export function PaginationContent({ className, children, ...rest }) {
  return (
    <ul data-slot="pagination-content" className={cx("flex flex-row items-center gap-1", className)} {...rest}>
      {children}
    </ul>
  );
}

export function PaginationItem({ children, ...rest }) {
  return (
    <li data-slot="pagination-item" {...rest}>
      {children}
    </li>
  );
}

export function PaginationLink({ className, isActive, size = "icon", children, ...rest }) {
  return (
    <a
      aria-current={isActive ? "page" : undefined}
      data-slot="pagination-link"
      data-active={isActive}
      className={buttonVariants({ variant: isActive ? "outline" : "ghost", size, className })}
      {...rest}
    >
      {children}
    </a>
  );
}

export function PaginationPrevious({ className, ...rest }) {
  return (
    <PaginationLink aria-label="Go to previous page" size="default" className={cx("gap-1 px-2.5 sm:pl-2.5", className)} {...rest}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
        <path d="m15 18-6-6 6-6" />
      </svg>
      <span className="hidden sm:block">Previous</span>
    </PaginationLink>
  );
}

export function PaginationNext({ className, ...rest }) {
  return (
    <PaginationLink aria-label="Go to next page" size="default" className={cx("gap-1 px-2.5 sm:pr-2.5", className)} {...rest}>
      <span className="hidden sm:block">Next</span>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
        <path d="m9 18 6-6-6-6" />
      </svg>
    </PaginationLink>
  );
}

export function PaginationEllipsis({ className, ...rest }) {
  return (
    <span aria-hidden data-slot="pagination-ellipsis" className={cx("flex size-9 items-center justify-center", className)} {...rest}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
        <circle cx="12" cy="12" r="1" />
        <circle cx="19" cy="12" r="1" />
        <circle cx="5" cy="12" r="1" />
      </svg>
      <span className="sr-only">More pages</span>
    </span>
  );
}

export default Pagination;
