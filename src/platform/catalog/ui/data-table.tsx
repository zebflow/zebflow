import { cx } from "zeb";
import { useState } from "zeb";
import { useDebounce } from "zeb/use";

interface ColumnDef<T = any> {
  key: string;
  header: string;
  accessorFn?: (row: T) => any;
  cell?: (value: any, row: T) => any;
  sortable?: boolean;
  className?: string;
}

interface DataTableProps<T = any> {
  data: T[];
  columns: ColumnDef<T>[];
  pageSize?: number;
  searchable?: boolean;
  searchKeys?: string[];
  className?: string;
  emptyMessage?: string;
  [key: string]: any;
}

interface SortState {
  key: string;
  dir: "asc" | "desc";
}

function getVal(row: any, col: ColumnDef) {
  if (col.accessorFn) return col.accessorFn(row);
  return row[col.key];
}

function sortRows<T>(rows: T[], sort: SortState | null, columns: ColumnDef<T>[]) {
  if (!sort) return rows;
  const col = columns.find(c => c.key === sort.key);
  if (!col) return rows;
  return [...rows].sort((a, b) => {
    const av = getVal(a, col);
    const bv = getVal(b, col);
    const cmp = av < bv ? -1 : av > bv ? 1 : 0;
    return sort.dir === "asc" ? cmp : -cmp;
  });
}

function filterRows<T>(rows: T[], query: string, searchKeys: string[]) {
  if (!query.trim()) return rows;
  const q = query.toLowerCase();
  return rows.filter(row => {
    return searchKeys.some(key => {
      const v = (row as any)[key];
      return v != null && String(v).toLowerCase().includes(q);
    });
  });
}

export function DataTable<T = any>({
  data,
  columns,
  pageSize = 10,
  searchable = false,
  searchKeys = [],
  className,
  emptyMessage = "No results.",
  ...rest
}: DataTableProps<T>) {
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<SortState | null>(null);
  const [page, setPage] = useState(0);
  const debouncedSearch = useDebounce(search, 300);

  const effectiveSearchKeys = searchKeys.length > 0 ? searchKeys : columns.map(c => c.key);

  let rows = filterRows(data, debouncedSearch, effectiveSearchKeys);
  rows = sortRows(rows, sort, columns);
  const totalPages = Math.max(1, Math.ceil(rows.length / pageSize));
  const currentPage = Math.min(page, totalPages - 1);
  const pageRows = rows.slice(currentPage * pageSize, (currentPage + 1) * pageSize);

  const handleSort = (key: string) => {
    setSort(prev => {
      if (prev?.key === key) {
        return prev.dir === "asc" ? { key, dir: "desc" } : null;
      }
      return { key, dir: "asc" };
    });
    setPage(0);
  };

  return (
    <div className={cx("space-y-4", className)} {...rest}>
      {searchable && (
        <input
          type="search"
          placeholder="Search..."
          value={search}
          onChange={e => { setSearch(e.target.value); setPage(0); }}
          className="flex h-9 w-full max-w-sm rounded-md border border-gray-200 bg-transparent px-3 py-1 text-sm shadow-sm placeholder:text-gray-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-950"
        />
      )}

      <div className="relative w-full overflow-auto rounded-md border border-gray-200">
        <table className="w-full caption-bottom text-sm">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50">
              {columns.map(col => (
                <th
                  key={col.key}
                  className={cx(
                    "h-10 px-4 text-left align-middle font-medium text-gray-500",
                    col.sortable ? "cursor-pointer select-none hover:text-gray-900" : "",
                    col.className
                  )}
                  onClick={col.sortable ? () => handleSort(col.key) : undefined}
                >
                  <span className="flex items-center gap-1">
                    {col.header}
                    {col.sortable && (
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={cx("h-3 w-3 transition-transform", sort?.key === col.key ? (sort.dir === "asc" ? "" : "rotate-180") : "opacity-30")}>
                        <path d="m6 9 6 6 6-6" />
                      </svg>
                    )}
                  </span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {pageRows.length === 0 ? (
              <tr>
                <td colSpan={columns.length} className="h-24 text-center text-gray-500">
                  {emptyMessage}
                </td>
              </tr>
            ) : (
              pageRows.map((row, ri) => (
                <tr key={ri} className="border-b border-gray-200 transition-colors hover:bg-gray-50">
                  {columns.map(col => {
                    const value = getVal(row, col);
                    return (
                      <td key={col.key} className={cx("p-4 align-middle", col.className)}>
                        {col.cell ? col.cell(value, row) : String(value ?? "")}
                      </td>
                    );
                  })}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {totalPages > 1 && (
        <div className="flex items-center justify-between px-2">
          <p className="text-sm text-gray-500">
            Page {currentPage + 1} of {totalPages} ({rows.length} rows)
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setPage(p => Math.max(0, p - 1))}
              disabled={currentPage === 0}
              className="inline-flex items-center justify-center h-8 w-8 rounded-md border border-gray-200 text-sm hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
                <path d="m15 18-6-6 6-6" />
              </svg>
            </button>
            <button
              type="button"
              onClick={() => setPage(p => Math.min(totalPages - 1, p + 1))}
              disabled={currentPage >= totalPages - 1}
              className="inline-flex items-center justify-center h-8 w-8 rounded-md border border-gray-200 text-sm hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
                <path d="m9 18 6-6-6-6" />
              </svg>
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default DataTable;
