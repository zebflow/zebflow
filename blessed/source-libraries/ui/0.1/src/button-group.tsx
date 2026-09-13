import { cx } from "zeb/react";

/**
 * ButtonGroup — a row (or column) of controls rendered as one connected
 * unit. Upstream squares off each child's inner corners with `[&>*]:` /
 * `has-[...]:` selectors that reach into arbitrary children; this engine's
 * Tailwind subset doesn't compile those, so the group instead walks its
 * `children` array (the same explicit-props pattern `RadioGroup` and
 * `ToggleGroup` use) and merges the squared-corner classes onto each child
 * directly by position.
 */

const ORIENTATION = {
  horizontal: "flex-row",
  vertical: "flex-col",
};

export function ButtonGroup({ className, orientation = "horizontal", children, ...props }) {
  const items = Array.isArray(children) ? children : [children];
  const visible = items.filter(Boolean);
  const last = visible.length - 1;
  const vertical = orientation === "vertical";

  const wired = visible.map((child, i) => {
    if (!child || typeof child !== "object") return child;
    const sideClasses = vertical
      ? cx(i !== 0 && "rounded-t-none border-t-0", i !== last && "rounded-b-none")
      : cx(i !== 0 && "rounded-l-none border-l-0", i !== last && "rounded-r-none");
    return { ...child, props: { ...child.props, className: cx(sideClasses, child.props?.className) } };
  });

  return (
    <div
      role="group"
      data-slot="button-group"
      data-orientation={orientation}
      className={cx("flex w-fit items-stretch", ORIENTATION[orientation] ?? ORIENTATION.horizontal, className)}
      {...props}
    >
      {wired}
    </div>
  );
}

export function ButtonGroupText({ className, ...props }) {
  return (
    <div
      data-slot="button-group-text"
      className={cx("flex items-center gap-2 rounded-md border border-border bg-muted px-4 text-sm font-medium shadow-xs", className)}
      {...props}
    />
  );
}

export function ButtonGroupSeparator({ className, orientation = "vertical", ...props }) {
  return (
    <div
      data-slot="button-group-separator"
      aria-orientation={orientation}
      className={cx("shrink-0 self-stretch bg-border", orientation === "vertical" ? "w-px" : "h-px w-full", className)}
      {...props}
    />
  );
}

export default ButtonGroup;
