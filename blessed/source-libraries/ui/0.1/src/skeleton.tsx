import { cx } from "zeb/react";

/** Skeleton — shadcn/ui's loading placeholder, verbatim. */

export function Skeleton({ className, ...rest }) {
  return <div data-slot="skeleton" className={cx("animate-pulse rounded-md bg-accent", className)} {...rest} />;
}

export default Skeleton;
