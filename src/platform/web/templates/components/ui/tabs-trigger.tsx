export default function TabsTrigger(props) {
  return (
    <button
      type="button"
      className={cx(
        "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-white transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-950 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 dark:ring-offset-slate-950 dark:focus-visible:ring-slate-300",
        props?.active ? "bg-white text-slate-950 shadow-sm dark:bg-slate-950 dark:text-slate-50" : "",
        props?.className
      )}
      onClick={props?.onClick}
    >
      {props.children}
      <span>{props.label}</span>
    </button>
  );
}
