import { cx } from "zeb/react";

export default function CardDescription(props) {
  return (
    <p className={cx("text-sm text-muted-foreground", props?.className)}>
      <span>{props?.children ?? props?.label}</span>
    </p>
  );
}
