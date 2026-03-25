import { cx } from "zeb";

export default function Card(props) {
  return (
    <div className={cx("rounded-xl border border-ui-border bg-ui-bg text-ui-text shadow-sm overflow-hidden", props?.className)}>
      {props.children}
    </div>
  );
}
