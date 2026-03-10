export default function ZCodeTabs(props) {
  return (
    <div className={cx("flex items-center bg-slate-50 dark:bg-slate-900/50 border-b border-slate-200 dark:border-slate-800 min-h-[35px] overflow-x-auto no-scrollbar", props?.className)}>
      {props.children}
    </div>
  );
}
