import { cx } from "zeb";

const ALIGN_CLASSES = {
  left: "left-0",
  right: "right-0",
  center: "left-1/2 -translate-x-1/2",
};

export default function DropdownMenuContent({ align, className, children, ...rest }) {
  const alignClass = ALIGN_CLASSES[align] ?? ALIGN_CLASSES.left;
  return (
    <div className={cx(
      "absolute z-50 mt-2 min-w-[8rem] overflow-hidden rounded-md p-1 shadow-md animate-in fade-in-80 zoom-in-95",
      "border border-[var(--studio-border,#2b3648)]",
      "bg-[var(--studio-panel,#111827)]",
      "text-[var(--studio-text,#e5edf7)]",
      alignClass, className)} {...rest}>
      {children}
    </div>
  );
}
