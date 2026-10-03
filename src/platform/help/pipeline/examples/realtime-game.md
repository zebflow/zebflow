# Real-Time Game (WebSocket State Sync)

## What this builds

A multiplayer game where all clients share synchronized state through
WebSocket rooms. State is managed server-side and synced to all participants.
Players join rooms, send moves, state updates are broadcast.

---

## Pipelines

1. `GET /game` → render game lobby page
2. `GET /game/:room` → render game room with initial state
3. `POST /api/game/rooms` → create room with initial state
4. `WS event: player.join` → add player to room state → broadcast
5. `WS event: player.move` → validate move → update game state → broadcast
6. `WS event: player.leave` → remove player → broadcast

---

## Tables

```sql
CREATE TABLE game_rooms (_key TEXT PRIMARY KEY, name TEXT, status TEXT, created_at INTEGER)
```

Room *state* (players, board, last move) lives in the WebSocket room's shared
state object (`ws.sync_state`), not in this table — the table only tracks
which rooms exist and whether they're joinable.

---

## DSL

### game-lobby — lobby page

```
| trigger.webhook --path /game --method GET
| sekejap.query -- "SELECT * FROM game_rooms WHERE status = 'waiting'"
| script -- "return { rooms: input.rows }"
| web.response --template pages/game-lobby.tsx
```

### game-room — room with initial state

```zf
register game/room --
[a] trigger.webhook --path /game/:room --method GET
[b] sekejap.query --params "{{ [input.params.room] }}" -- "SELECT * FROM game_rooms WHERE _key = $1"
[c] logic.if --expr "input.rows.length > 0"
[d] script -- "return { room: input.rows[0] };"
[e] web.response --template pages/game-room.tsx
[f] web.response --location /game

[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[d] -> [e]
[c]:false -> [f]
```

### api-room-create — create game room

```zf
register game/api-room-create --
[trig] trigger.webhook --path /api/game/rooms --method POST
[draft] script -- "const id = 'room-' + Math.random().toString(36).slice(2,8); return { id, name: (input.body && input.body.name) || id };"
[ins] sekejap.query --read-only false --params "{{ [$nodes.draft.id, $nodes.draft.name, Date.now()] }}" -- "INSERT INTO game_rooms (_key, name, status, created_at) VALUES ($1, $2, 'waiting', $3)"
[ok] script -- "return { ok: true, room_id: $nodes.draft.id };"

[trig] -> [draft]
[draft] -> [ins]
[ins] -> [ok]
```

### ws-player-join — player joins room

`--path` on `ws.sync_state` supports `{key}` placeholders read from the
payload, so `player_id` is lifted to the top level first:

```zf
register game/ws-player-join --
[a] trigger.ws --event player.join
[lift] script -- "return { player_id: input.payload.player_id, name: input.payload.name };"
[merge] ws.sync_state --op merge --path "/players/{player_id}" --value "{{ { id: input.player_id, name: input.name, score: 0, joined_at: Date.now() } }}"
[emit] ws.emit --to all --event state.updated

[a] -> [lift]
[lift] -> [merge]
[merge] -> [emit]
```

### ws-player-move — player makes a move

```zf
register game/ws-player-move --
[a] trigger.ws --event player.move
[guard] logic.if --expr "!!(input.payload && input.payload.player_id && input.payload.move)"
[lift] script -- "return { player_id: input.payload.player_id, move: input.payload.move, ts: Date.now() };"
[set] ws.sync_state --op set --path "/last_move" --value "{{ input }}"
[emit] ws.emit --to all --event player.moved --payload "{{ input }}"

[a] -> [guard]
[guard]:true -> [lift]
[lift] -> [set]
[set] -> [emit]
```

An invalid move just stops at `[guard]` — there is no `false` edge, and a
`script` returning `null` would not have stopped the pipeline anyway; only a
`logic.if` branch does.

### ws-player-leave — player disconnects

```
| trigger.ws --event player.leave
| ws.emit --to all --event player.left --payload "{{ { player_id: input.payload.player_id } }}"
```

---

## Nodes Used

- `trigger.webhook` — HTTP lobby and room pages
- `trigger.ws --event <name>` — WebSocket event handlers (join, move, leave); `--room` omitted, it is a literal filter, not per-connection routing
- `sekejap.query` — track which rooms exist; plain `SELECT`/`INSERT`, no `--table`/`--op`
- `script` — lift nested payload fields, move validation
- `ws.sync_state --path "/…/{key}" --value "{{ expr }}"` — merge/set patches into the server-side room state
- `ws.emit --payload "{{ expr }}"` — broadcast events to all players in the room

---

## State Sync Protocol

Server-side room state is a JSON object. Clients receive:

```json
{ "type": "joined", "session_id": "...", "room": "room-abc123", "state": { "players": {}, "last_move": null } }
{ "type": "state_patch", "state": { "players": { "p1": { "id": "p1", "name": "...", "score": 0 } }, "last_move": null } }
{ "type": "event", "event": "player.moved", "payload": { "player_id": "p1", "move": "...", "ts": 1234 } }
{ "type": "resync", "state": { "players": {}, "last_move": null } }
```

`state_patch` and `resync` always carry the **full** current state, not a diff —
the client replaces its local copy wholesale. `resync` arrives when the
connection fell behind and messages were dropped.

`ws.emit --to session` / `--to others` is enforced by the server: a session-only
event reaches that one socket and no other.

## Joining, leaving and who is online

The server raises two reserved events on every connection's own ordered queue:

- `$connect` — after `joined`.
- `$disconnect` — when the socket ends, payload `{ "reason": "close" | "error" }`.
  It runs even if the client never said goodbye, and after every event that
  socket sent, so a late `move` can never resurrect a player who left.

Only a trigger that names them receives them (`trigger.ws --event $disconnect`);
a catch-all `trigger.ws` does not, and clients cannot send `$` events. Presence
is two small pipelines:

```
register pipelines/presence-in --
| trigger.ws --event $connect
| ws.sync_state --op merge --path "/players/{session_id}" --value "{{ { since: Date.now() } }}"

register pipelines/presence-out --
| trigger.ws --event $disconnect
| ws.sync_state --op delete --path "/players/{session_id}"
```

A path placeholder must resolve: `{session_id}` missing from the payload is the
error `FW_NODE_WS_PATH_SEGMENT_EMPTY`, never a write to `/players` itself. Numbers are
written as text. When the last connection leaves, the room and its state are
disposed; the next visitor starts from `{}`.

A connection is admitted to a room when the room has no `trigger.ws`, or one of
them is open, or it passes the auth of at least one of them; otherwise the socket
is closed with code 4401 before any state is sent.

---

## Templates Needed

- `pages/game-lobby.tsx` — room list + create room
- `pages/game-room.tsx` — game board + player list + WebSocket connection

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
