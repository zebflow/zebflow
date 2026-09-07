import { cx } from "zeb/react";

export default function DialogDescription(props) {
  return (
    <p className={cx("text-sm text-[var(--color-body-soft,var(--color-ui-text-soft))]", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </p>
  );
}
