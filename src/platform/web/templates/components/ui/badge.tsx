import { cx } from "zeb/react";

const VARIANT_CLASSES = {
  default:     "border-transparent bg-foreground text-popover",
  secondary:   "border-transparent bg-accent text-foreground",
  destructive: "border-transparent bg-destructive text-destructive-foreground",
  outline:     "text-foreground border-border",
};

export default function Badge(props) {
  const variant = VARIANT_CLASSES[props?.variant] ?? VARIANT_CLASSES.default;
  return (
    <div className={cx("inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring/40", variant, props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </div>
  );
}
