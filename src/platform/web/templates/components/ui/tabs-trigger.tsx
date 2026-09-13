import { cx } from "zeb/react";

export default function TabsTrigger(props) {
  return (
    <button
      type="button"
      disabled={Boolean(props?.disabled)}
      className={cx(
        "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40 disabled:pointer-events-none disabled:opacity-50",
        props?.active ? "bg-popover text-foreground shadow-sm" : "text-muted-foreground",
        props?.className
      )}
      onClick={props?.onClick}
    >
      {props.children}
      <span>{props.label}</span>
    </button>
  );
}
