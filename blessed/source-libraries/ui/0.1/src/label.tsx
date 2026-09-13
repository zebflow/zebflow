import { cx } from "zeb/react";

/**
 * Label — shadcn's form label, on Zebflow's engine: a plain `<label>`
 * instead of Radix's `Label.Root`, which only adds double-click-to-select
 * prevention. Pair it with a control via `htmlFor`/`id`, same as upstream.
 * `peer-disabled:` is kept for a control styled with a literal `peer` class
 * of its own, though none in this library adds one — the engine's Tailwind
 * subset has no rule for the bare `peer` marker itself (only `peer-<state>:`
 * variants compile). Upstream's `group-data-[disabled=true]:` (styling read
 * off a wrapping Field's data attribute) is dropped — `group-data-[...]`
 * isn't in the engine's variant set; drive a disabled look from a
 * `className` prop the parent computes instead.
 */

export function Label({ className, ...props }) {
  return (
    <label
      data-slot="label"
      className={cx(
        "flex items-center gap-2 text-sm leading-none font-medium select-none peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
        className
      )}
      {...props}
    />
  );
}

export default Label;
