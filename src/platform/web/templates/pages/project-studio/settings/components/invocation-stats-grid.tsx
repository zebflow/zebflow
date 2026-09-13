import { formatBytes } from "@/components/lib/format";

/**
 * The four totals above the logging form.
 *
 * Takes the stats payload whole and does its own arithmetic, so the panel
 * above it does not have to carry derived numbers it never reads itself.
 */
export default function InvocationStatsGrid({ stats }) {
  const pipelines = Array.isArray(stats?.pipelines) ? stats.pipelines : [];
  const storeBytes =
    Number(stats?.db_bytes || 0) + Number(stats?.wal_bytes || 0) + Number(stats?.shm_bytes || 0);

  const cells = [
    { label: "Rows", value: stats?.count ?? 0 },
    { label: "Trace JSON", value: formatBytes(stats?.trace_bytes) },
    { label: "Log Store", value: formatBytes(storeBytes) },
    { label: "Pipelines", value: pipelines.length },
  ];

  return (
    <div className="grid grid-cols-4 gap-2 mb-4">
      {cells.map((cell) => (
        <div key={cell.label} className="border border-border bg-dark-panel p-3">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-muted-foreground">{cell.label}</p>
          <p className="mt-1 text-[1rem] font-semibold text-foreground">{cell.value}</p>
        </div>
      ))}
    </div>
  );
}
