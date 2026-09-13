import { cx } from "zeb/react";

/**
 * Progress — shadcn/ui's progress bar, on Zebflow's engine.
 *
 * No Radix: a plain `<div>` with `role="progressbar"`. `value` is 0–100; the
 * indicator's position is an inline style (a computed value, not a class) so
 * it needs no compile-time literal.
 */

export function Progress({ className, value = 0, ...rest }) {
  const pct = Math.min(Math.max(value, 0), 100);
  return (
    <div
      data-slot="progress"
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct}
      className={cx("relative h-2 w-full overflow-hidden rounded-full bg-primary/20", className)}
      {...rest}
    >
      <div
        data-slot="progress-indicator"
        className="h-full w-full flex-1 bg-primary transition-all"
        style={{ transform: `translateX(-${100 - pct}%)` }}
      />
    </div>
  );
}

export default Progress;
