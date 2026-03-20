export default function TabsContent(props) {
  if (!props?.active) return null;
  return (
    <div 
      className={cx("focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--zf-color-brand-blue)]/40", props?.className)}
    >
      {props.children}
    </div>
  );
}
