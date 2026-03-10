export default function ZCode(props) {
  return (
    <div className={cx("flex h-full w-full overflow-hidden bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100", props?.className)}>
      {props.children}
    </div>
  );
}
