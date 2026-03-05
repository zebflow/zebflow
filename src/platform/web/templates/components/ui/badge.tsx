function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

const VARIANT_CLASSES = {
  default: "border-transparent bg-slate-900 text-slate-50 hover:bg-slate-900/80 dark:bg-slate-50 dark:text-slate-900",
  secondary: "border-transparent bg-slate-100 text-slate-900 hover:bg-slate-100/80 dark:bg-slate-800 dark:text-slate-50",
  destructive: "border-transparent bg-red-500 text-slate-50 hover:bg-red-500/80 dark:bg-red-900 dark:text-slate-50",
  outline: "text-slate-950 dark:text-slate-50 border-slate-200 dark:border-slate-800",
};

export default function Badge(props) {
  const variant = VARIANT_CLASSES[props?.variant] ?? VARIANT_CLASSES.default;
  return (
    <div className={cx("inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-slate-950 focus:ring-offset-2 dark:focus:ring-slate-300", variant, props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </div>
  );
}
