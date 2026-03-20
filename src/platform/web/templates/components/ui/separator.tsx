export function Separator(props) {
  const orientation = props?.orientation === "vertical" ? "w-px h-full" : "h-px w-full";
  return (
    <div 
      className={cx("bg-[var(--zf-ui-border)]", orientation, props?.className)}
      role="separator" 
    />
  );
}

export default Separator;
