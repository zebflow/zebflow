import { cx } from "rwe";

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
        "flex w-full rounded-md border border-slate-200 bg-white px-3 py-2 text-sm shadow-sm placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-[var(--zf-color-brand-blue)]/10 focus-visible:border-[var(--zf-color-brand-blue)]/40 resize-y disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-100",
        className
      )}
    />
  );
}
