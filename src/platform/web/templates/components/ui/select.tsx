export function Select(props) {
  return (
    <div className={cx("relative group", props?.className)}>
      <select
        name={props?.name}
        required={Boolean(props?.required)}
        disabled={Boolean(props?.disabled)}
        value={props?.value}
        onChange={props?.onChange}
        className="flex h-9 w-full appearance-none rounded-md border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg)] text-[var(--zf-ui-text)] px-3 py-1 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--zf-color-brand-blue)]/40 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 pr-8"
      >
        {props.children}
      </select>
      <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-[var(--zf-ui-text-muted)] opacity-50">
        <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
          <path d="M7 10l5 5 5-5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"/>
        </svg>
      </div>
    </div>
  );
}

export function SelectOption(props) {
  return (
    <option value={props?.value} selected={Boolean(props?.selected)}>
      {props.label}
      {props.children}
    </option>
  );
}

export default Select;
