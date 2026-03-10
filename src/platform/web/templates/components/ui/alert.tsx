import { cx } from "rwe";

const VARIANT_CLASSES = {
  error:   "border-red-200 bg-red-50 text-red-700 dark:border-red-800 dark:bg-red-950/30 dark:text-red-400",
  warning: "border-yellow-200 bg-yellow-50 text-yellow-700 dark:border-yellow-800 dark:bg-yellow-950/30 dark:text-yellow-400",
  success: "border-green-200 bg-green-50 text-green-700 dark:border-green-800 dark:bg-green-950/30 dark:text-green-400",
  info:    "border-blue-200 bg-blue-50 text-blue-700 dark:border-blue-800 dark:bg-blue-950/30 dark:text-blue-400",
};

export default function Alert({ variant = "info", className, children }) {
  const variantClass = VARIANT_CLASSES[variant] ?? VARIANT_CLASSES.info;
  return (
    <div className={cx("rounded-md border px-3 py-2 text-sm", variantClass, className)}>
      {children}
    </div>
  );
}
