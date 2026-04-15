import { cx } from "zeb";

const VARIANT_CLASSES = {
  primary: "border border-dark-accent1 bg-dark-accent1 !text-black hover:opacity-90",
  outline: "border border-dark-border bg-transparent hover:bg-dark-border !text-body",
  secondary: "border border-dark-border bg-dark-border !text-body hover:bg-dark-background",
  ghost: "!text-body-soft hover:bg-dark-border hover:!text-body",
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
        tw-variants="border border-dark-accent1 bg-dark-accent1 !text-black hover:opacity-90 border-dark-border bg-transparent hover:bg-dark-border !text-body bg-dark-border hover:bg-dark-background !text-body-soft hover:!text-body bg-red-500/10 !text-red-500 border border-red-500/20 hover:bg-red-500/20 !text-brand-blue underline-offset-4 hover:underline bg-green-600 !text-white hover:bg-green-700 border border-green-600"
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
