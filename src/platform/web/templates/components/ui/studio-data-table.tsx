import { cx } from "zeb";

const studioTableThClass =
  "px-[0.8rem] py-[0.65rem] border-b border-[var(--studio-border-soft)] text-left text-[0.68rem] font-mono uppercase tracking-[0.12em] text-[var(--studio-text-soft)]";

const studioTableTdClassInner =
  "px-[0.8rem] py-[0.65rem] border-b border-[var(--studio-border-soft)] text-left text-[0.8rem] text-[var(--studio-text)]";

/** For behaviors that create `<td>` in JS — same string as `StudioTd`. */
export const studioTableTdClass = studioTableTdClassInner;

export function StudioTable({ variant = "default", className, children, ...rest }) {
  const tableClass =
    variant === "dbGrid" ? "w-full border-collapse project-table" : "w-full border-collapse";
  return (
    <table className={cx(tableClass, className)} {...rest}>
      {children}
    </table>
  );
}

export function StudioThead({ className, children, ...rest }) {
  return (
    <thead className={cx("bg-[var(--studio-panel-2)]", className)} {...rest}>
      {children}
    </thead>
  );
}

export function StudioTh({ className, children, ...rest }) {
  return (
    <th className={cx(studioTableThClass, className)} {...rest}>
      {children}
    </th>
  );
}

export function StudioTd({ className, children, ...rest }) {
  return (
    <td className={cx(studioTableTdClassInner, className)} {...rest}>
      {children}
    </td>
  );
}
