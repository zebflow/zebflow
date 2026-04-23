export default function DialogHeader(props) {
  return (
    <div className={cx("space-y-1.5 border-b border-border px-6 pt-6 pb-4", props?.className)}>
      {props.children}
    </div>
  );
}
