import { cx, createContext, useContext, useId, useState } from "zeb/react";

/**
 * RadioGroup — a set of mutually exclusive options. Composed through context
 * exactly as upstream Radix does: `RadioGroupItem` reads the group's current
 * value/name/disabled/onChange from `RadioGroupContext` no matter how deep it
 * is nested (the usual `<div><RadioGroupItem/><Label/></div>` layout works
 * unchanged). Controlled via `value`/`onValueChange` on the group (upstream
 * names), uncontrolled via `defaultValue`. `name` defaults to `useId()` so
 * two groups on the same page never collide through native radio grouping;
 * pass an explicit `name` only to opt into sharing one native group. Each
 * Item is a real native `<input type="radio">` (`sr-only`) inside a
 * `<label>`. As with Checkbox, the focus ring is tracked with
 * `onFocus`/`onBlur` state rather than a `peer-focus-visible:` variant — this
 * engine's Tailwind subset compiles no rule for the bare `peer` marker class.
 */

const RadioGroupContext = createContext(null);

export function RadioGroup({ value, defaultValue, onValueChange, name, disabled, className, children, ...props }) {
  const [internal, setInternal] = useState(defaultValue ?? "");
  const isControlled = value !== undefined;
  const current = isControlled ? value : internal;
  const generatedName = useId();
  const groupName = name ?? generatedName;

  function handleChange(itemValue) {
    if (!isControlled) setInternal(itemValue);
    onValueChange?.(itemValue);
  }

  const ctx = { name: groupName, value: current, disabled, onChange: handleChange };

  return (
    <div role="radiogroup" data-slot="radio-group" className={cx("grid gap-3", className)} {...props}>
      <RadioGroupContext.Provider value={ctx}>{children}</RadioGroupContext.Provider>
    </div>
  );
}

export function RadioGroupItem({ value, disabled, className, id, "aria-invalid": ariaInvalid, ...props }) {
  const ctx = useContext(RadioGroupContext);
  const [focused, setFocused] = useState(false);
  const checked = ctx?.value === value;
  const isDisabled = disabled || ctx?.disabled;

  return (
    <label className={cx("relative inline-flex size-4 shrink-0", isDisabled ? "cursor-not-allowed opacity-50" : "cursor-pointer")}>
      <input
        type="radio"
        id={id}
        name={ctx?.name}
        value={value}
        checked={checked}
        disabled={isDisabled}
        onChange={() => ctx?.onChange?.(value)}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        aria-invalid={ariaInvalid}
        className="sr-only"
        {...props}
      />
      <span
        data-slot="radio-group-item"
        data-state={checked ? "checked" : "unchecked"}
        aria-hidden="true"
        className={cx(
          "flex aspect-square size-4 items-center justify-center rounded-full border border-input shadow-xs transition-colors",
          focused ? "border-ring ring-[3px] ring-ring/50" : "",
          ariaInvalid ? "border-destructive ring-destructive/20" : "",
          "bg-input/30",
          className
        )}
      >
        {checked ? <span data-slot="radio-group-indicator" className="size-2 rounded-full bg-primary" /> : null}
      </span>
    </label>
  );
}

export default RadioGroup;
