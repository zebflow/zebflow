import { cx } from "zeb";

export default function Card(props) {
  return (
    <div className={cx("rounded-xl border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg)] text-[var(--zf-ui-text)] shadow-sm overflow-hidden", props?.className)}>
      {props.children}
    </div>
  );
}
