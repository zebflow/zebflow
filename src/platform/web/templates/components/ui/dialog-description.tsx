export default function DialogDescription(props) {
  return (
    <p className={cx("text-sm text-ui-text-muted", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </p>
  );
}
