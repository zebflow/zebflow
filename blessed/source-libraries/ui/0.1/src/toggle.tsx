import { cx, useState } from "zeb/react";

/**
 * Toggle — a two-state button (bold/italic-style). Radix's `Toggle.Root` is
 * a real `<button aria-pressed>`, which is exactly what a plain button
 * gives natively. Controlled via `pressed`/`onPressedChange` (upstream
 * names), uncontrolled via `defaultPressed`.
 */

const VARIANTS = {
  default: "bg-transparent",
  outline: "border border-input bg-transparent shadow-xs hover:bg-accent hover:text-accent-foreground",
};

const SIZES = {
  default: "h-9 min-w-9 px-2",
  sm: "h-8 min-w-8 px-1.5",
  lg: "h-10 min-w-10 px-2.5",
};

const BASE =
  "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap outline-none transition-colors hover:bg-muted hover:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 data-[state=on]:bg-accent data-[state=on]:text-accent-foreground";

export function toggleVariants({ variant = "default", size = "default", className = "" } = {}) {
  return cx(BASE, VARIANTS[variant] ?? VARIANTS.default, SIZES[size] ?? SIZES.default, className);
}

export function Toggle({ className, variant = "default", size = "default", pressed, defaultPressed, onPressedChange, disabled, children, ...props }) {
  const [internal, setInternal] = useState(Boolean(defaultPressed));
  const isControlled = pressed !== undefined;
  const isPressed = isControlled ? Boolean(pressed) : internal;

  function handleClick() {
    if (disabled) return;
    const next = !isPressed;
    if (!isControlled) setInternal(next);
    onPressedChange?.(next);
  }

  return (
    <button
      type="button"
      data-slot="toggle"
      data-state={isPressed ? "on" : "off"}
      aria-pressed={isPressed}
      disabled={disabled}
      onClick={handleClick}
      className={toggleVariants({ variant, size, className })}
      {...props}
    >
      {children}
    </button>
  );
}

export default Toggle;
