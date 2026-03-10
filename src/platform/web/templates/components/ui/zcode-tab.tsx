export default function ZCodeTab(props) {
  const activeClass = props?.active ? "bg-white text-slate-950 font-medium dark:bg-slate-950 dark:text-slate-50" : "text-slate-500";
  return (
    <div
      className={cx("flex items-center gap-2 px-3 py-2 text-xs border-r border-slate-200 dark:border-slate-800 cursor-pointer transition-colors hover:bg-white dark:hover:bg-slate-800", activeClass)}
      onClick={props?.onClick}
    >
      <span>{props.label}</span>
      {props?.closable ? (
        <button className="p-0.5 rounded-sm hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">
          <svg viewBox="0 0 24 24" fill="none" className="w-3 h-3"><path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/></svg>
        </button>
      ) : null}
    </div>
  );
}
