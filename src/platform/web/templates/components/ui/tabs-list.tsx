function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function TabsList(props) {
  return (
    <div className={cx("inline-flex h-10 items-center justify-center rounded-md bg-slate-100 p-1 text-slate-500 dark:bg-slate-800 dark:text-slate-400", props?.className)}>
      {props.children}
    </div>
  );
}
