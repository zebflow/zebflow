export function Select(props) {
  return (
    <div className={cx("relative group", props?.className)}>
      <select
        name={props?.name}
        required={Boolean(props?.required)}
        disabled={Boolean(props?.disabled)}
        value={props?.value}
        onChange={props?.onChange}
        className="flex h-10 w-full appearance-none rounded-md border border-slate-200 bg-white px-3 py-2 text-sm ring-offset-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-950 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-800 dark:bg-slate-950 dark:ring-offset-slate-950 dark:focus-visible:ring-slate-300 pr-10"
      >
        {props.children}
      </select>
      <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3 text-slate-500 dark:text-slate-400">
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
