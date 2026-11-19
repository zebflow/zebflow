import { formatBytes } from "@/components/lib/format";
import { sourceLabel } from "@/pages/project-studio/hub/components/publish-source-types";

/** Step 3 of publishing: the exact file set that will be packed. */
export default function PublishPreview({ publishPreview }) {
  return (
    <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
      <div>
        <p className="project-content-subtitle">3. Export Tree Preview</p>
        <p className="text-sm text-muted-foreground">This is the exact file set that will be packed and published.</p>
      </div>
      {publishPreview ? (
        <>
          <div className="grid gap-3 md:grid-cols-4 rounded-lg border border-border bg-accent/20 px-3 py-3 text-sm">
            <div>
              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Package kind</div>
              <div className="mt-1 text-foreground">{publishPreview.asset_kind}</div>
            </div>
            <div>
              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Source</div>
              <div className="mt-1 text-foreground">{sourceLabel(publishPreview.source_type)}</div>
            </div>
            <div>
              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Files</div>
              <div className="mt-1 text-foreground">{publishPreview.total_files}</div>
            </div>
            <div>
              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Bytes</div>
              <div className="mt-1 text-foreground">{formatBytes(publishPreview.total_bytes)}</div>
            </div>
          </div>

          <div className="rounded-lg border border-border bg-accent/20">
            <div className="border-b border-border px-3 py-2 text-xs font-mono uppercase tracking-widest text-muted-foreground">Resolved Files</div>
            <div className="max-h-80 overflow-auto divide-y divide-border/60">
              {publishPreview.entries.map((entry, index) => (
                <div key={`${entry.rel_path}-${index}`} className="grid gap-2 px-3 py-2 md:grid-cols-[minmax(0,1fr)_140px_90px]">
                  <div className="min-w-0">
                    <code className="block truncate text-xs text-foreground">{entry.rel_path}</code>
                    <div className="mt-1 text-[11px] text-muted-foreground">{entry.reason}</div>
                  </div>
                  <div className="text-xs text-muted-foreground">{entry.kind}</div>
                  <div className="text-right text-xs text-muted-foreground">{entry.size_bytes}</div>
                </div>
              ))}
            </div>
          </div>

          {publishPreview.warnings?.length ? (
            <div className="rounded-lg border border-amber-400/30 bg-amber-500/10 px-3 py-3">
              <p className="text-sm font-medium text-amber-100">Warnings</p>
              <ul className="mt-2 space-y-1 text-xs text-amber-50">
                {publishPreview.warnings.map((item, index) => (
                  <li key={`${item}-${index}`}>{item}</li>
                ))}
              </ul>
            </div>
          ) : null}
        </>
      ) : (
        <div className="rounded-lg border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
          Select a source item to generate the export tree.
        </div>
      )}
    </div>
  );
}
