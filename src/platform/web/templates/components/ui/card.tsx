import { cx } from "rwe";

export default function Card(props) {
  return (
    <div className={cx("rounded-xl border border-slate-200 bg-white text-slate-950 shadow-sm overflow-hidden dark:border-slate-800 dark:bg-slate-950 dark:text-slate-50", props?.className)}>
      {props.children}
    </div>
  );
}
