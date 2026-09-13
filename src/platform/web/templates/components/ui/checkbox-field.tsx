import { cx } from "zeb/react";

/**
 * A checkbox with a label, and room for the sentence that explains it.
 *
 * Distinct from `Checkbox`, which is the compact mono control for toolbars and
 * consoles. This is the form shape: a choice with a consequence worth stating,
 * where the description is often the only place the reader learns what the box
 * actually does.
 *
 * `description` is rendered under the label, and dims with the control when the
 * option is unavailable — an explanation of something you cannot pick should
 * not shout louder than the thing itself.
 */
export default function CheckboxField({
  label,
  description,
  checked,
  disabled,
  onChange,
  className,
}) {
  return (
    <label
      className={cx(
        "flex items-start gap-2 text-sm",
        disabled ? "text-muted-foreground" : "text-foreground cursor-pointer",
        className,
      )}
    >
      <input
        type="checkbox"
        className="mt-1 size-3.5 accent-primary disabled:cursor-not-allowed"
        checked={!!checked}
        disabled={!!disabled}
        onChange={onChange}
      />
      <span className="min-w-0">
        {label}
        {description ? (
          <span className="block text-xs text-muted-foreground">{description}</span>
        ) : null}
      </span>
    </label>
  );
}
