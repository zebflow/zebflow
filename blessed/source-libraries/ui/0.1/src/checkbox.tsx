import { cx, useEffect, useRef, useState } from "zeb/react";

/**
 * Checkbox — shadcn's checkbox, on Zebflow's engine. Radix renders a
 * button-like control with a hidden native input underneath for form
 * submission; here the real native `<input type="checkbox">` IS the
 * control — visually hidden with `sr-only` inside a `<label>` — and a
 * sibling `<span>` draws the box from the same state, so keyboard, forms
 * and autofill all work natively. Controlled via `checked`/`onCheckedChange`
 * (upstream names), uncontrolled via `defaultChecked`; both accept upstream's
 * `checked="indeterminate"`. Indeterminate is not a DOM attribute — it can
 * only be set imperatively on the input element — so it is applied in an
 * effect via a ref, and the visible box gets a dash glyph with
 * `aria-checked="mixed"` rather than being coerced to a plain checked state.
 * The engine's Tailwind subset compiles no rule for the bare `peer` marker
 * class, so the focus ring can't be a `peer-focus-visible:` variant on the
 * sibling box — it is tracked with `onFocus`/`onBlur` state instead.
 */

export function Checkbox({ className, checked, defaultChecked, onCheckedChange, disabled, id, "aria-invalid": ariaInvalid, ...props }) {
  const [internal, setInternal] = useState(defaultChecked ?? false);
  const [focused, setFocused] = useState(false);
  const isControlled = checked !== undefined;
  const current = isControlled ? checked : internal;
  const isIndeterminate = current === "indeterminate";
  const isChecked = current === true;
  const inputRef = useRef(null);

  useEffect(() => {
    if (inputRef.current) inputRef.current.indeterminate = isIndeterminate;
  }, [isIndeterminate]);

  function handleChange(e) {
    const next = e.target.checked;
    if (!isControlled) setInternal(next);
    onCheckedChange?.(next);
  }

  const state = isIndeterminate ? "indeterminate" : isChecked ? "checked" : "unchecked";

  return (
    <label className={cx("relative inline-flex size-4 shrink-0", disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer")}>
      <input
        ref={inputRef}
        type="checkbox"
        id={id}
        checked={isChecked}
        onChange={handleChange}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        disabled={disabled}
        aria-checked={isIndeterminate ? "mixed" : isChecked}
        aria-invalid={ariaInvalid}
        className="sr-only"
        {...props}
      />
      <span
        data-slot="checkbox"
        data-state={state}
        aria-hidden="true"
        className={cx(
          "flex aspect-square size-4 items-center justify-center rounded-sm border border-input shadow-xs transition-shadow",
          focused ? "border-ring ring-[3px] ring-ring/50" : "",
          ariaInvalid ? "border-destructive ring-destructive/20" : "",
          isChecked || isIndeterminate ? "border-primary bg-primary text-primary-foreground" : "bg-input/30",
          className
        )}
      >
        {isIndeterminate ? (
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="size-3.5">
            <path d="M5 12h14" />
          </svg>
        ) : isChecked ? (
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="size-3.5">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        ) : null}
      </span>
    </label>
  );
}

export default Checkbox;
