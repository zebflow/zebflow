import { cx } from "rwe";

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
        "flex h-9 w-full rounded-md border border-slate-200 bg-white px-3 py-1 text-sm shadow-sm transition-all placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-[var(--zf-color-brand-blue)]/10 focus-visible:border-[var(--zf-color-brand-blue)]/40 disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-100",
        className
      )}
      min={min}
      max={max}
      step={step}
    />
  );
}
