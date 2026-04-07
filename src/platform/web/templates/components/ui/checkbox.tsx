import { cx } from "zeb";

/**
 * Inline checkbox toggle — compact mono style for dark console/toolbar contexts.
 * Forwards all extra props (including data-* attributes) to the underlying input.
 */
export default function Checkbox({ label, className, ...rest }) {
  return (
    <label className={cx("inline-flex items-center gap-1.5 cursor-pointer select-none", className)}>
      <input type="checkbox" className="size-3.5 accent-[var(--color-accent,#60a5fa)] cursor-pointer" {...rest} />
      <span className="text-[0.7rem] font-mono text-slate-400">{label}</span>
    </label>
  );
}
