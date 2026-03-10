const HEIGHT_CLASSES = {
  sm: "h-40",
  md: "h-60",
  lg: "h-80",
  full: "h-full",
};

export function CodeEditor(props) {
  const headerVisible = props?.header !== false;
  const heightClass = HEIGHT_CLASSES[props?.height] ?? HEIGHT_CLASSES.sm;

  return (
    <div className={cx("relative group rounded-md border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-950 overflow-hidden", props?.className)}>
      <div className={cx("items-center justify-between px-3 py-1.5 border-b border-slate-200 bg-slate-50 dark:border-slate-800 dark:bg-slate-900", headerVisible ? "flex" : "hidden")}>
        <span className="text-[10px] font-medium text-slate-500 uppercase tracking-wider">{props.language}</span>
        <span className="text-[10px] text-slate-400">{props.filename}</span>
      </div>
      <div 
        data-zeb-lib="codemirror"
        data-zeb-wrapper="CodeEditor"
        data-config={typeof props?.config === "string" ? props.config : JSON.stringify(props?.config ?? {})}
        className={cx("w-full", heightClass)}
      ></div>
    </div>
  );
}

export default CodeEditor;
