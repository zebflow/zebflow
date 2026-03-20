import { cx } from "zeb";

export default function CardDescription(props) {
  return (
    <p className={cx("text-sm text-[var(--zf-ui-text-soft)]", props?.className)}>
      <span>{props?.children ?? props?.label}</span>
    </p>
  );
}
