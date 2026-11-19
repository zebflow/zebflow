import { formatBytes } from "@/components/lib/format";
import { ViolationNotice } from "@/components/package-review/findings";

function ReviewList({ title, items, danger = false }) {
  const values = Array.isArray(items) ? items.filter(Boolean) : [];
  return (
    <div className={danger ? "rounded-lg border border-red-400/30 bg-red-500/10 px-3 py-2" : "rounded-lg border border-border bg-accent/20 px-3 py-2"}>
      <p className={danger ? "m-0 text-xs font-medium text-red-100" : "m-0 text-xs font-medium text-foreground"}>{title}</p>
      {values.length ? (
        <ul className={danger ? "mt-1 space-y-1 text-[11px] text-red-50" : "mt-1 space-y-1 text-[11px] text-muted-foreground"}>
          {values.slice(0, 8).map((item, index) => <li key={`${title}-${index}`}>{String(item)}</li>)}
          {values.length > 8 ? <li>+{values.length - 8} more</li> : null}
        </ul>
      ) : (
        <p className="m-0 mt-1 text-[11px] text-muted-foreground">None</p>
      )}
    </div>
  );
}

/** What the server found when it reviewed the package before publishing. */
export default function PublishReview({ publishReview, publishReviewDirty }) {
  return (
    <div className="rounded-lg border border-border bg-accent/20 p-3 space-y-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="m-0 text-sm font-semibold text-foreground">Publish Review</p>
          <p className="m-0 mt-1 text-xs text-muted-foreground">
            {publishReview.package_id}@{publishReview.version} · {publishReview.asset_kind} · risk {publishReview.risk_level}
          </p>
        </div>
        {publishReviewDirty ? <span className="rounded-full border border-amber-400/40 px-2 py-0.5 text-[10px] uppercase text-amber-100">stale</span> : <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase text-muted-foreground">ready</span>}
      </div>
      <div className="grid gap-3 md:grid-cols-4 text-sm">
        <div><div className="text-[11px] uppercase text-muted-foreground">Files</div><div>{publishReview.total_files}</div></div>
        <div><div className="text-[11px] uppercase text-muted-foreground">Bytes</div><div>{formatBytes(publishReview.total_bytes)}</div></div>
        <div><div className="text-[11px] uppercase text-muted-foreground">Visibility</div><div>{publishReview.visibility}</div></div>
        <div><div className="text-[11px] uppercase text-muted-foreground">Cover</div><div>{publishReview.media?.[0]?.name || "None"}</div></div>
      </div>
      {publishReview.media?.length ? (
        <div className="rounded-lg border border-border bg-popover px-3 py-2 text-xs text-muted-foreground">
          Cover will be published as <code>{publishReview.media[0].name}</code> ({publishReview.media[0].content_type}, {formatBytes(publishReview.media[0].size_bytes)}).
        </div>
      ) : null}
      {publishReview.asset_kind === "project_bundle" ? (
        <div className="rounded-lg border border-border bg-popover px-3 py-2 text-xs text-muted-foreground">
          <div className="font-medium text-foreground">Initialization</div>
          <div className="mt-1">Sekejap schema: {publishReview.project_initialization?.include_sekejap_schema ? "included" : "not included"}</div>
          <div>SQLite schema: {publishReview.project_initialization?.include_sqlite_schema ? "included" : "not included"}</div>
          <div>Libraries: {(publishReview.project_initialization?.libraries || []).length ? publishReview.project_initialization.libraries.join(", ") : "none"}</div>
          <div className="mt-2 font-medium text-foreground">Initial data to execute</div>
          {(publishReview.project_initialization?.initial_data || []).length ? (
            <ul className="m-0 mt-1 space-y-1 pl-4">
              {publishReview.project_initialization.initial_data.map((item, index) => (
                <li key={`${item.path}-${index}`}>
                  <code>{item.path}</code> · {item.engine} · {item.statement_count || 0} statement(s)
                </li>
              ))}
            </ul>
          ) : (
            <div>none</div>
          )}
        </div>
      ) : null}
      <ViolationNotice items={publishReview.violations} subject="This package" outcome="published" refuser="publish" />
      <div className="grid gap-2 text-xs md:grid-cols-2">
        <ReviewList title="Nodes" items={publishReview.nodes_used} />
        <ReviewList title="Credentials" items={publishReview.credentials_required} danger />
        <ReviewList title="External URLs" items={publishReview.external_urls} danger />
        <ReviewList title="Database effects" items={publishReview.database_effects} danger />
        <ReviewList title="Filesystem effects" items={publishReview.filesystem_effects} />
        <ReviewList title="Outbound connections" items={publishReview.network_effects} danger />
        <ReviewList title="Runs supplied code" items={publishReview.code_execution} danger />
        <ReviewList title="Public endpoints" items={publishReview.public_endpoints} danger />
        <ReviewList title="Schedules" items={publishReview.schedules} danger />
        <ReviewList title="Large files" items={publishReview.large_files} />
        <ReviewList title="Seed/demo data" items={publishReview.seed_data} />
        <ReviewList title="Warnings" items={publishReview.warnings} danger />
      </div>
    </div>
  );
}
