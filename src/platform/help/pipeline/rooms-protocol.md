# Rooms wire protocol zebflow.rooms.v1

How a client — a page, a native game, a backend service in any language —
talks to a Zebflow room. This page is the whole protocol: a client that does
what it says needs nothing else. The server side of a room is `trigger.room`
and the `ws.*` nodes (`help("pipeline/nodes/trigger.room")`).

## Connecting

Open a WebSocket to the room, naming the protocol:

```
GET /_ws/rooms/{room}                         (on the project's own hosts)
GET /ws/{owner}/{project}/rooms/{room}        (on the platform)
Sec-WebSocket-Protocol: zebflow.rooms.v1
```

```js
const ws = new WebSocket(`wss://${location.host}/_ws/rooms/lobby`, "zebflow.rooms.v1");
```

- `?ticket=<ticket>` joins a `--private` room with a ticket from
  `ws.ticket.create`.
- The room's join rule (`--auth` on its `trigger.room`) reads the upgrade
  request's headers, as a webhook's does.
- **A request that names no protocol, or none this server speaks, is refused
  before the upgrade:** HTTP `426 Upgrade Required`, the supported protocols
  in the `Sec-WebSocket-Protocol` response header, and a JSON body:

```json
{ "ok": false, "error": "a room socket names its wire protocol in Sec-WebSocket-Protocol: zebflow.rooms.v1", "supported": ["zebflow.rooms.v1"] }
```

A protocol version is replaced, never kept beside its successor: a client
that names `zebflow.rooms.v1` against a server that speaks only a later
version is refused the same way, and learns which version to speak.

## What the client sends

One text frame per event, JSON:

```json
{ "event": "player.move", "payload": { "x": 3, "y": 7 } }
```

`event` is the name a `trigger.room --event` matches; names starting with
`$` are the server's own and are ignored. A frame that is not JSON is
ignored. A session with 256 events waiting to run is closed with `1008`.

## What the server sends

Text frames, JSON, each with a `type`.

### `joined` — first, once

```json
{ "type": "joined", "session_id": "ws-1f0c", "room": "match-7", "state": { "players": { "ws-9a2e": { "x": 0, "y": 0 } } } }
```

`state` is the session's whole view of the room's state. Keep it: every
later `patch` changes it.

### `patch` — what changed

```json
{ "type": "patch", "ops": [
    { "op": "set", "path": ["players", "ws-1f0c"], "value": { "x": 3, "y": 7 } },
    { "op": "set", "path": ["players", "ws-9a2e", "x"], "value": 1 },
    { "op": "del", "path": ["players", "ws-4b11"] } ] }
```

Only what changed since the last frame this session got. Apply the ops in
order:

- `set`: walk `path` (a list of object keys) from the root of the state,
  creating each missing object on the way, and put `value` at the last key;
- `del`: remove the last key of `path` from its object, if it is there.

A list is a single value: a changed list is `set` whole. An unchanged part
of the state sends nothing.

```js
function apply(state, ops) {
  for (const { op, path, value } of ops) {
    let at = state;
    for (const key of path.slice(0, -1)) at = at[key] ??= {};
    const last = path[path.length - 1];
    if (op === "set") at[last] = value; else delete at[last];
  }
}
```

### `patch` in a tick room — one frame per tick

A room with a `trigger.room --event $tick` sends what each tick changed and
the events it sent in one frame, after the tick:

```json
{ "type": "patch", "tick": 412, "ops": [ { "op": "set", "path": ["players", "ws-1f0c", "x"], "value": 4 } ],
  "events": [ { "event": "hit", "payload": { "by": "ws-9a2e", "damage": 3 } } ] }
```

Apply `ops` first, then handle `events` in order. `tick` counts the room's
ticks. A tick that changed nothing and sent nothing to this session sends
no frame.

### `event` — a message

```json
{ "type": "event", "event": "chat.message", "payload": { "from": "ws-9a2e", "text": "hi" } }
```

From `ws.message.send`, outside a tick. It does not change the state.

### `resync` — the whole view again

```json
{ "type": "resync", "state": { "players": { "ws-1f0c": { "x": 4, "y": 7 } } } }
```

The session fell more than 256 frames behind. Replace the state with
`state`; the next `patch` follows from it. Events it missed are lost: an
app that must not lose a message keeps it in state or in its database.

## Close codes

| Code | Why | Before `joined`? |
| --- | --- | --- |
| `4404` | the room is not open (or no `trigger.room` declares it) | yes |
| `4401` | the join rule or the ticket refused | yes |
| `4429` | the room is full (`--max-sessions`) | yes |
| `4408` | the login the join rule proved expired | no |
| `4409` | the server closed this session (`ws.session.close`) | no |
| `4410` | the room was closed (`ws.room.close`) | no |
| `1008` | too many events waiting (flooding) | no |

A client that closes on purpose sends a close frame; the room then raises
`$disconnect { reason: "left" }`. A connection that ends without one is a
drop (`dropped`).
