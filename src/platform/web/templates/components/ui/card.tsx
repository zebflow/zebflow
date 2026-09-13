import { cx } from "zeb/react";

export default function Card(props) {
  return (
    <div className={cx("rounded-xl border border-border bg-popover text-foreground shadow-sm overflow-hidden", props?.className)}>
      {props.children}
    </div>
  );
}
