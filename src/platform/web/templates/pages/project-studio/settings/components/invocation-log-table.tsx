import Button from "@/components/ui/button";
import { formatBytes } from "@/components/lib/format";
import { formatOperationTimestamp } from "@/pages/project-studio/settings/components/settings-lib";

const ROW_GRID =
  "grid grid-cols-[minmax(0,1fr)_5.5rem_7rem_7rem_10rem_5rem] gap-2 border-b border-dark-border px-3 py-2";

/**
 * One row per pipeline that has stored invocations.
 *
 * Clearing is handed up rather than done here: the confirmation and the
 * reload after it belong to whoever owns the stats.
 */
export default function InvocationLogTable({ pipelines, loadingStats, clearing, onClear }) {
  const rows = Array.isArray(pipelines) ? pipelines : [];

  return (
    <div className="border border-dark-border">
      <div className={`${ROW_GRID} bg-dark-panel text-[0.68rem] font-semibold uppercase tracking-[0.08em] text-body-soft`}>
        <span>Pipeline</span>
        <span className="text-right">Rows</span>
        <span className="text-right">Trace</span>
        <span className="text-right">Largest</span>
        <span>Latest</span>
        <span className="text-right">Action</span>
      </div>
      <div className="max-h-[420px] overflow-auto">
        {rows.length ? rows.map((item) => (
          <div key={item.file_rel_path} className={`${ROW_GRID} text-[0.74rem] last:border-b-0`}>
            <span className="truncate font-mono text-body" title={item.file_rel_path}>{item.file_rel_path}</span>
            <span className="text-right text-body-soft">{item.count}</span>
            <span className="text-right text-body-soft">{formatBytes(item.trace_bytes)}</span>
            <span className="text-right text-body-soft">{formatBytes(item.largest_trace_bytes)}</span>
            <span className="truncate text-body-soft">{formatOperationTimestamp(item.latest_at)}</span>
            <span className="text-right">
              <Button
                type="button"
                variant="outline"
                size="xs"
                disabled={clearing}
                onClick={() => onClear(item.file_rel_path)}
              >
                Clear
              </Button>
            </span>
          </div>
        )) : (
          <div className="px-3 py-4 text-[0.78rem] text-body-soft">
            {loadingStats ? "Loading invocation logs..." : "No invocation logs stored for this project."}
          </div>
        )}
      </div>
    </div>
  );
}
