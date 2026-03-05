function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function DialogFooter(props) {
  return (
    <div className={cx("flex justify-end gap-2 pt-4", props?.className)}>
      {props.children}
    </div>
  );
}
