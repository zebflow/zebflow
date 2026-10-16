# zeb/rooms

The client for Zebflow rooms. A page joins a room, keeps the room's state in
step, sends events and reconnects on its own. It speaks the wire protocol
`zebflow.rooms.v1` (help `pipeline/rooms-protocol`); nothing else is needed.

## In a component

```tsx
import { useRoom } from "zeb/rooms";

export default function Match() {
  const { state, status, send } = useRoom("match-7", {
    ticket: "/api/matches/7/ticket",          // or a function, or leave it out for an open room
    select: (s) => s.players,                  // optional: re-render only when this part changes
    onEvent: (event, payload) => { /* … */ },  // optional
  });
  if (status !== "joined") return <p>{status}</p>;
  return <Board players={state} onMove={(x, y) => send("move", { x, y })} />;
}
```

## Without React

```js
import { joinRoom } from "zeb/rooms";

const room = joinRoom("match-7", { ticket: "/api/matches/7/ticket" });
const off = room.on("hit", (payload) => {});  // returns an unsubscribe function
room.onState((state, ops) => {});            // after every patch or resync
room.onStatus((status, detail) => {});       // connecting, joined, away, closed
room.send("move", { x: 3, y: 7 });            // true if sent; false unless joined
room.leave();

// A page outside Zebflow names where rooms live:
joinRoom("match-7", { url: "wss://play.example.com/_ws/rooms", ticket: () => getTicket() });
```

## Behaviour

- **Where.** A Zebflow page is told by its server where its rooms live. With
  the project's `ws` surface off, a room closes at once with reason
  `rooms-off`.
- **Tickets.** A string is a URL, POSTed with the page's cookies; the answer's
  `ticket` is used. A function returns a ticket or a promise of one. Fetched
  again for every fresh join; never for a resume. A failed fetch closes the
  room with reason `ticket` and the HTTP `status`.
- **State.** `joined` and `resync` replace it; `patch` changes only the
  objects along each changed path, so everything else keeps its identity.
  A tick's ops apply before its events are delivered.
- **Statuses.** `connecting`, `joined`, `away` (reconnecting), `closed`
  (`room.closed` is `{ code, reason }`).
- **Reconnecting.** A dropped connection retries with backoff (about 250 ms,
  doubling, at most 5 s). In a room with a hold it resumes the seat; when the
  hold has ended it joins afresh with a new ticket. Close codes 4401, 4404,
  4408, 4409, 4410, 4429 and 1008 are never retried.
- **Sending** is now or never: `send` is false unless joined, and names
  starting with `$` belong to the server.

## Tests

`node --test tests/rwe/runtime/rooms.test.mjs` from the repository root.
