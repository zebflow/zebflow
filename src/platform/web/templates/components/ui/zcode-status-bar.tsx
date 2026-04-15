export default function ZCodeStatusBar(props) {
  return (
    <div className={cx("flex items-center gap-4 px-3 py-1 bg-gray-100 text-gray-600 dark:bg-gray-900 dark:text-gray-400 border-t border-gray-200 dark:border-gray-800 text-[10px]", props?.className)}>
      {props.children}
    </div>
  );
}
