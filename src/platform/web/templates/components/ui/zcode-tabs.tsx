export default function ZCodeTabs(props) {
  return (
    <div className={cx("flex items-center bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-800 min-h-[35px] overflow-x-auto no-scrollbar", props?.className)}>
      {props.children}
    </div>
  );
}
