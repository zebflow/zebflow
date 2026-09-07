import { cx } from "zeb/react";

export default function CardDescription(props) {
  return (
    <p className={cx("text-sm text-ui-text-soft", props?.className)}>
      <span>{props?.children ?? props?.label}</span>
    </p>
  );
}
