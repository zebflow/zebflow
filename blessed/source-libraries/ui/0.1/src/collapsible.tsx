import { cx, useState } from "zeb/react";

/**
 * Collapsible — shadcn/ui's collapsible, on Zebflow's engine.
 *
 * Same native `<details>`/`<summary>` approach as `zeb/ui/accordion`, but for
 * a single standalone item: `open` is a real DOM attribute here, so the
 * browser renders the correct state before hydration. `open`/`onOpenChange`
 * make it controlled; `defaultOpen` makes it uncontrolled — same pattern as
 * upstream, just without Radix underneath. No context is needed here (unlike
 * `zeb/ui/accordion`, there's only ever one item): `CollapsibleTrigger`/
 * `CollapsibleContent` are plain `<summary>`/`<div>` — `<details>` already
 * hides its content when closed, so they don't need the open flag at all.
 */

export function Collapsible({ open, defaultOpen = false, onOpenChange, className, children, ...props }) {
  const [internal, setInternal] = useState(defaultOpen);
  const controlled = open !== undefined;
  const isOpen = controlled ? open : internal;
  const onToggle = (e) => {
    const next = e.target.open;
    if (!controlled) setInternal(next);
    onOpenChange?.(next);
  };
  return (
    <details data-slot="collapsible" open={isOpen} onToggle={onToggle} className={cx("group", className)} {...props}>
      {children}
    </details>
  );
}

export function CollapsibleTrigger({ className, children, ...props }) {
  return (
    <summary data-slot="collapsible-trigger" style={{ listStyle: "none" }} className={cx("cursor-pointer", className)} {...props}>
      {children}
    </summary>
  );
}

export function CollapsibleContent({ className, children, ...props }) {
  return (
    <div data-slot="collapsible-content" className={cx("", className)} {...props}>
      {children}
    </div>
  );
}

export default Collapsible;
