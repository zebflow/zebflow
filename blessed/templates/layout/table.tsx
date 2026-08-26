import { cx } from "zeb";

interface TableProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Table({ className, children, ...rest }: TableProps) {
  return (
    <div className="relative w-full overflow-auto">
      <table
        className={cx("w-full caption-bottom text-sm", className)}
        {...rest}
      >
        {children}
      </table>
    </div>
  );
}

export function TableHeader({ className, children, ...rest }: TableProps) {
  return (
    <thead className={cx("[&_tr]:border-b", className)} {...rest}>
      {children}
    </thead>
  );
}

export function TableBody({ className, children, ...rest }: TableProps) {
  return (
    <tbody className={cx("[&_tr:last-child]:border-0", className)} {...rest}>
      {children}
    </tbody>
  );
}

export function TableFooter({ className, children, ...rest }: TableProps) {
  return (
    <tfoot
      className={cx("border-t bg-gray-50 font-medium [&>tr]:last:border-b-0", className)}
      {...rest}
    >
      {children}
    </tfoot>
  );
}

export function TableRow({ className, children, ...rest }: TableProps) {
  return (
    <tr
      className={cx(
        "border-b border-gray-200 transition-colors hover:bg-gray-50 data-[state=selected]:bg-gray-100",
        className
      )}
      {...rest}
    >
      {children}
    </tr>
  );
}

export function TableHead({ className, children, ...rest }: TableProps) {
  return (
    <th
      className={cx(
        "h-10 px-2 text-left align-middle font-medium text-gray-500 [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px]",
        className
      )}
      {...rest}
    >
      {children}
    </th>
  );
}

export function TableCell({ className, children, ...rest }: TableProps) {
  return (
    <td
      className={cx(
        "p-2 align-middle [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px]",
        className
      )}
      {...rest}
    >
      {children}
    </td>
  );
}

export function TableCaption({ className, children, ...rest }: TableProps) {
  return (
    <caption
      className={cx("mt-4 text-sm text-gray-500", className)}
      {...rest}
    >
      {children}
    </caption>
  );
}

export default Table;
