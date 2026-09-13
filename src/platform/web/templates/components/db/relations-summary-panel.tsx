import { RelationStatsList } from "@/components/db/relations-graph";

/** How the open collection relates to the others, counted per edge type. */
export default function RelationsSummaryPanel({ tableName, stats }) {
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-foreground">Relations</p>
          <p className="mt-1 text-xs text-muted-foreground">
            Collection-level relation patterns for {tableName}.
          </p>
        </div>
      </div>

      {stats.busy ? <p className="text-xs text-muted-foreground">Loading relation statistics…</p> : null}
      {stats.error ? (
        <p className="text-xs text-danger">Failed to load relation statistics: {stats.error}</p>
      ) : null}

      <div className="grid gap-3 xl:grid-cols-2">
        <RelationStatsList
          title="Outbound By Type"
          items={stats.outgoing}
          emptyText="No outbound relation types are declared for this collection."
          peerKey="to"
        />
        <RelationStatsList
          title="Inbound By Type"
          items={stats.incoming}
          emptyText="No inbound relation types are declared for this collection."
          peerKey="from"
        />
      </div>
    </div>
  );
}
