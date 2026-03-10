import { cx } from "rwe";

const VARIANT_CLASSES = {
  primary: "bg-slate-900 text-white hover:opacity-90",
  outline: "border border-[var(--studio-border)] bg-transparent hover:bg-[var(--studio-panel-3)] text-[var(--studio-text)]",
  secondary: "bg-[var(--studio-panel-3)] text-[var(--studio-text)] hover:opacity-80",
  ghost: "hover:bg-[var(--studio-panel-3)] text-[var(--studio-text-soft)] hover:text-[var(--studio-text)]",
  destructive: "bg-red-500/10 text-red-500 border border-red-500/20 hover:bg-red-500/20",
  link: "text-[var(--zf-color-brand-blue)] underline-offset-4 hover:underline",
};

const SIZE_CLASSES = {
  md: "h-8 px-3",
  sm: "h-6 px-2 text-xs",
  xs: "h-7 px-2.5 text-[0.8rem]",
  lg: "h-9 px-4",
  icon: "size-8",
};

export default function Button({
  type = "button",
  variant = "primary",
  size = "md",
  className,
  children,
  label,
  ...rest
}) {
  const variantClass = VARIANT_CLASSES[variant] ?? VARIANT_CLASSES.primary;
  const sizeClass = SIZE_CLASSES[size] ?? SIZE_CLASSES.md;
  const content = children ?? label;

  return (
    <button
      type={type}
      {...rest}
      className={cx(
        "inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium transition-all outline-none select-none disabled:pointer-events-none disabled:opacity-50",
        variantClass,
        sizeClass,
        className
      )}
    >
      <span>{content}</span>
    </button>
  );
}
