import { cx } from "zeb/react";

export default function CardHeader(props) {
  return (
    <div className={cx("flex flex-col space-y-1.5 px-6 py-4 border-b border-border bg-muted", props?.className)}>
      {props.children}
    </div>
  );
}
