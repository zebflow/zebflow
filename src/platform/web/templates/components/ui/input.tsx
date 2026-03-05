function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function Input(props) {
  const hasValue = Object.prototype.hasOwnProperty.call(props || {}, "value");
  return (
    <input
      type={props?.type ?? "text"}
      name={props?.name}
      id={props?.id}
      value={hasValue ? (props?.value ?? "") : undefined}
      defaultValue={hasValue ? undefined : props?.defaultValue}
      required={props?.required}
      disabled={props?.disabled}
      readOnly={props?.readOnly}
      autoComplete={props?.autoComplete}
      placeholder={props?.placeholder}
      onInput={props?.onInput}
      onChange={props?.onChange}
      onBlur={props?.onBlur}
      onFocus={props?.onFocus}
      className={cx(
        "flex h-9 w-full rounded-md border border-slate-200 bg-white px-3 py-1 text-sm shadow-sm transition-all placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-[var(--zf-color-brand-blue)]/10 focus-visible:border-[var(--zf-color-brand-blue)]/40 disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-100",
        props?.className
      )}
      min={props?.min}
      max={props?.max}
      step={props?.step}
    />
  );
}
