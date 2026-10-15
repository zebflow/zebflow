import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import { StudioTable, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { StudioEmptyState } from "@/pages/project-studio/components/studio-panel";
import PipelineTabHeader from "@/pages/project-studio/pipelines/components/pipeline-tab-header";
import RoomRow from "@/pages/project-studio/pipelines/components/room-row";

/**
 * The rooms open in this project right now, read from the rooms API.
 *
 * There is no push: the list is as fresh as the last Refresh. Closing a room
 * drops its state and disconnects its sessions, so it asks first.
 */
export default function RoomsPanel({ api, title, subtitle }) {
  const [rooms, setRooms] = useState([]);
  const [total, setTotal] = useState({ rooms: 0, sessions: 0 });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [expanded, setExpanded] = useState("");
  const [closing, setClosing] = useState("");

  async function load() {
    setLoading(true);
    setError("");
    try {
      const data = await requestJson(api);
      if (!data) return;
      setRooms(Array.isArray(data.rooms) ? data.rooms : []);
      setTotal({ rooms: Number(data.total?.rooms || 0), sessions: Number(data.total?.sessions || 0) });
    } catch (e) {
      // A server without the rooms API has no rooms to show.
      setRooms([]);
      setTotal({ rooms: 0, sessions: 0 });
      if ((e as any)?.status !== 404) setError((e as any)?.message || "Could not read the rooms.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { load(); }, []);

  async function closeRoom(name) {
    setError("");
    try {
      await requestJson(`${api}/${encodeURIComponent(name)}/close`, { method: "POST" });
    } catch (e) {
      setError((e as any)?.message || `Could not close ${name}.`);
    }
    await load();
  }

  return (
    <div className="flex-1 min-h-0 overflow-auto flex flex-col" data-rooms-panel="true" data-rooms-api={api}>
      <PipelineTabHeader
        title={title}
        subtitle={subtitle}
        trailing={
          <>
            <Badge variant="secondary">{total.rooms} rooms · {total.sessions} sessions</Badge>
            <Button variant="outline" size="xs" disabled={loading} onClick={() => load()}>
              {loading ? "Refreshing…" : "Refresh"}
            </Button>
          </>
        }
      />
      <div className="flex-1 min-h-0 overflow-auto px-[0.875rem] py-3 flex flex-col gap-3">
        {error ? <p className="text-[0.76rem] text-destructive">{error}</p> : null}
        {rooms.length === 0 ? (
          <StudioEmptyState>
            {loading
              ? "Reading rooms…"
              : "No rooms are open. A fixed room opens while its trigger.room pipeline is active; an opened room, when a pipeline runs ws.room.open."}
          </StudioEmptyState>
        ) : (
          <div className="overflow-x-auto rounded-md border border-border">
            <StudioTable>
              <StudioThead>
                <tr>
                  <StudioTh>Room</StudioTh>
                  <StudioTh>Kind</StudioTh>
                  <StudioTh>Mode</StudioTh>
                  <StudioTh>Sessions</StudioTh>
                  <StudioTh>Opened by</StudioTh>
                  <StudioTh>Last activity</StudioTh>
                  <StudioTh />
                </tr>
              </StudioThead>
              {rooms.map((room) => (
                <RoomRow
                  key={room?.room}
                  room={room}
                  expanded={expanded === room?.room}
                  onToggle={() => setExpanded(expanded === room?.room ? "" : room?.room)}
                  onClose={() => setClosing(room?.room)}
                />
              ))}
            </StudioTable>
          </div>
        )}
      </div>
      <ConfirmDialog
        open={!!closing}
        onClose={() => setClosing("")}
        onConfirm={() => closeRoom(closing)}
        title={`Close ${closing}?`}
        message="Every session in the room is disconnected and its state is dropped."
        confirmLabel="Close room"
        variant="destructive"
      />
    </div>
  );
}
