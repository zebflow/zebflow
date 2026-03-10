export default function DropdownMenuSeparator(props) {
  return (
    <div className={cx("-mx-1 my-1 h-px bg-slate-100 dark:bg-slate-800", props?.className)} />
  );
}
