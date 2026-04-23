export default function DialogDescription(props) {
  return (
    <p className={cx("text-sm text-body-soft", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </p>
  );
}
