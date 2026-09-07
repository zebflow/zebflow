import { cx } from "zeb/react";

const VARIANT_CLASSES = {
  primary: "border border-brand-orange bg-brand-orange text-white hover:opacity-90",
  outline: "border border-[var(--color-border,var(--color-ui-border))] bg-transparent text-[var(--color-body,var(--color-ui-text))] hover:bg-[var(--color-surface-2,var(--color-ui-bg-subtle))]",
  secondary: "border border-[var(--color-border,var(--color-ui-border))] bg-[var(--color-surface-2,var(--color-ui-bg-subtle))] text-[var(--color-body,var(--color-ui-text))] hover:bg-[var(--color-surface-3,var(--color-ui-bg-muted))]",
  ghost: "text-[var(--color-body-soft,var(--color-ui-text-soft))] hover:bg-[var(--color-surface-2,var(--color-ui-bg-subtle))] hover:text-[var(--color-body,var(--color-ui-text))]",
  destructive: "bg-red-500/10 !text-red-500 border border-red-500/20 hover:bg-red-500/20",
  link: "!text-brand-blue underline-offset-4 hover:underline",
  live: "bg-green-600 !text-white hover:bg-green-700 border border-green-600",
};
const ALL_VARIANT_TOKENS = Object.values(VARIANT_CLASSES).join(" ");

const SIZE_CLASSES = {
  md: "h-9 px-4",
  sm: "h-8 px-3 text-xs",
  xs: "h-7 px-2.5 text-[0.8rem]",
  lg: "h-10 px-6",
  icon: "h-9 w-9",
};

export default function Button({
  type = "button",
  as: Tag,
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
  const Element = Tag || "button";

  return (
    <>
      <span
        hidden
        tw-variants="border border-brand-orange bg-brand-orange text-white hover:opacity-90 border-[var(--color-border,var(--color-ui-border))] bg-transparent text-[var(--color-body,var(--color-ui-text))] hover:bg-[var(--color-surface-2,var(--color-ui-bg-subtle))] bg-[var(--color-surface-2,var(--color-ui-bg-subtle))] hover:bg-[var(--color-surface-3,var(--color-ui-bg-muted))] text-[var(--color-body-soft,var(--color-ui-text-soft))] hover:text-[var(--color-body,var(--color-ui-text))] bg-red-500/10 !text-red-500 border border-red-500/20 hover:bg-red-500/20 !text-brand-blue underline-offset-4 hover:underline bg-green-600 !text-white hover:bg-green-700 border border-green-600"
      />
      <Element
        type={Element === "button" ? type : undefined}
        tw-variants={ALL_VARIANT_TOKENS}
        {...rest}
        className={cx(
          "inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium transition-all outline-none select-none disabled:pointer-events-none disabled:opacity-50",
          variantClass,
          sizeClass,
          className
        )}
      >
        <span className="inline-flex items-center gap-2">{content}</span>
      </Element>
    </>
  );
}
