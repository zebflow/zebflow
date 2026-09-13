import { cx } from "zeb/react";

/**
 * Badge — shadcn/ui's badge, on Zebflow's engine.
 *
 * No `asChild`: pass `as="a"` and an `href` instead (see button.tsx). Upstream
 * tints an anchor child's hover state with the `[a&]:hover:…` arbitrary
 * variant; that bracket selector isn't compiled, so the hover class here
 * applies unconditionally regardless of tag — the visual result is the same
 * whether the badge renders as a `<span>` or an `<a>`.
 */

const VARIANTS = {
  default: "bg-primary text-primary-foreground hover:bg-primary/90",
  secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/90",
  destructive:
    "bg-destructive text-destructive-foreground hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
  outline: "border-border text-foreground hover:bg-accent hover:text-accent-foreground",
  ghost: "hover:bg-accent hover:text-accent-foreground",
  link: "text-primary underline-offset-4 hover:underline",
};

const BASE =
  "inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap transition-colors focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40";

export function badgeVariants({ variant = "default", className = "" } = {}) {
  return cx(BASE, VARIANTS[variant] ?? VARIANTS.default, className);
}

export function Badge({ className, variant = "default", as: Tag = "span", children, ...rest }) {
  const Comp = Tag;
  return (
    <Comp data-slot="badge" data-variant={variant} className={badgeVariants({ variant, className })} {...rest}>
      {children}
    </Comp>
  );
}

export default Badge;
