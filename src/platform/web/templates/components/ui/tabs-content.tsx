export default function TabsContent(props) {
  if (!props?.active) return null;
  return (
    <div 
      className={cx("ring-offset-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-950 focus-visible:ring-offset-2 dark:ring-offset-slate-950 dark:focus-visible:ring-slate-300", props?.className)}
    >
      {props.children}
    </div>
  );
}
