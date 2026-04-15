export default function ZCodeTab(props) {
  const activeClass = props?.active ? "bg-white text-gray-950 font-medium dark:bg-gray-950 dark:text-gray-50" : "text-gray-500";
  return (
    <div
      className={cx("flex items-center gap-2 px-3 py-2 text-xs border-r border-gray-200 dark:border-gray-800 cursor-pointer transition-colors hover:bg-white dark:hover:bg-gray-800", activeClass)}
      onClick={props?.onClick}
    >
      <span>{props.label}</span>
      {props?.closable ? (
        <button className="p-0.5 rounded-sm hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200">
          <svg viewBox="0 0 24 24" fill="none" className="w-3 h-3"><path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/></svg>
        </button>
      ) : null}
    </div>
  );
}
