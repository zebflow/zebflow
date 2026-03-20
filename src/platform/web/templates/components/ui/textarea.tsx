import { cx } from "zeb";

/**
 * Textarea — styled multi-line text input that matches the Input component's
 * visual language (same border, focus ring, colour tokens).
 */
export default function Textarea({
  name,
  id,
  value,
  defaultValue,
  placeholder,
  rows,
  disabled,
  readOnly,
  onInput,
  onChange,
  className,
  ...rest
}) {
  const hasValue = value !== undefined;
  return (
    <textarea
      {...rest}
      name={name}
      id={id}
      placeholder={placeholder}
      rows={rows}
      disabled={disabled}
      readOnly={readOnly}
      onInput={onInput}
      onChange={onChange}
      value={hasValue ? value : undefined}
      defaultValue={hasValue ? undefined : defaultValue}
      className={cx(
        "flex w-full rounded-md border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg)] text-[var(--zf-ui-text)] px-3 py-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--zf-color-brand-blue)]/40 focus-visible:border-[var(--zf-color-brand-blue)]/40 resize-y disabled:cursor-not-allowed disabled:opacity-50",
        className
      )}
    />
  );
}
