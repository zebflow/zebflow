export default function ZCodeSidebar(props) {
  return (
    <aside className={cx("w-64 border-r border-slate-200 dark:border-slate-800 flex flex-col shrink-0", props?.className)}>
      {props.children}
    </aside>
  );
}
