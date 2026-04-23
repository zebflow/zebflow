export default function DialogFooter(props) {
  return (
    <div className={cx("flex items-center justify-end gap-2 border-t border-border px-6 py-4", props?.className)}>
      {props.children}
    </div>
  );
}
