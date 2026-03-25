const VARIANT_CLASSES = {
  default:     "border-transparent bg-ui-text text-ui-bg",
  secondary:   "border-transparent bg-ui-bg-muted text-ui-text",
  destructive: "border-transparent bg-red-500 text-slate-50",
  outline:     "text-ui-text border-ui-border",
};

export default function Badge(props) {
  const variant = VARIANT_CLASSES[props?.variant] ?? VARIANT_CLASSES.default;
  return (
    <div className={cx("inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-brand-blue/40", variant, props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </div>
  );
}
