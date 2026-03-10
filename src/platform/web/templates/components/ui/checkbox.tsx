import { cx } from "rwe";

/**
 * Inline checkbox toggle — compact mono style for dark console/toolbar contexts.
 * Forwards all extra props (including data-* attributes) to the underlying input.
 */
export default function Checkbox({ label, className, ...rest }) {
  return (
    <label className={cx("zf-checkbox", className)}>
      <input type="checkbox" className="zf-checkbox-input" {...rest} />
      <span className="zf-checkbox-label">{label}</span>
    </label>
  );
}
