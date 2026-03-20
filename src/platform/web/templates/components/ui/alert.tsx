import { cx } from "zeb";

const VARIANT_CLASSES = {
  error:   "border-red-500/30 bg-red-500/10 text-red-500",
  warning: "border-yellow-500/30 bg-yellow-500/10 text-yellow-500",
  success: "border-green-500/30 bg-green-500/10 text-green-500",
  info:    "border-blue-500/30 bg-blue-500/10 text-blue-500",
};

export default function Alert({ variant = "info", className, children }) {
  const variantClass = VARIANT_CLASSES[variant] ?? VARIANT_CLASSES.info;
  return (
    <div className={cx("rounded-md border px-3 py-2 text-sm", variantClass, className)}>
      {children}
    </div>
  );
}
