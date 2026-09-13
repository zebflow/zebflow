import { cx } from "zeb/react";

export default function DialogDescription(props) {
  return (
    <p className={cx("text-sm text-[var(--muted-foreground)]", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </p>
  );
}
