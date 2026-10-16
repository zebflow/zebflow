# Real-Time Game (WebSocket State Sync)

## What this builds

A multiplayer game where all clients share synchronized state through
WebSocket rooms. State is managed server-side and synced to all participants.
Each game room is a **tick room**: players send moves whenever they like, and
twenty times a second one run resolves every move of that interval, in the
order they arrived, and every client gets one result.

---

## Pipelines

1. `GET /game` → render game lobby page
2. `GET /game/:room` → render game room with initial state
3. `POST /api/game/rooms` → record the room and open it (`ws.room.open`)
4. `$connect` / `$disconnect` → seat or remove the player in room state
5. `$tick` every 50 ms → apply every move of the interval → state goes out once

---

## Tables

```sql
CREATE TABLE game_rooms (_key TEXT PRIMARY KEY, name TEXT, status TEXT, created_at INTEGER)
```

Room *state* (players, board, last move) lives in the WebSocket room's shared
state object (`ws.state.put` / `ws.state.update` / `ws.state.delete`), not in this table — the table only tracks
which rooms were made and whether they're joinable. The live room itself
exists only because `api-room-create` opened it: a visitor naming a room
nobody opened is refused (close code `4404`).

---

## DSL

### game-lobby — lobby page

```
| trigger.webhook --route /game --method GET
| sekejap.query.run -- "SELECT * FROM game_rooms WHERE status = 'waiting'"
| javascript.script.run -- "return { rooms: input.query.rows }"
| web.response.send --template pages/game-lobby.tsx
```

### game-room — room with initial state

```zf
register game/room --
[a] trigger.webhook --route /game/:room --method GET
[b] sekejap.query.run --param "1={{ input.webhook.params.room }}" -- "SELECT * FROM game_rooms WHERE _key = $1"
[c] logic.if --when "input.query.rows.length > 0"
[d] javascript.script.run -- "return { room: input.query.rows[0] };"
[e] web.response.send --template pages/game-room.tsx
[f] web.response.send --status 302 --header "Location=/game"

[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[d] -> [e]
[c]:false -> [f]
```

### api-room-create — create and open a game room

`ws.room.open` opens the room under the `room-*` pattern the WebSocket
handlers below declare; `--max-sessions` caps it (a join past it closes with
`4429`) and the room closes by itself after sitting empty for `--idle`.

```zf
register game/api-room-create --
[trig] trigger.webhook --route /api/game/rooms --method POST
[draft] javascript.script.run -- "const id = 'room-' + Math.random().toString(36).slice(2,8); return { id, name: (input.webhook.body && input.webhook.body.name) || id };"
[ins] sekejap.query.run --write --param "1={{ $nodes.draft.script.id }}" --param "2={{ $nodes.draft.script.name }}" --param "3={{ Date.now() }}" -- "INSERT INTO game_rooms (_key, name, status, created_at) VALUES ($1, $2, 'waiting', $3)"
[open] ws.room.open --room "{{ $nodes.draft.script.id }}" --max-sessions 8 --idle 30m
[ok] javascript.script.run -- "return { ok: true, room_id: $nodes.draft.script.id };"
[resp] web.response.send --body "{{ $nodes.ok.script }}"

[trig] -> [draft]
[draft] -> [ins]
[ins] -> [open]
[open] -> [ok]
[ok] -> [resp]
```

A private room (`--private`) admits only players holding a ticket from
`ws.ticket.create --room <room> --user <sub>`, issued by a webhook that checks
the player belongs there; the client connects with `?ticket=…`.

### ws-tick — every move of the interval, once per tick

`--event $tick --every 50ms` makes every room `room-*` matches a tick room.
A client event there no longer runs a trigger of its own — `player.move`,
chat, anything — it waits for the next tick. The tick pipeline gets them all,
in arrival order, as `input.room.inputs` (`{ session, event, payload, auth }`
each), with `input.room.tick` (its number) and `input.room.dt_ms` (the time
since the last one). A session sending more than `--max-inputs` (default 30)
in one interval has the rest dropped and counted (`inputs_dropped` in
`ws.room.list`).

```
register game/ws-tick --
[tick] trigger.room --room "room-*" --event $tick --every 50ms --max-inputs 10
[moves] javascript.script.run -- "const moved = {}; let count = 0; for (const i of input.room.inputs) { if (i.event === 'player.move' && i.payload) { moved[i.session] = { x: i.payload.x, y: i.payload.y, tick: input.room.tick }; count += 1; } } return { moved, count };"
[any] logic.if --when "input.script.count > 0"
[save] ws.state.update --key /players --value "{{ input.script.moved }}"

[tick] -> [moves]
[moves] -> [any]
[any]:true -> [save]
```

- **One thing at a time per room.** A room runs its tick, its `$connect` and
  its `$disconnect` one after another, never two at once, so a pipeline that
  reads a value and writes it back needs no lock.
- **Ticks never overlap.** A tick that takes longer than `--every` delays the
  next one and counts a `tick_overrun`. An empty tick (no input) still runs:
  timers and respawns live in state.
- **State goes out once per tick**, after the tick pipeline ends, however many
  writes the tick made.
- Every `$tick` trigger that can match a room declares the same `--every` and
  `--max-inputs`, or activation refuses.

The client sends `{ "event": "player.move", "payload": { "x": 3, "y": 7 } }`
whenever the player moves, and draws `state.players` from each `state_patch`.

---

## Nodes Used

- `trigger.webhook` — HTTP lobby and room pages
- `trigger.room --room "room-*" --event $tick --every 50ms` — the game's tick: every move of the interval in one run; `--event $connect` / `$disconnect` seat and remove players. `--room` is required: an exact name is a fixed room, a pattern (`room-*`, `room-:id`) declares opened rooms
- `ws.room.open` — opens a game room under that pattern; `ws.room.close` ends it (every player gets `4410`)
- `sekejap.query.run` — track which rooms exist; plain `SELECT`/`INSERT`, no `--table`/`--op`
- `javascript.script.run` — shape the lobby rows and new room ids; its answer sits under `script` (`$nodes.<id>.script.<field>`), the rest of the payload is kept
- `logic.if` — skip the state write on a tick nobody moved in
- `ws.state.update` / `ws.state.put` / `ws.state.delete --key "/players/{{ expr }}"` — change the server-side room state
- `ws.message.send --body "{{ expr }}"` — send an event to the players in the room (from any pipeline, for example a webhook)

---

## State Sync Protocol

Server-side room state is a JSON object. Clients receive:

```json
{ "type": "joined", "session_id": "ws-1f0c", "room": "room-abc123", "state": { "players": {} } }
{ "type": "state_patch", "state": { "players": { "ws-1f0c": { "x": 3, "y": 7, "tick": 412 } } } }
{ "type": "event", "event": "match.over", "payload": { "winner": "ws-1f0c" } }
{ "type": "resync", "state": { "players": { "ws-1f0c": { "x": 3, "y": 7, "tick": 412 } } } }
```

`state_patch` and `resync` always carry the **full** current state, not a diff —
the client replaces its local copy wholesale. `resync` arrives when the
connection fell behind and messages were dropped.

`ws.message.send --recipient session` / `--recipient others` is enforced by the server: a session-only
event reaches that one socket and no other.

## Joining, leaving and who is online

The server raises two reserved events on the room's one queue, in order with
everything else the room handles:

- `$connect` — after `joined`.
- `$disconnect` — when the socket ends, payload `{ "reason": … }`: `left`
  (the client closed), `dropped` (the connection broke), `closed`
  (`ws.session.close`), `room_closed` (`ws.room.close`), `expired` (the
  login's `exp` passed) or `flood` (too many events waiting).
  It runs even if the client never said goodbye, and after every event that
  socket sent — in a tick room, right after the tick that takes that socket's
  last inputs — so a late `move` can never resurrect a player who left.

Only a trigger that names them receives them (`trigger.room --room "room-*" --event $disconnect`);
a catch-all event trigger does not, and clients cannot send `$` events. Presence
is two small pipelines:

```
register pipelines/presence-in --
| trigger.room --room "room-*" --event $connect
| ws.state.update --key "/players/{{ input.room.session_id }}" --value "{{ { since: Date.now() } }}"

register pipelines/presence-out --
| trigger.room --room "room-*" --event $disconnect
| ws.state.delete --key "/players/{{ input.room.session_id }}"
```

Every segment of `--key` must say something: `{{ input.room.session_id }}` resolving
empty leaves `/players/`, which is refused (`FW_NODE_WS_STATE_UPDATE_KEY`,
`FW_NODE_WS_STATE_DELETE_KEY`), never a write to `/players` itself. An
opened room keeps its state when the last player leaves, until it is closed
or sits empty past its `--idle`; a fixed room (an exact `--room`) forgets its
state with its last session.

A connection joins a room only when an active `trigger.room` declares it and
the room is open (`4404` otherwise), the room's join rule — the `--auth` /
`--credential` / `--role` every trigger on that room declares alike — admits
it (`4401`), and a seat is free (`4429`). The rule is checked once, at
connect; what it proved rides on every event as `input.room.auth`, and the
session closes with `4408` when the login expires.

---

## Templates Needed

- `pages/game-lobby.tsx` — room list + create room
- `pages/game-room.tsx` — game board + player list + WebSocket connection

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
