import { cx } from "rwe";

/**
 * iOS-style toggle switch. Drop-in replacement for Checkbox when a
 * binary on/off control is needed without the checkbox visual.
 *
 * Usage:
 *   <Toggle checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
 *   <Toggle checked={value} onChange={handler} label="Enable" disabled={loading} />
 */
export default function Toggle({ label, checked, onChange, disabled, className, ...rest }) {
  return (
    <label className={cx("zf-toggle", disabled && "is-disabled", className)}>
      <input
        type="checkbox"
        checked={checked}
        onChange={onChange}
        disabled={disabled}
        className="sr-only"
        {...rest}
      />
      <span className="zf-toggle-track" aria-hidden="true">
        <span className="zf-toggle-thumb" />
      </span>
      {label && <span className="zf-toggle-label">{label}</span>}
    </label>
  );
}
