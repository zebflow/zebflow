export default function DropdownMenuSeparator(props) {
  return (
    <div className={cx("-mx-1 my-1 h-px bg-[var(--zf-ui-bg-muted)]", props?.className)} />
  );
}
