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
    <div className={cx("relative group rounded-md border border-ui-border bg-ui-bg overflow-hidden", props?.className)}>
      <div className={cx("items-center justify-between px-3 py-1.5 border-b border-ui-border bg-ui-bg-subtle", headerVisible ? "flex" : "hidden")}>
        <span className="text-[10px] font-medium text-ui-text-muted uppercase tracking-wider">{props.language}</span>
        <span className="text-[10px] text-ui-text-muted">{props.filename}</span>
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
