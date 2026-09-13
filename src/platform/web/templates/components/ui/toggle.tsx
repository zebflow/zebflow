import { cx, useState } from "zeb/react";

/**
 * Switch — an on/off control with a sliding thumb. Controlled when `checked`
 * is given; otherwise keeps its own state. The track is `bg-input` off and
 * `bg-primary` on, so it follows the theme like every other primitive.
 */
export default function Toggle({ label, checked, onChange, disabled, className, ...rest }) {
  const [own, setOwn] = useState(Boolean(rest?.defaultChecked));
  const isControlled = typeof checked === "boolean";
  const on = isControlled ? checked : own;

  function handleChange(event) {
    if (!isControlled) setOwn(Boolean(event?.target?.checked));
    onChange?.(event);
  }

  return (
    <label
      className={cx(
        "inline-flex items-center gap-2 select-none",
        disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer",
        className,
      )}
    >
      <input
        type="checkbox"
        role="switch"
        aria-checked={on ? "true" : "false"}
        checked={on}
        onChange={handleChange}
        disabled={disabled}
        className="sr-only"
        {...rest}
      />
      <span
        aria-hidden="true"
        tw-variants="bg-primary bg-input translate-x-4 translate-x-0"
        className={cx(
          "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-transparent transition-colors",
          on ? "bg-primary" : "bg-input",
        )}
      >
        <span
          className={cx(
            "block h-4 w-4 rounded-full bg-background shadow-sm transition-transform",
            on ? "translate-x-4" : "translate-x-0",
          )}
        />
      </span>
      {label ? <span className="text-sm text-foreground">{label}</span> : null}
    </label>
  );
}
