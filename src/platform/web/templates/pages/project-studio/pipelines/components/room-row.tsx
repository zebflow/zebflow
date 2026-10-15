import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import { StudioTd } from "@/components/ui/studio-data-table";
import RoomDetails from "@/pages/project-studio/pipelines/components/room-details";
import { formatRoomTime } from "@/pages/project-studio/pipelines/components/room-format";

/** Columns in a room row, so the details row can span them all. */
const ROOM_COLUMNS = 7;

/**
 * One open room: its row, and its details below it while expanded. Each room
 * is its own `<tbody>`, so the two rows stay together without a fragment.
 */
export default function RoomRow({ room, expanded, onToggle, onClose }) {
  const name = String(room?.room ?? "");
  const cap = room?.max_sessions ? String(room.max_sessions) : "∞";
  return (
    <tbody>
      <tr className="hover:bg-muted/50">
        <StudioTd>
          <button
            type="button"
            className="inline-flex items-center gap-1.5 font-mono text-left text-foreground hover:text-primary"
            aria-expanded={expanded ? "true" : "false"}
            onClick={onToggle}
          >
            <span className="text-muted-foreground" aria-hidden="true">{expanded ? "▾" : "▸"}</span>
            <span className="break-all">{name}</span>
          </button>
          {room?.private ? <Badge variant="outline" className="ml-2">private</Badge> : null}
        </StudioTd>
        <StudioTd><Badge variant="secondary">{room?.kind ?? "-"}</Badge></StudioTd>
        <StudioTd><Badge variant="outline">{room?.mode ?? "-"}</Badge></StudioTd>
        <StudioTd className="font-mono">{Number(room?.sessions || 0)} / {cap}</StudioTd>
        <StudioTd className="font-mono text-[0.72rem] text-muted-foreground">{room?.opened_by || "-"}</StudioTd>
        <StudioTd className="text-[0.74rem] text-muted-foreground">{formatRoomTime(room?.last_activity_at)}</StudioTd>
        <StudioTd className="text-right">
          <Button variant="destructive" size="xs" onClick={onClose}>Close</Button>
        </StudioTd>
      </tr>
      {expanded ? (
        <tr>
          <StudioTd colSpan={ROOM_COLUMNS} className="bg-card">
            <RoomDetails room={room} />
          </StudioTd>
        </tr>
      ) : null}
    </tbody>
  );
}
