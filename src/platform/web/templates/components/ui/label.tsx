import { cx } from "zeb/react";

export default function Label(props) {
  return (
    <label
      htmlFor={props?.htmlFor ?? props?.for}
      className={cx("text-xs font-mono uppercase tracking-widest text-muted-foreground leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70", props?.className)}
    >
      <span>{props.label}</span>
    </label>
  );
}
