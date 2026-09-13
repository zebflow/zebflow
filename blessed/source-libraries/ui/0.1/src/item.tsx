import { cx } from "zeb/react";
import { Separator } from "zeb/ui/separator";

/**
 * Item — shadcn/ui's item, on Zebflow's engine.
 *
 * No `asChild`: pass `as="a"` and an `href` instead (see button.tsx).
 * Upstream tints a child `<a>` on hover with the `[a]:hover:bg-accent/50`
 * bracket variant; dropped — pass `className="hover:bg-accent/50"` to that
 * child anchor yourself. `ItemMedia`'s icon/image auto-sizing
 * (`[&_svg…]`, `[&_img]:…`) and its `group-has-[…]/item:…` shift when a
 * description is present are dropped the same way — arbitrary/named-group
 * selectors this engine doesn't compile.
 */

const ITEM_VARIANTS = {
  default: "bg-transparent",
  outline: "border-border",
  muted: "bg-muted/50",
};

const ITEM_SIZES = {
  default: "gap-4 p-4",
  sm: "gap-2.5 px-4 py-3",
};

export function Item({ className, variant = "default", size = "default", as: Tag = "div", children, ...rest }) {
  const Comp = Tag;
  return (
    <Comp
      data-slot="item"
      data-variant={variant}
      data-size={size}
      className={cx(
        "flex flex-wrap items-center rounded-md border border-transparent text-sm transition-colors duration-100 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
        ITEM_VARIANTS[variant] ?? ITEM_VARIANTS.default,
        ITEM_SIZES[size] ?? ITEM_SIZES.default,
        className
      )}
      {...rest}
    >
      {children}
    </Comp>
  );
}

const ITEM_MEDIA_VARIANTS = {
  default: "bg-transparent",
  icon: "size-8 rounded-sm border bg-muted",
  image: "size-10 overflow-hidden rounded-sm",
};

export function ItemMedia({ className, variant = "default", children, ...rest }) {
  return (
    <div
      data-slot="item-media"
      data-variant={variant}
      className={cx("flex shrink-0 items-center justify-center gap-2", ITEM_MEDIA_VARIANTS[variant] ?? ITEM_MEDIA_VARIANTS.default, className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export function ItemContent({ className, children, ...rest }) {
  return (
    <div data-slot="item-content" className={cx("flex flex-1 flex-col gap-1", className)} {...rest}>
      {children}
    </div>
  );
}

export function ItemTitle({ className, children, ...rest }) {
  return (
    <div data-slot="item-title" className={cx("flex w-fit items-center gap-2 text-sm leading-snug font-medium", className)} {...rest}>
      {children}
    </div>
  );
}

export function ItemDescription({ className, children, ...rest }) {
  return (
    <p data-slot="item-description" className={cx("line-clamp-2 text-sm leading-normal font-normal text-balance text-muted-foreground", className)} {...rest}>
      {children}
    </p>
  );
}

export function ItemActions({ className, children, ...rest }) {
  return (
    <div data-slot="item-actions" className={cx("flex items-center gap-2", className)} {...rest}>
      {children}
    </div>
  );
}

export function ItemGroup({ className, children, ...rest }) {
  return (
    <div role="list" data-slot="item-group" className={cx("flex flex-col", className)} {...rest}>
      {children}
    </div>
  );
}

export function ItemSeparator({ className, ...rest }) {
  return <Separator data-slot="item-separator" orientation="horizontal" className={cx("my-0", className)} {...rest} />;
}

export function ItemHeader({ className, children, ...rest }) {
  return (
    <div data-slot="item-header" className={cx("flex basis-full items-center justify-between gap-2", className)} {...rest}>
      {children}
    </div>
  );
}

export function ItemFooter({ className, children, ...rest }) {
  return (
    <div data-slot="item-footer" className={cx("flex basis-full items-center justify-between gap-2", className)} {...rest}>
      {children}
    </div>
  );
}

export default Item;
