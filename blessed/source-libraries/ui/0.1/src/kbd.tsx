import { cx } from "zeb/react";

/**
 * Kbd — shadcn/ui's keyboard-key label, on Zebflow's engine.
 *
 * Upstream auto-sizes an unsized child `<svg>` (`[&_svg:not(…)]:size-3`) and
 * re-tints itself inside a tooltip (`[[data-slot=tooltip-content]_&]:…`) —
 * both are arbitrary descendant/compound selectors this engine doesn't
 * compile. Size an icon child yourself (`className="size-3"`); a `Kbd`
 * nested in `zeb/ui/tooltip` keeps its normal colours.
 */

export function Kbd({ className, children, ...rest }) {
  return (
    <kbd
      data-slot="kbd"
      className={cx(
        "pointer-events-none inline-flex h-5 w-fit min-w-5 items-center justify-center gap-1 rounded-sm bg-muted px-1 font-sans text-xs font-medium text-muted-foreground select-none",
        className
      )}
      {...rest}
    >
      {children}
    </kbd>
  );
}

export function KbdGroup({ className, children, ...rest }) {
  return (
    <kbd data-slot="kbd-group" className={cx("inline-flex items-center gap-1", className)} {...rest}>
      {children}
    </kbd>
  );
}

export default Kbd;
