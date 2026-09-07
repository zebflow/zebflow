import { cx } from "zeb/react";

export default function DialogTitle(props) {
  return (
    <h3 className={cx("font-display text-[0.95rem] font-semibold leading-[1.2] text-[var(--color-body,var(--color-ui-text))]", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </h3>
  );
}
