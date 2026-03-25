export default function TabsTrigger(props) {
  return (
    <button
      type="button"
      className={cx(
        "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-blue/40 disabled:pointer-events-none disabled:opacity-50",
        props?.active ? "bg-ui-bg text-ui-text shadow-sm" : "text-ui-text-muted",
        props?.className
      )}
      onClick={props?.onClick}
    >
      {props.children}
      <span>{props.label}</span>
    </button>
  );
}
