import { cx } from "zeb";

interface PaginationProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Pagination({ className, children, ...rest }: PaginationProps) {
  return (
    <nav
      role="navigation"
      aria-label="pagination"
      className={cx("mx-auto flex w-full justify-center", className)}
      {...rest}
    >
      {children}
    </nav>
  );
}

export function PaginationContent({ className, children, ...rest }: PaginationProps) {
  return (
    <ul className={cx("flex flex-row items-center gap-1", className)} {...rest}>
      {children}
    </ul>
  );
}

export function PaginationItem({ className, children, ...rest }: PaginationProps) {
  return (
    <li className={cx("", className)} {...rest}>
      {children}
    </li>
  );
}

interface PaginationLinkProps extends PaginationProps {
  href?: string;
  isActive?: boolean;
  size?: "default" | "icon";
}

export function PaginationLink({ className, isActive, href, children, size = "icon", ...rest }: PaginationLinkProps) {
  return (
    <a
      href={href ?? "#"}
      aria-current={isActive ? "page" : undefined}
      className={cx(
        "inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-950 hover:bg-gray-100 hover:text-gray-900",
        size === "icon" ? "h-9 w-9" : "h-9 px-4",
        isActive ? "border border-gray-200 bg-white shadow-sm" : "",
        className
      )}
      {...rest}
    >
      {children}
    </a>
  );
}

export function PaginationPrevious({ className, href, ...rest }: PaginationLinkProps) {
  return (
    <PaginationLink href={href} size="default" className={cx("gap-1 pl-2.5", className)} {...rest}>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
        <path d="m15 18-6-6 6-6" />
      </svg>
      <span>Previous</span>
    </PaginationLink>
  );
}

export function PaginationNext({ className, href, ...rest }: PaginationLinkProps) {
  return (
    <PaginationLink href={href} size="default" className={cx("gap-1 pr-2.5", className)} {...rest}>
      <span>Next</span>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
        <path d="m9 18 6-6-6-6" />
      </svg>
    </PaginationLink>
  );
}

export function PaginationEllipsis({ className, ...rest }: PaginationProps) {
  return (
    <span
      aria-hidden
      className={cx("flex h-9 w-9 items-center justify-center", className)}
      {...rest}
    >
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
        <circle cx="12" cy="12" r="1" />
        <circle cx="19" cy="12" r="1" />
        <circle cx="5" cy="12" r="1" />
      </svg>
      <span className="sr-only">More pages</span>
    </span>
  );
}

export default Pagination;
