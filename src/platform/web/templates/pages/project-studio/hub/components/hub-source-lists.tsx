import Button from "@/components/ui/button";

/**
 * The two lists in the Sources dialog: sources this project owns (editable)
 * and sources the platform shares with it (read-only).
 */
export default function HubSourceLists({ hubSources, onEdit, onDelete }) {
  const projectSources = hubSources.filter((item) => item?.source_scope === "project_local" || item?.editable);
  const sharedSources = hubSources.filter((item) => item?.source_scope !== "project_local" && !item?.editable);
  return (
    <div className="mt-5 grid gap-4 lg:grid-cols-2">
      <div className="rounded-lg border border-border bg-accent/20">
        <div className="border-b border-border px-3 py-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Project sources
        </div>
        <div className="divide-y divide-border/60">
          {projectSources.map((item) => (
            <div key={`project-${item.repository_id}`} className="flex items-start justify-between gap-3 px-3 py-3">
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="truncate text-sm font-medium text-foreground">{item.title || item.repository_id}</span>
                  <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase text-muted-foreground">{item.enabled ? "enabled" : "off"}</span>
                </div>
                <p className="m-0 mt-1 truncate text-xs text-muted-foreground">{item.base_url}</p>
                <p className="m-0 mt-1 text-xs text-muted-foreground">
                  {item.repository_id}{item.has_read_token ? " · token set" : ""}
                </p>
              </div>
              <div className="flex shrink-0 gap-1">
                <Button type="button" variant="ghost" size="sm" onClick={() => onEdit(item)}>Edit</Button>
                <Button type="button" variant="ghost" size="sm" onClick={() => onDelete(item)}>Delete</Button>
              </div>
            </div>
          ))}
          {!projectSources.length ? (
            <div className="px-3 py-4 text-sm text-muted-foreground">No project-local Hub sources.</div>
          ) : null}
        </div>
      </div>

      <div className="rounded-lg border border-border bg-accent/20">
        <div className="border-b border-border px-3 py-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Shared from platform
        </div>
        <div className="divide-y divide-border/60">
          {sharedSources.map((item) => (
            <div key={`shared-${item.repository_id}`} className="px-3 py-3">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm font-medium text-foreground">{item.title || item.repository_id}</span>
                <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase text-muted-foreground">read-only</span>
              </div>
              <p className="m-0 mt-1 truncate text-xs text-muted-foreground">{item.base_url}</p>
              <p className="m-0 mt-1 text-xs text-muted-foreground">
                {item.repository_id}{item.has_read_token ? " · token set" : ""}
              </p>
            </div>
          ))}
          {!sharedSources.length ? (
            <div className="px-3 py-4 text-sm text-muted-foreground">No platform-granted Hub sources.</div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
