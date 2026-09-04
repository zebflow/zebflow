import { cx } from "zeb";
import Button from "@/components/ui/button";

/**
 * Storage health and upkeep for engines that expose one.
 *
 * Declared by `capabilities.maintenance`. The panel reads a health report
 * and a delta; it does not know which engine produced them.
 */

function formatBytes(value) {
  const bytes = Number(value || 0);
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let amount = bytes;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  const fixed = amount >= 100 || unit === 0 ? 0 : amount >= 10 ? 1 : 2;
  return `${amount.toFixed(fixed)} ${units[unit]}`;
}

function maintenanceDelta(report, key) {
  if (!report?.before || !report?.after) return "";
  const before = Number(report.before?.[key] || 0);
  const after = Number(report.after?.[key] || 0);
  const diff = after - before;
  if (!diff) return "";
  return `${diff > 0 ? "+" : "-"}${formatBytes(Math.abs(diff))}`;
}

export default function MaintenancePanel({ health, report, busy, status, onRefresh, onSync, onCompact }) {
  const walBytes = Number(health?.wal_bytes || 0);
  const shouldCompact = walBytes >= 64 * 1024 * 1024;
  const statItems = [
    { label: "Nodes", value: Number(health?.node_count || 0).toLocaleString() },
    { label: "Edges", value: Number(health?.edge_count || 0).toLocaleString() },
    { label: "WAL", value: formatBytes(health?.wal_bytes), delta: maintenanceDelta(report, "wal_bytes") },
    { label: "Snapshot", value: formatBytes(health?.snapshot_bytes), delta: maintenanceDelta(report, "snapshot_bytes") },
    { label: "Payload", value: formatBytes(health?.payload_bytes), delta: maintenanceDelta(report, "payload_bytes") },
    { label: "Indexes", value: formatBytes(health?.sidecar_bytes), delta: maintenanceDelta(report, "sidecar_bytes") },
    { label: "Check Time", value: `${Number(health?.duration_ms || 0)} ms` },
    { label: "Root", value: health?.root ? String(health.root) : "Not loaded", mono: true },
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-4">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-ui-text">Sekejap Store Maintenance</p>
          <p className="mt-1 max-w-2xl text-xs text-ui-text-soft">
            Inspect the project-local store, flush pending WAL writes, and compact the snapshot during low-traffic windows.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onRefresh}>
            Refresh
          </Button>
          <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onSync}>
            Sync WAL
          </Button>
          <Button type="button" size="sm" disabled={busy} onClick={onCompact}>
            Compact
          </Button>
        </div>
      </div>

      {status ? (
        <div
          className={cx(
            "mb-4 rounded-md border px-3 py-2 text-xs",
            status.startsWith("Error")
              ? "border-red-300/70 bg-red-50/50 text-red-700 dark:border-red-800/60 dark:bg-red-950/20 dark:text-red-400"
              : "border-ui-border/80 bg-ui-bg-muted/30 text-ui-text-soft"
          )}
        >
          {status}
        </div>
      ) : null}

      {shouldCompact ? (
        <div className="mb-4 rounded-md border border-amber-300/70 bg-amber-50/50 px-3 py-2 text-xs text-amber-800 dark:border-amber-800/60 dark:bg-amber-950/20 dark:text-amber-300">
          WAL is above 64 MB. Compacting will checkpoint the store into a clean snapshot and truncate replay data.
        </div>
      ) : null}

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        {statItems.map((item) => (
          <div key={item.label} className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/20 p-3">
            <p className="text-[0.68rem] font-medium uppercase tracking-[0.14em] text-ui-text-soft">{item.label}</p>
            <p className={cx("mt-1 truncate text-sm font-medium text-ui-text", item.mono ? "font-mono text-[0.72rem]" : "")} title={item.value}>
              {item.value}
            </p>
            {item.delta ? (
              <p className="mt-1 text-[0.68rem] tabular-nums text-ui-text-soft">{item.delta}</p>
            ) : null}
          </div>
        ))}
      </div>

      {report ? (
        <div className="mt-4 rounded-lg border border-ui-border/80 bg-ui-bg-muted/10 p-3">
          <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Last Operation</p>
          <p className="mt-2 text-sm text-ui-text">
            {report.operation} completed in {Number(report.duration_ms || 0)} ms.
          </p>
        </div>
      ) : null}
    </div>
  );
}
