export default function DialogDescription(props) {
  return (
    <p className={cx("text-sm text-slate-500 dark:text-slate-400", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </p>
  );
}
