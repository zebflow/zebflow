import { cx } from "zeb/react";
import { Button } from "zeb/ui/button";
import { Input } from "zeb/ui/input";
import { Textarea } from "zeb/ui/textarea";

/**
 * InputGroup — a bordered row combining an Input/Textarea with icon/button/
 * text addons. Upstream detects a nested textarea or an invalid control
 * with `:has(>textarea)` / `:has([aria-invalid=true])`; this engine's
 * Tailwind subset doesn't reach into children that way, so this port takes
 * that state as explicit props instead: `multiline` (the height/direction
 * switch upstream gets from `:has(>textarea)`) and `invalid` (the border/
 * ring upstream gets from `:has([aria-invalid=true])`). `InputGroupButton`
 * reuses `Button`'s own size scale (`xs`/`sm`/`icon-xs`/`icon-sm`) instead
 * of redeclaring one.
 */

const ALIGN = {
  "inline-start": "order-first pl-3",
  "inline-end": "order-last pr-3",
  "block-start": "order-first w-full justify-start px-3 pt-3",
  "block-end": "order-last w-full justify-start px-3 pb-3",
};

export function InputGroup({ className, multiline, invalid, ...props }) {
  return (
    <div
      data-slot="input-group"
      role="group"
      className={cx(
        "relative flex w-full items-center rounded-md border border-input shadow-xs transition-colors outline-none dark:bg-input/30",
        multiline ? "h-auto flex-col" : "h-9 min-w-0",
        invalid ? "border-destructive ring-destructive/20 dark:ring-destructive/40" : "",
        className
      )}
      {...props}
    />
  );
}

export function InputGroupAddon({ className, align = "inline-start", ...props }) {
  return (
    <div
      role="group"
      data-slot="input-group-addon"
      data-align={align}
      className={cx(
        "flex h-auto cursor-text items-center justify-center gap-2 py-1.5 text-sm font-medium text-muted-foreground select-none",
        ALIGN[align] ?? ALIGN["inline-start"],
        className
      )}
      onClick={(e) => {
        if (e.target?.closest?.("button")) return;
        e.currentTarget.parentElement?.querySelector("input,textarea")?.focus();
      }}
      {...props}
    />
  );
}

export function InputGroupButton({ className, type = "button", variant = "ghost", size = "xs", ...props }) {
  return <Button type={type} variant={variant} size={size} className={cx("shadow-none", className)} {...props} />;
}

export function InputGroupText({ className, ...props }) {
  return <span className={cx("flex items-center gap-2 text-sm text-muted-foreground", className)} {...props} />;
}

export function InputGroupInput({ className, ...props }) {
  return (
    <Input
      data-slot="input-group-control"
      className={cx("flex-1 rounded-none border-0 bg-transparent shadow-none focus-visible:ring-0", className)}
      {...props}
    />
  );
}

export function InputGroupTextarea({ className, ...props }) {
  return (
    <Textarea
      data-slot="input-group-control"
      className={cx("flex-1 resize-none rounded-none border-0 bg-transparent py-3 shadow-none focus-visible:ring-0", className)}
      {...props}
    />
  );
}

export default InputGroup;
