export default function ZCodeSidebarHeader(props) {
  return (
    <div className={cx("px-3 py-2 border-b border-gray-200 dark:border-gray-800 flex items-center justify-between min-h-[40px]", props?.className)}>
      <span className="text-[10px] font-bold uppercase tracking-wider text-gray-500">{props.title}</span>
      <div className="flex items-center gap-1">
        {props.children}
      </div>
    </div>
  );
}
