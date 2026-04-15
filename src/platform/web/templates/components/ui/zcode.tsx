export default function ZCode(props) {
  return (
    <div className={cx("flex h-full w-full overflow-hidden bg-white text-gray-900 dark:bg-gray-950 dark:text-gray-100", props?.className)}>
      {props.children}
    </div>
  );
}
