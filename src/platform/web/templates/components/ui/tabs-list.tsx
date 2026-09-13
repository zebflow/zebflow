import { cx } from "zeb/react";

export default function TabsList(props) {
  return (
    <div className={cx("inline-flex h-10 items-center justify-center rounded-md bg-accent p-1 text-muted-foreground", props?.className)}>
      {props.children}
    </div>
  );
}
