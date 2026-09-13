import Badge from "@/components/ui/badge";

/**
 * `database_initialization` — what the install-time SQL would do, file by file.
 *
 * The review reads the installer's own statement splitter, so a count here is a
 * count of executions rather than an estimate. Every field the report carries is
 * rendered: nothing is summarised away.
 */

const DESTRUCTIVE_KINDS = ["DROP TABLE", "DELETE FROM", "TRUNCATE", "ALTER TABLE"];

function statementRows(statements) {
  if (!statements || typeof statements !== "object") return [];
  return Object.keys(statements)
    .map((kind) => ({ kind, count: Number(statements[kind] || 0) }))
    .filter((row) => row.count > 0)
    .sort((a, b) => (b.count - a.count) || a.kind.localeCompare(b.kind));
}

function totalStatements(rows) {
  return rows.reduce((sum, row) => sum + row.count, 0);
}

function StatementChip({ kind, count }) {
  const destructive = DESTRUCTIVE_KINDS.indexOf(kind) >= 0;
  return (
    <span
      className={
        destructive
          ? "inline-flex items-center gap-1.5 rounded-md border border-red-500/40 bg-red-500/10 px-2 py-1 font-mono text-[0.7rem] text-red-600"
          : "inline-flex items-center gap-1.5 rounded-md border border-border bg-muted px-2 py-1 font-mono text-[0.7rem] text-foreground"
      }
    >
      <span>{kind}</span>
      <span className="font-semibold">×{count}</span>
    </span>
  );
}

export function DatabaseInitializationCard({ report }) {
  const rows = statementRows(report?.statements);
  const total = totalStatements(rows);
  const tables = Array.isArray(report?.tables) ? report.tables : [];
  const destructive = Array.isArray(report?.destructive) ? report.destructive : [];
  const atRisk = !!report?.existing_data_at_risk;
  const unreadable = String(report?.unreadable || "");

  return (
    <div className="rounded-lg border border-border bg-popover p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="secondary" label={String(report?.engine || "unknown engine")} />
        <span className="min-w-0 break-all font-mono text-xs text-foreground">{String(report?.source || "")}</span>
      </div>

      <div
        className={
          atRisk
            ? "mt-3 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-red-700"
            : "mt-3 rounded-md border border-green-600/30 bg-green-600/10 px-3 py-2 text-xs text-green-800"
        }
      >
        <p className="m-0 font-semibold">
          Store: {String(report?.store || "not described by this build")}
        </p>
        {atRisk ? (
          <p className="m-0 mt-1">
            existing_data_at_risk: true — this build does not describe where this SQL lands, so a
            database you already had may be exposed to it.
          </p>
        ) : (
          <p className="m-0 mt-1">
            existing_data_at_risk: false — this store is created by this install, so this SQL cannot
            reach a database you already had.
          </p>
        )}
      </div>

      {unreadable ? (
        <p className="mt-3 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-800">
          The bytes of this file could not be read, so the breakdown below is not the whole file: {unreadable}
        </p>
      ) : null}

      <div className="mt-3">
        <p className="m-0 text-[0.7rem] font-mono uppercase tracking-widest text-muted-foreground">
          Statements ({total})
        </p>
        {rows.length ? (
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {rows.map((row) => <StatementChip key={row.kind} kind={row.kind} count={row.count} />)}
          </div>
        ) : (
          <p className="m-0 mt-1 text-xs text-muted-foreground">No statements.</p>
        )}
      </div>

      <div className="mt-3">
        <p className="m-0 text-[0.7rem] font-mono uppercase tracking-widest text-muted-foreground">
          Tables touched ({tables.length})
        </p>
        {tables.length ? (
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {tables.map((table, index) => (
              <span
                key={`table-${index}`}
                className="inline-flex items-center rounded-md border border-border bg-muted px-2 py-1 font-mono text-[0.7rem] text-foreground"
              >
                {String(table)}
              </span>
            ))}
          </div>
        ) : (
          <p className="m-0 mt-1 text-xs text-muted-foreground">None named.</p>
        )}
      </div>

      <div className="mt-3">
        <p className="m-0 text-[0.7rem] font-mono uppercase tracking-widest text-muted-foreground">
          Destructive statements ({destructive.length})
        </p>
        {destructive.length ? (
          <ul className="m-0 mt-1.5 list-none space-y-1 p-0">
            {destructive.map((item, index) => (
              <li
                key={`destructive-${index}`}
                className="overflow-x-auto rounded-md border border-red-500/40 bg-red-500/10 px-2 py-1 font-mono text-[0.7rem] text-red-700"
              >
                {String(item)}
              </li>
            ))}
          </ul>
        ) : (
          <p className="m-0 mt-1 text-xs text-muted-foreground">
            None. Nothing in this file drops, deletes, truncates, or alters.
          </p>
        )}
      </div>
    </div>
  );
}

export default function SqlReport({ reports, executeSchema = true, emptyNote }) {
  const items = Array.isArray(reports) ? reports : [];
  return (
    <section className="rounded-lg border border-border bg-accent/30 p-3">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <p className="m-0 text-sm font-semibold text-foreground">Install-time SQL</p>
        <p className="m-0 text-xs text-muted-foreground">
          {items.length} file{items.length === 1 ? "" : "s"} of SQL this install would replay
        </p>
      </div>
      {items.length ? (
        <>
          {executeSchema ? null : (
            <p className="m-0 mt-2 rounded-md border border-border bg-popover px-3 py-2 text-xs text-muted-foreground">
              You have turned execution off, so none of the statements below run. The files are still
              written into the repository and this is still what they would do when you run them.
            </p>
          )}
          <div className="mt-3 space-y-3">
            {items.map((report, index) => (
              <DatabaseInitializationCard key={`db-init-${index}`} report={report} />
            ))}
          </div>
        </>
      ) : (
        <p className="m-0 mt-2 text-xs text-muted-foreground">
          {emptyNote || "This package carries no install-time SQL. Nothing is replayed into any store."}
        </p>
      )}
    </section>
  );
}
