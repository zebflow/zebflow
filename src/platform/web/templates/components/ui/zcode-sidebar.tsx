export default function ZCodeSidebar(props) {
  return (
    <aside className={cx("w-64 border-r border-gray-200 dark:border-gray-800 flex flex-col shrink-0", props?.className)}>
      {props.children}
    </aside>
  );
}
