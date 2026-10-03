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
state object (`ws.state.put` / `ws.state.update` / `ws.state.delete`), not in this table — the table only tracks
which rooms exist and whether they're joinable.

---

## DSL

### game-lobby — lobby page

```
| trigger.webhook --path /game --method GET
| sekejap.query.run -- "SELECT * FROM game_rooms WHERE status = 'waiting'"
| script.result.run -- "return { rooms: input.query.rows }"
| web.response.send --template pages/game-lobby.tsx
```

### game-room — room with initial state

```zf
register game/room --
[a] trigger.webhook --path /game/:room --method GET
[b] sekejap.query.run --param "1={{ input.params.room }}" -- "SELECT * FROM game_rooms WHERE _key = $1"
[c] logic.if --expr "input.query.rows.length > 0"
[d] script.result.run -- "return { room: input.query.rows[0] };"
[e] web.response.send --template pages/game-room.tsx
[f] web.response.send --location /game

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
[draft] script.result.run -- "const id = 'room-' + Math.random().toString(36).slice(2,8); return { id, name: (input.body && input.body.name) || id };"
[ins] sekejap.query.run --write --param "1={{ $nodes.draft.id }}" --param "2={{ $nodes.draft.name }}" --param "3={{ Date.now() }}" -- "INSERT INTO game_rooms (_key, name, status, created_at) VALUES ($1, $2, 'waiting', $3)"
[ok] script.result.run -- "return { ok: true, room_id: $nodes.draft.id };"

[trig] -> [draft]
[draft] -> [ins]
[ins] -> [ok]
```

### ws-player-join — player joins room

A part of `--key` that comes from the payload is written with `{{ }}`; the
`ws.*` nodes keep the payload, so `input.payload` and the trigger's room are
still there for the next node:

```zf
register game/ws-player-join --
[a] trigger.room --event player.join
[merge] ws.state.update --key "/players/{{ input.payload.player_id }}" --value "{{ { id: input.payload.player_id, name: input.payload.name, score: 0, joined_at: Date.now() } }}"
[emit] ws.message.send --event state.updated --body "{{ { player_id: input.payload.player_id } }}"

[a] -> [merge]
[merge] -> [emit]
```

### ws-player-move — player makes a move

```zf
register game/ws-player-move --
[a] trigger.room --event player.move
[guard] logic.if --expr "!!(input.payload && input.payload.player_id && input.payload.move)"
[set] ws.state.put --key /last_move --value "{{ { player_id: input.payload.player_id, move: input.payload.move, ts: Date.now() } }}"
[emit] ws.message.send --event player.moved --body "{{ { player_id: input.payload.player_id, move: input.payload.move } }}"

[a] -> [guard]
[guard]:true -> [set]
[set] -> [emit]
```

An invalid move just stops at `[guard]` — there is no `false` edge, and a
`script` returning `null` would not have stopped the pipeline anyway; only a
`logic.if` branch does.

### ws-player-leave — player disconnects

```
| trigger.room --event player.leave
| ws.message.send --event player.left --body "{{ { player_id: input.payload.player_id } }}"
```

---

## Nodes Used

- `trigger.webhook` — HTTP lobby and room pages
- `trigger.room --event <name>` — WebSocket event handlers (join, move, leave); `--room` omitted, it is a literal filter, not per-connection routing
- `sekejap.query.run` — track which rooms exist; plain `SELECT`/`INSERT`, no `--table`/`--op`
- `script` — shape the lobby rows and new room ids
- `logic.if` — move validation
- `ws.state.update` / `ws.state.put` / `ws.state.delete --key "/players/{{ expr }}"` — change the server-side room state
- `ws.message.send --body "{{ expr }}"` — broadcast events to all players in the room

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

`ws.message.send --recipient session` / `--recipient others` is enforced by the server: a session-only
event reaches that one socket and no other.

## Joining, leaving and who is online

The server raises two reserved events on every connection's own ordered queue:

- `$connect` — after `joined`.
- `$disconnect` — when the socket ends, payload `{ "reason": "close" | "error" }`.
  It runs even if the client never said goodbye, and after every event that
  socket sent, so a late `move` can never resurrect a player who left.

Only a trigger that names them receives them (`trigger.room --event $disconnect`);
a catch-all `trigger.room` does not, and clients cannot send `$` events. Presence
is two small pipelines:

```
register pipelines/presence-in --
| trigger.room --event $connect
| ws.state.update --key "/players/{{ input.session_id }}" --value "{{ { since: Date.now() } }}"

register pipelines/presence-out --
| trigger.room --event $disconnect
| ws.state.delete --key "/players/{{ input.session_id }}"
```

Every segment of `--key` must say something: `{{ input.session_id }}` resolving
empty leaves `/players/`, which is refused (`FW_NODE_WS_STATE_UPDATE_KEY`,
`FW_NODE_WS_STATE_DELETE_KEY`), never a write to `/players` itself. When the last connection leaves, the room and its state are
disposed; the next visitor starts from `{}`.

A connection is admitted to a room when the room has no `trigger.room`, or one of
them is open, or it passes the auth of at least one of them; otherwise the socket
is closed with code 4401 before any state is sent.

---

## Templates Needed

- `pages/game-lobby.tsx` — room list + create room
- `pages/game-room.tsx` — game board + player list + WebSocket connection

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
