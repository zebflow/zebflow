import { cx } from "zeb/react";

/**
 * Separator — shadcn/ui's separator, on Zebflow's engine.
 *
 * No Radix primitive: a plain `<div>` sets `data-orientation` itself and the
 * `data-[orientation=…]` classes key off that same element, which the rules
 * allow. `decorative` (the default) reports `role="none"`; set it false for
 * `role="separator"` with `aria-orientation`.
 */

export function Separator({ className, orientation = "horizontal", decorative = true, ...rest }) {
  return (
    <div
      data-slot="separator"
      data-orientation={orientation}
      role={decorative ? "none" : "separator"}
      aria-orientation={decorative ? undefined : orientation}
      className={cx(
        "shrink-0 bg-border data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-px",
        className
      )}
      {...rest}
    />
  );
}

export default Separator;
