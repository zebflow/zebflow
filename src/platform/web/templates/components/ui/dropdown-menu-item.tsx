import { cx } from "zeb/react";

export default function DropdownMenuItem(props) {
  const isDestructive = props?.variant === "destructive";
  return (
    <div
      className={cx(
        "relative flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors hover:bg-accent hover:text-foreground",
        isDestructive ? "text-destructive hover:text-destructive" : "",
        props?.className
      )}
      onClick={props?.onClick}
    >
      {props?.icon ? <span className="mr-2 h-4 w-4">{props.icon}</span> : null}
      <span className="flex-1">{props.label}</span>
      {props.children}
    </div>
  );
}
