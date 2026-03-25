export default function DropdownMenuSeparator(props) {
  return (
    <div className={cx("-mx-1 my-1 h-px bg-ui-bg-muted", props?.className)} />
  );
}
