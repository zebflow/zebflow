import { cx } from "zeb/react";

/**
 * ScrollArea — shadcn/ui's scroll area, on Zebflow's engine.
 *
 * Upstream wraps Radix's viewport/scrollbar/thumb/corner primitives to draw a
 * custom scrollbar. There is no Radix here, and no `[&::-webkit-scrollbar]`
 * arbitrary variant in this engine's Tailwind subset either — so this is a
 * plain `overflow-auto` div whose native scrollbar is thinned with the
 * standard `scrollbar-width: thin` CSS property (set inline via `style`,
 * since it isn't a class the compiler needs to see). `ScrollBar` is kept as a
 * no-op export purely so a shadcn snippet that imports it still compiles —
 * it renders nothing, because there is no separate scrollbar element to
 * theme in this implementation.
 */

export function ScrollArea({ className, style, children, ...props }) {
  return (
    <div
      data-slot="scroll-area"
      className={cx("relative overflow-auto rounded-[inherit]", className)}
      style={{ scrollbarWidth: "thin", ...style }}
      {...props}
    >
      {children}
    </div>
  );
}

export function ScrollBar({ orientation = "vertical", className, ...props }) {
  return null;
}

export default ScrollArea;
