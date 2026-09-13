import { cx } from "zeb/react";

const VARIANT_CLASSES = {
  primary: "border border-primary bg-primary text-primary-foreground hover:bg-primary/90",
  outline: "border border-input bg-transparent text-foreground hover:bg-muted",
  secondary: "border border-input bg-secondary text-secondary-foreground hover:bg-accent",
  ghost: "text-muted-foreground hover:bg-muted hover:text-foreground",
  destructive: "border border-destructive/20 bg-destructive/10 !text-destructive hover:bg-destructive/20",
  link: "!text-info underline-offset-4 hover:underline",
  live: "border border-success bg-success !text-success-foreground hover:bg-success/90",
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
        tw-variants="border border-primary bg-primary text-primary-foreground hover:bg-primary/90 border border-input bg-transparent text-foreground hover:bg-muted border border-input bg-secondary text-secondary-foreground hover:bg-accent text-muted-foreground hover:bg-muted hover:text-foreground border border-destructive/20 bg-destructive/10 !text-destructive hover:bg-destructive/20 !text-info underline-offset-4 hover:underline border border-success bg-success !text-success-foreground hover:bg-success/90"
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
