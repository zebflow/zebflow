import { cx } from "zeb/react";

/**
 * Spinner — shadcn/ui's loading spinner, on Zebflow's engine.
 *
 * Upstream renders lucide's `Loader2Icon`; there is no lucide here, so this
 * is the same glyph (a circle with one open arc) as an inline `<svg>`.
 */

export function Spinner({ className, ...rest }) {
  return (
    <svg
      data-slot="spinner"
      role="status"
      aria-label="Loading"
      viewBox="0 0 24 24"
      fill="none"
      className={cx("size-4 animate-spin", className)}
      {...rest}
    >
      <path
        d="M21 12a9 9 0 1 1-6.219-8.56"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default Spinner;
