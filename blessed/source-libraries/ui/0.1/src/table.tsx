import { cx } from "zeb/react";

/**
 * Table — shadcn/ui's table primitives, on Zebflow's engine.
 *
 * Upstream tidies border edges with descendant selectors this engine doesn't
 * compile: `TableHeader`'s `[&_tr]:border-b` is redundant here anyway since
 * `TableRow` already carries its own `border-b`; `TableBody`'s
 * `[&_tr:last-child]:border-0` and `TableFooter`'s `[&>tr]:last:border-b-0`
 * are dropped, so the last row of a body or footer keeps its bottom border —
 * pass `className="border-b-0"` to that row yourself if you want it gone.
 * `TableHead`/`TableCell`'s checkbox-column nudge (`[&:has(…)]`,
 * `[&>[role=checkbox]]:…`) is dropped the same way.
 */

export function Table({ className, children, ...rest }) {
  return (
    <div data-slot="table-container" className="relative w-full overflow-x-auto">
      <table data-slot="table" className={cx("w-full caption-bottom text-sm", className)} {...rest}>
        {children}
      </table>
    </div>
  );
}

export function TableHeader({ className, children, ...rest }) {
  return (
    <thead data-slot="table-header" className={cx(className)} {...rest}>
      {children}
    </thead>
  );
}

export function TableBody({ className, children, ...rest }) {
  return (
    <tbody data-slot="table-body" className={cx(className)} {...rest}>
      {children}
    </tbody>
  );
}

export function TableFooter({ className, children, ...rest }) {
  return (
    <tfoot data-slot="table-footer" className={cx("border-t bg-muted/50 font-medium", className)} {...rest}>
      {children}
    </tfoot>
  );
}

export function TableRow({ className, children, ...rest }) {
  return (
    <tr
      data-slot="table-row"
      className={cx("border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted", className)}
      {...rest}
    >
      {children}
    </tr>
  );
}

export function TableHead({ className, children, ...rest }) {
  return (
    <th
      data-slot="table-head"
      className={cx("h-10 px-2 text-left align-middle font-medium whitespace-nowrap text-foreground", className)}
      {...rest}
    >
      {children}
    </th>
  );
}

export function TableCell({ className, children, ...rest }) {
  return (
    <td data-slot="table-cell" className={cx("p-2 align-middle whitespace-nowrap", className)} {...rest}>
      {children}
    </td>
  );
}

export function TableCaption({ className, children, ...rest }) {
  return (
    <caption data-slot="table-caption" className={cx("mt-4 text-sm text-muted-foreground", className)} {...rest}>
      {children}
    </caption>
  );
}

export default Table;
