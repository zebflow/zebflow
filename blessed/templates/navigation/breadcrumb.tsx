import { cx } from "zeb";

interface BreadcrumbProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Breadcrumb({ className, children, ...rest }: BreadcrumbProps) {
  return (
    <nav aria-label="breadcrumb" className={cx("", className)} {...rest}>
      {children}
    </nav>
  );
}

export function BreadcrumbList({ className, children, ...rest }: BreadcrumbProps) {
  return (
    <ol
      className={cx(
        "flex flex-wrap items-center gap-1.5 break-words text-sm text-gray-500 sm:gap-2.5",
        className
      )}
      {...rest}
    >
      {children}
    </ol>
  );
}

export function BreadcrumbItem({ className, children, ...rest }: BreadcrumbProps) {
  return (
    <li className={cx("inline-flex items-center gap-1.5", className)} {...rest}>
      {children}
    </li>
  );
}

interface BreadcrumbLinkProps extends BreadcrumbProps {
  href?: string;
  asChild?: boolean;
}

export function BreadcrumbLink({ className, children, href, ...rest }: BreadcrumbLinkProps) {
  return (
    <a
      href={href}
      className={cx("transition-colors hover:text-gray-900", className)}
      {...rest}
    >
      {children}
    </a>
  );
}

export function BreadcrumbPage({ className, children, ...rest }: BreadcrumbProps) {
  return (
    <span
      role="link"
      aria-disabled="true"
      aria-current="page"
      className={cx("font-normal text-gray-900", className)}
      {...rest}
    >
      {children}
    </span>
  );
}

export function BreadcrumbSeparator({ className, children, ...rest }: BreadcrumbProps) {
  return (
    <li role="presentation" aria-hidden="true" className={cx("[&>svg]:w-3.5 [&>svg]:h-3.5", className)} {...rest}>
      {children ?? (
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="w-3.5 h-3.5">
          <path d="m9 18 6-6-6-6" />
        </svg>
      )}
    </li>
  );
}

export function BreadcrumbEllipsis({ className, ...rest }: BreadcrumbProps) {
  return (
    <span
      role="presentation"
      aria-hidden="true"
      className={cx("flex h-9 w-9 items-center justify-center", className)}
      {...rest}
    >
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
        <circle cx="12" cy="12" r="1" />
        <circle cx="19" cy="12" r="1" />
        <circle cx="5" cy="12" r="1" />
      </svg>
      <span className="sr-only">More</span>
    </span>
  );
}

export default Breadcrumb;
