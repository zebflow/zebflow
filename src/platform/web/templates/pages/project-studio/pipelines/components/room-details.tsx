import { formatBytes } from "@/components/lib/format";
import { StudioMetric, StudioMetricGrid } from "@/pages/project-studio/components/studio-panel";
import { formatRoomTime, roomCounterLabel } from "@/pages/project-studio/pipelines/components/room-format";

/**
 * What one room holds beyond its row: its state size, when it opened, its
 * counters and its `meta`. Counters only rise from the moment the room
 * opened, so they are shown as read; a rate is two readings apart.
 */
export default function RoomDetails({ room }) {
  const counters = room?.counters && typeof room.counters === "object" ? Object.entries(room.counters) : [];
  const meta = room?.meta && typeof room.meta === "object" && Object.keys(room.meta).length > 0 ? room.meta : null;
  return (
    <div className="flex flex-col gap-3 py-1">
      <StudioMetricGrid className="md:grid-cols-4">
        <StudioMetric label="State size" value={formatBytes(room?.state_bytes)} />
        <StudioMetric label="Opened at" value={formatRoomTime(room?.opened_at)} />
        <StudioMetric label="Opened by" value={room?.opened_by || "-"} mono />
        <StudioMetric label="Admission" value={room?.private ? "Ticket holders only" : "Join rule"} />
      </StudioMetricGrid>
      {counters.length > 0 ? (
        <StudioMetricGrid className="md:grid-cols-4">
          {counters.map(([key, value]) => (
            <StudioMetric key={key} label={roomCounterLabel(key)} value={Number(value || 0).toLocaleString()} mono />
          ))}
        </StudioMetricGrid>
      ) : null}
      <div>
        <p className="font-mono text-[0.63rem] font-medium uppercase tracking-[0.14em] text-muted-foreground">Meta</p>
        {meta ? (
          <pre className="mt-1 max-h-48 overflow-auto rounded-md border border-border bg-muted px-3 py-2 font-mono text-[0.72rem] text-foreground">{JSON.stringify(meta, null, 2)}</pre>
        ) : (
          <p className="mt-1 text-[0.74rem] text-muted-foreground">No meta.</p>
        )}
      </div>
    </div>
  );
}
