import { cx, useState } from "zeb/react";

/**
 * Switch — an on/off control. Radix's `Switch.Root` renders as a real
 * `<button>`, which is exactly what a plain `<button role="switch"
 * aria-checked>` gives natively — no hook needed beyond tracking the state.
 * Controlled via `checked`/`onCheckedChange` (upstream names), uncontrolled
 * via `defaultChecked`. `size` keeps upstream's `"sm" | "default"`. A
 * caller's own `onClick` is composed after the toggle handler (not
 * overwritten by it) — props are spread before the toggle's own `onClick` so
 * the composed handler always wins the DOM binding while everything else a
 * caller passes (data attributes, aria overrides, …) still comes through.
 */

const TRACK_SIZE = {
  default: "h-[1.15rem] w-8",
  sm: "h-3.5 w-6",
};

const THUMB_SIZE = {
  default: "size-4",
  sm: "size-3",
};

export function Switch({ className, checked, defaultChecked, onCheckedChange, disabled, size = "default", id, onClick, ...props }) {
  const [internal, setInternal] = useState(Boolean(defaultChecked));
  const isControlled = checked !== undefined;
  const isChecked = isControlled ? Boolean(checked) : internal;

  function handleClick(e) {
    if (disabled) return;
    const next = !isChecked;
    if (!isControlled) setInternal(next);
    onCheckedChange?.(next);
    onClick?.(e);
  }

  return (
    <button
      type="button"
      role="switch"
      id={id}
      aria-checked={isChecked}
      data-slot="switch"
      data-state={isChecked ? "checked" : "unchecked"}
      data-size={size}
      disabled={disabled}
      className={cx(
        "inline-flex shrink-0 items-center rounded-full border border-transparent shadow-xs outline-none transition-all focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20",
        TRACK_SIZE[size] ?? TRACK_SIZE.default,
        isChecked ? "bg-primary" : "bg-input",
        className
      )}
      {...props}
      onClick={handleClick}
    >
      <span
        data-slot="switch-thumb"
        data-state={isChecked ? "checked" : "unchecked"}
        className={cx(
          "pointer-events-none block rounded-full bg-background shadow-sm transition-transform",
          THUMB_SIZE[size] ?? THUMB_SIZE.default,
          isChecked ? "translate-x-[calc(100%-2px)]" : "translate-x-0"
        )}
      />
    </button>
  );
}

export default Switch;
