import { cx } from "zeb/react";

const VARIANT_CLASSES = {
  error:   "border-destructive/30 bg-destructive/10 text-destructive",
  warning: "border-warning/30 bg-warning/10 text-warning",
  success: "border-success/30 bg-success/10 text-success",
  info:    "border-info/30 bg-info/10 text-info",
};

export default function Alert({ variant = "info", className, children }) {
  const variantClass = VARIANT_CLASSES[variant] ?? VARIANT_CLASSES.info;
  return (
    <div className={cx("rounded-md border px-3 py-2 text-sm", variantClass, className)}>
      {children}
    </div>
  );
}
