function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function DropdownMenu(props) {
  return (
    <details className={cx("relative inline-block group", props?.className)} data-dropdown-menu="true">
      {props.children}
    </details>
  );
}
