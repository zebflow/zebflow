export default function DropdownMenuItem(props) {
  const isDestructive = props?.variant === "destructive";
  return (
    <div
      className={cx(
        "relative flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors hover:bg-[var(--zf-ui-bg-muted)] hover:text-[var(--zf-ui-text)]",
        isDestructive ? "text-red-500 hover:text-red-500" : "",
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
