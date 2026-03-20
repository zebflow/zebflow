import { cx } from "zeb";

export default function CardTitle(props) {
  return (
    <h3 className={cx("text-2xl font-semibold tracking-tight text-[var(--zf-ui-text)]", props?.className)}>
      <span>{props?.children ?? props?.label}</span>
    </h3>
  );
}
