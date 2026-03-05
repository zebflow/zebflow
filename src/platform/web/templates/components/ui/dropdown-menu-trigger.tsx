function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function DropdownMenuTrigger(props) {
  return (
    <summary className={cx("list-none cursor-pointer outline-none", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </summary>
  );
}
