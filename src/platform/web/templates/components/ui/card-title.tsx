import { cx } from "rwe";

export default function CardTitle(props) {
  return (
    <h3 className={cx("text-2xl font-black tracking-tight text-slate-900 dark:text-slate-100", props?.className)}>
      <span>{props?.children ?? props?.label}</span>
    </h3>
  );
}
