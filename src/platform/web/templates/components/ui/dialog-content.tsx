export default function DialogContent(props) {
  return (
    <div className={cx("p-6 space-y-4", props?.className)}>
      {props.children}
    </div>
  );
}
