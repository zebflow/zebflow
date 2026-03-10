import { cx } from "rwe";

export default function CardHeader(props) {
  return (
    <div className={cx("flex flex-col space-y-1.5 px-7 py-6 border-b border-slate-100 bg-slate-50 dark:border-slate-800 dark:bg-slate-900/50", props?.className)}>
      {props.children}
    </div>
  );
}
