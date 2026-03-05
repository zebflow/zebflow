function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function ZCodeStatusBar(props) {
  return (
    <div className={cx("flex items-center gap-4 px-3 py-1 bg-slate-100 text-slate-600 dark:bg-slate-900 dark:text-slate-400 border-t border-slate-200 dark:border-slate-800 text-[10px]", props?.className)}>
      {props.children}
    </div>
  );
}
