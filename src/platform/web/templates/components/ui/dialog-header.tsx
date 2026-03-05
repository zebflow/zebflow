function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function DialogHeader(props) {
  return (
    <div className={cx("space-y-1.5", props?.className)}>
      {props.children}
    </div>
  );
}
