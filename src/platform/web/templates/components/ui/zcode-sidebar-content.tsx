function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function ZCodeSidebarContent(props) {
  return (
    <div className={cx("flex-1 overflow-y-auto p-2", props?.className)}>
      {props.children}
    </div>
  );
}
