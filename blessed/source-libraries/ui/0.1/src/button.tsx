import { cx } from "zeb/react";

/**
 * Button — shadcn/ui's button, on Zebflow's engine.
 *
 * Same variant and size names as upstream so a snippet written for shadcn
 * reads the same here. Two deliberate differences: there is no `asChild`
 * (no Slot in zeb/react) — pass `as="a"` and an `href` instead — and variants
 * are a plain object joined by `cx`, not `cva`, because the class strings must
 * be literal for the compile-time Tailwind scan.
 */

const VARIANTS = {
  default: "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
  destructive:
    "bg-destructive text-destructive-foreground shadow-xs hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
  outline:
    "border border-input bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:hover:bg-input/50",
  secondary: "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80",
  ghost: "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
  link: "text-primary underline-offset-4 hover:underline",
};

const SIZES = {
  default: "h-9 px-4 py-2",
  xs: "h-6 gap-1 rounded-md px-2 text-xs",
  sm: "h-8 gap-1.5 rounded-md px-3",
  lg: "h-10 rounded-md px-6",
  icon: "size-9",
  "icon-xs": "size-6 rounded-md",
  "icon-sm": "size-8",
  "icon-lg": "size-10",
};

const BASE =
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40";

export function buttonVariants({ variant = "default", size = "default", className = "" } = {}) {
  return cx(BASE, VARIANTS[variant] ?? VARIANTS.default, SIZES[size] ?? SIZES.default, className);
}

export function Button({ className, variant = "default", size = "default", as: Tag = "button", type, children, ...props }) {
  const Comp = Tag;
  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      type={Comp === "button" ? (type ?? "button") : type}
      className={buttonVariants({ variant, size, className })}
      {...props}
    >
      {children}
    </Comp>
  );
}

export default Button;
