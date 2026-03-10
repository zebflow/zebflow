import { cx } from "rwe";

export default function DropdownMenu({ className, children, ...rest }) {
  return (
    <details className={cx("relative inline-block group", className)} data-dropdown-menu="true" {...rest}>
      {children}
    </details>
  );
}
