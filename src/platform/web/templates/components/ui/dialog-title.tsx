function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function DialogTitle(props) {
  return (
    <h3 className={cx("text-lg font-semibold leading-none tracking-tight", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </h3>
  );
}
