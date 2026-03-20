import { cx } from "zeb";

export default function Input(props) {
  const {
    type,
    name,
    id,
    value,
    defaultValue,
    required,
    disabled,
    readOnly,
    autoComplete,
    placeholder,
    onInput,
    onChange,
    onBlur,
    onFocus,
    className,
    min,
    max,
    step,
    ...rest
  } = props || {};
  const hasValue = Object.prototype.hasOwnProperty.call(props || {}, "value");
  return (
    <input
      {...rest}
      type={type ?? "text"}
      name={name}
      id={id}
      value={hasValue ? (value ?? "") : undefined}
      defaultValue={hasValue ? undefined : defaultValue}
      required={required}
      disabled={disabled}
      readOnly={readOnly}
      autoComplete={autoComplete}
      placeholder={placeholder}
      onInput={onInput}
      onChange={onChange}
      onBlur={onBlur}
      onFocus={onFocus}
      className={cx(
        "flex h-9 w-full rounded-md border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg)] text-[var(--zf-ui-text)] px-3 py-1 text-sm shadow-sm transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--zf-color-brand-blue)]/40 focus-visible:border-[var(--zf-color-brand-blue)]/40 disabled:cursor-not-allowed disabled:opacity-50",
        className
      )}
      min={min}
      max={max}
      step={step}
    />
  );
}
