import { cx } from "rwe";

export default function DropdownMenuTrigger({ className, children, label, ...rest }) {
  return (
    <summary className={cx("list-none cursor-pointer outline-none", className)} {...rest}>
      {children}
      {label ? <span>{label}</span> : null}
    </summary>
  );
}
