function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

const ALIGN_CLASSES = {
  left: "left-0",
  right: "right-0",
  center: "left-1/2 -translate-x-1/2",
};

export default function DropdownMenuContent(props) {
  const alignClass = ALIGN_CLASSES[props?.align] ?? ALIGN_CLASSES.left;
  return (
    <div className={cx("absolute z-50 mt-2 min-w-[8rem] overflow-hidden rounded-md border border-slate-200 bg-white p-1 text-slate-950 shadow-md animate-in fade-in-80 zoom-in-95 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-50", alignClass, props?.className)}>
      {props.children}
    </div>
  );
}
