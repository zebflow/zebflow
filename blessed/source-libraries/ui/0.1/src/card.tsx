import { cx } from "zeb/react";

/**
 * Card — shadcn/ui's card, on Zebflow's engine.
 *
 * Two upstream tricks are dropped because they need selectors this engine
 * doesn't compile: `CardHeader`'s `@container/card-header` +
 * `has-data-[slot=card-action]:grid-cols-[1fr_auto]` (a container query and a
 * `has-data-` variant) — pass `className="grid-cols-[1fr_auto]"` yourself
 * when the header holds a `CardAction`. Likewise `[.border-b]:pb-6` on
 * `CardHeader` and `[.border-t]:pt-6` on `CardFooter` (arbitrary compound
 * selectors) — add `pb-6` / `pt-6` yourself alongside `border-b` / `border-t`.
 */

export function Card({ className, children, ...rest }) {
  return (
    <div
      data-slot="card"
      className={cx("flex flex-col gap-6 rounded-xl border bg-card py-6 text-card-foreground shadow-sm", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export function CardHeader({ className, children, ...rest }) {
  return (
    <div
      data-slot="card-header"
      className={cx("grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export function CardTitle({ className, children, ...rest }) {
  return (
    <div data-slot="card-title" className={cx("leading-none font-semibold", className)} {...rest}>
      {children}
    </div>
  );
}

export function CardDescription({ className, children, ...rest }) {
  return (
    <div data-slot="card-description" className={cx("text-sm text-muted-foreground", className)} {...rest}>
      {children}
    </div>
  );
}

export function CardAction({ className, children, ...rest }) {
  return (
    <div
      data-slot="card-action"
      className={cx("col-start-2 row-span-2 row-start-1 self-start justify-self-end", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export function CardContent({ className, children, ...rest }) {
  return (
    <div data-slot="card-content" className={cx("px-6", className)} {...rest}>
      {children}
    </div>
  );
}

export function CardFooter({ className, children, ...rest }) {
  return (
    <div data-slot="card-footer" className={cx("flex items-center px-6", className)} {...rest}>
      {children}
    </div>
  );
}

export default Card;
