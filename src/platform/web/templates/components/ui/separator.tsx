function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export function Separator(props) {
  const orientation = props?.orientation === "vertical" ? "w-px h-full" : "h-px w-full";
  return (
    <div 
      className={cx("bg-slate-200 dark:bg-slate-800", orientation, props?.className)} 
      role="separator" 
    />
  );
}

export default Separator;
