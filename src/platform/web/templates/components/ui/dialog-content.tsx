function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function DialogContent(props) {
  return (
    <div className={cx("p-6 space-y-4", props?.className)}>
      {props.children}
    </div>
  );
}
