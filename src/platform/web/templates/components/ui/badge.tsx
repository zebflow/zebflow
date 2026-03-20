const VARIANT_CLASSES = {
  default:     "border-transparent bg-[var(--zf-ui-text)] text-[var(--zf-ui-bg)]",
  secondary:   "border-transparent bg-[var(--zf-ui-bg-muted)] text-[var(--zf-ui-text)]",
  destructive: "border-transparent bg-red-500 text-slate-50",
  outline:     "text-[var(--zf-ui-text)] border-[var(--zf-ui-border)]",
};

export default function Badge(props) {
  const variant = VARIANT_CLASSES[props?.variant] ?? VARIANT_CLASSES.default;
  return (
    <div className={cx("inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--zf-color-brand-blue)]/40", variant, props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </div>
  );
}
