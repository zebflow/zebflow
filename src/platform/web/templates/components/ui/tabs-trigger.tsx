export default function TabsTrigger(props) {
  return (
    <button
      type="button"
      className={cx(
        "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--zf-color-brand-blue)]/40 disabled:pointer-events-none disabled:opacity-50",
        props?.active ? "bg-[var(--zf-ui-bg)] text-[var(--zf-ui-text)] shadow-sm" : "text-[var(--zf-ui-text-muted)]",
        props?.className
      )}
      onClick={props?.onClick}
    >
      {props.children}
      <span>{props.label}</span>
    </button>
  );
}
