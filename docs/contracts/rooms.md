# Rooms

Status: **decided** — 2026-10-16, by the owner. Served: room kinds, tickets,
the join rule, the management nodes, events and ticks, the fast lane, views,
`--hold`, the `zebflow.rooms.v1` protocol and the `zeb/rooms` library
(ledger: `zebflow › ws-room-lifecycle`).

A room is a live channel between a project's server and the clients joined to
it: chat, a classroom, a dashboard, a match, an MMORPG zone. **Every room
exists because the server made it, and the server can see and close every
room.** No visitor can create a room by naming one. It aims to match a
dedicated game room server on one machine, down to a small board on a LAN.

| | |
| --- | --- |
| Address | the `ws` surface ([Addressing](./addressing.md) §2): `/_ws/rooms/{room}` on the project's hosts, `/ws/{o}/{p}/rooms/{room}` on the platform; **off** by default |
| Handled by | `trigger.room` pipelines — per event, or once per tick; one at a time per room |
| Answered with | `ws.message.send`, `ws.state.put` / `update` / `delete`, `ws.session.tag` |
| Managed with | `ws.room.open`, `ws.ticket.create`, `ws.room.close`, `ws.room.list`, `ws.session.close` |
| Client | any WebSocket speaking `zebflow.rooms.v1`; pages use the blessed `zeb/rooms` library (§ Clients and upgrades) |

## Two kinds of room

| | Fixed room | Opened room |
| --- | --- | --- |
| Declared by | `trigger.room --room lobby` (an exact name) | `trigger.room --room "match-:id"` or `"zone-*"` (a pattern) |
| Exists | always, while that trigger is active | from `ws.room.open` until `ws.room.close`, or until it has been empty for its `--idle` time |
| Joining one that does not exist | — | refused, `4404` |

- `--room` is required on `trigger.room`; there is no "any room".
- `ws.room.open` is refused unless an active `trigger.room` pattern matches the
  name, and refused for a fixed room's name.
- A fixed room keeps its state while it has sessions; an opened room keeps it
  until it closes.

## Opening a room and letting people in

```
| trigger.webhook --route /api/matches --method POST --auth jwt
| …create the match record, answering `match`…
| ws.room.open --room "match-{{ input.match.id }}" --private --max-sessions 10
               --meta "{{ { mode: 'ranked', map: 'valley' } }}" --hold 30s
| web.response.send --body "{{ input.room }}"

| trigger.webhook --route /api/matches/:id/ticket --auth jwt
| …check the caller belongs in match :id…
| ws.ticket.create --room "match-{{ $trigger.params.id }}" --user "{{ $trigger.auth.sub }}"
→ { "room": "match-…", "ticket": "…", "expires_at": "…" }
```

The client connects to `/_ws/rooms/match-…?ticket=…`.

- **A ticket** is a pass for one user into one room: signed by the platform,
  usable once, valid for `--ttl` (default 60 s, at most 10 min). The webhook
  that issues it is where the app checks its own data; the room queries nothing.
- `--private` rooms admit only ticket holders; other rooms admit whoever passes
  the join rule. Tickets come only from `ws.ticket.create`.
- `--max-sessions` caps the room: a join past it is refused with `4429`, and
  `ws.ticket.create` refuses a full room.
- `--meta` is a small JSON object shown by `ws.room.list` — what matchmaking
  filters on (mode, map, level band).

## The join rule

- **One rule per room, checked once, when the socket connects:** the `--auth` /
  `--credential` / `--role` of the room's `trigger.room` pipelines, read as on
  `trigger.webhook`. Every active `trigger.room` that can match the same room
  declares the same rule, or activation refuses, naming both pipelines.
- A ticket's user must be the identity the rule admitted; under `--auth none`
  the ticket's user becomes the session's identity.
- **The identity travels with the session:** every event carries
  `input.room.auth` as proved at join; events are not re-authenticated.
- A refusal closes with `4401` before anything of the room is sent; an expired
  login closes with `4408`.

## Events and ticks

A room runs in one of two modes, chosen by its triggers.

| | Event room | Tick room |
| --- | --- | --- |
| When | no `trigger.room --event $tick` matches it | one does: `trigger.room --room "match-:id" --event $tick --every 50ms` |
| A client event | runs its `--event` trigger | waits for the next tick — every client event, chat included |
| The tick pipeline gets | — | `input.room: { room_id, event: "$tick", tick, dt_ms, inputs: [ { session, event, payload, auth } … ] }` — every waiting input, in arrival order |
| `--state /pos` (any trigger) | adds `input.room.state` | the same: the state at the named paths as the run starts |
| State goes out | on each change, or each 33 ms with `--batch` | once per tick, after the tick pipeline ends |

- **A room handles one thing at a time, in arrival order across all its
  sessions** — events, ticks and `$` events alike. Two bids, two votes or two
  moves never interleave, so a pipeline that reads state and writes it back
  needs no lock. A slow step (a model call, a heavy query) holds the room, so
  it belongs in a webhook that answers later with `ws.message.send`.
- **A pipeline reads the room's state through its trigger:** `--state /pos`
  (repeatable, at most 16; `/` is the whole state) adds `input.room.state`,
  each path where it sits in the state (`{ pos: { … } }`, a path holding
  nothing left out), read under one lock just before the run, after every
  run before it. Only the named paths are copied; a trigger without
  `--state` copies nothing. Paths are fixed at activation (`{{ }}` refused).
- `--every` is 16 ms to 1 s. A room never runs two ticks at once: a tick that
  outlasts `--every` delays the next and counts a `tick_overrun`.
- `--every` is required with `$tick`; it and `--max-inputs` are refused on any
  other event. Every `$tick` trigger that can match one room declares the same
  `--every` and `--max-inputs`, or activation refuses; matching ones each run
  once per tick, in path order.
- A room ticks while it has sessions. `tick` counts the room's ticks; `dt_ms`
  is the time since the last tick started, `--every` on the first.
- In a tick room, `$connect` and `$disconnect` still run their own triggers,
  and every state change goes out after the tick, whatever made it.
- A `$disconnect` runs after every event its socket sent: in a tick room,
  right after the tick that takes that session's last inputs, the last
  session's included.
- A room that stops being a tick room runs its waiting inputs as events.
- A tick is how fast fights work: ten players' attacks in one interval are
  resolved in one run, in one order, and every client gets one result.
- Per session, at most `--max-inputs` (default 30) inputs wait per tick; more
  are dropped and counted. More than 256 events waiting to run closes the
  session with `1008`.
- `$connect`, `$disconnect`, `$away` and `$back` are raised by the server only;
  a client cannot send an event starting with `$`.

## Views: each client sees only its part

- **The server never sends a client what it may not see.** Hidden state is not
  hidden by the client; its bytes never leave the server.
- `ws.session.tag --session <id> --tag team:red` (repeatable, usually on
  `$connect`) gives a session tags. `ws.state.put --visible-to team:red`
  limits that path to sessions with the tag: fog of war, private hands, a
  team's chat.
- `--untag <tag>` (repeatable) removes a tag; a value ending in `*` removes
  every tag with that prefix, and `*` alone removes all. In one call every
  `--untag` applies before every `--tag`, and the node answers
  `session: { session_id, tags }`.
- **Nearby only** is the same rule: each tick moves every session to the grid
  cells around it (`--untag "cell:*" --tag cell:12:7 …`, its other tags
  untouched) and puts each entity `--visible-to` its cell, so an MMORPG zone
  sends each player what is near them, not the zone.
- Each distinct view is encoded once per flush and the same bytes go to every
  session holding it.

## Sending

- `ws.message.send --room` takes an exact name or a pattern. An exact name that
  is not open is an error, `FW_NODE_WS_ROOM_NOT_OPEN`; a pattern
  (`"zone-*"`) reaches every open room it matches and answers
  `message: { sent, rooms, event }` (`rooms: 0` is not an error).
- `--recipient` also takes `tag:<tag>` (sessions holding that tag) and
  `user:<sub>` (every session of that identity, on every device): one fixed
  `notifications` room reaches any signed-in user.
- `ws.state.*` takes an exact room only.

## Leaving, dropping and closing

| What happens | Sessions receive | Raised |
| --- | --- | --- |
| client leaves on purpose | — | `$disconnect` `{ reason: "left" }` |
| connection drops, room has `--hold 30s` | seat kept; `joined` gave a `resume` token | `$away`; then `$back` on resume, or `$disconnect` `{ reason: "timeout" }` |
| connection drops, no `--hold` | — | `$disconnect` `{ reason: "dropped" }` |
| `ws.session.close --room <room> --session <id>` or `--user <sub>` | `4409` | `$disconnect` `{ reason: "closed" }` |
| `ws.room.close --room <name or pattern>` (state dropped) | `4410` | `$disconnect` `{ reason: "room_closed" }` |
| login expires | `4408` | `$disconnect` `{ reason: "expired" }` |
| flooding | `1008` | `$disconnect` `{ reason: "flood" }` |

- A resume token is used once, works only for that session during its hold,
  and the `joined` of the resumed socket carries a new one; the session keeps
  its id, tags and seat.
- Closing a fixed room empties it; it exists again, empty, at the next join
  while its trigger is active.
- Kicking someone is a webhook the app guards with its own auth, ending in
  `ws.session.close` — never an event a client sends.

## Listing

`ws.room.list --room <pattern>` (default `*`) answers this project's open rooms
under `room`:

```json
"room": { "rooms": [ {
    "room": "zone-forest-3", "kind": "opened", "private": false, "mode": "tick",
    "meta": { "level": "20-30" }, "sessions": 412, "max_sessions": 500,
    "opened_at": "…", "opened_by": "pipelines/game/open-zone", "last_activity_at": "…",
    "state_bytes": 18432,
    "counters": { "messages_in": 1840221, "messages_out": 75600410, "ticks": 98112,
                  "tick_overruns": 4, "inputs_dropped": 0, "pipeline_runs": 99003,
                  "pipeline_errors": 12, "resyncs": 3, "flood_closes": 1 }
  } ],
  "total": { "rooms": 1, "sessions": 412 } }
```

- Counters only rise, from the moment the room opened; a rate is the
  difference between two readings.
- `--members` adds `members: [ { session, sub, tags, away } ]`; without it no
  identities are listed.
- Studio shows the same list per project and closes a room;
  `GET /api/projects/{o}/{p}/ws/rooms`, `POST …/ws/rooms/{room}/close` and the
  project MCP's `ws_rooms_list` / `ws_room_close` are the same operations.

## The fast lane

Rooms run apart from the rest of the server, so a slow query in a webhook never
makes a game stutter.

- Room tasks — sockets, ticks, encoding — run on their own worker threads,
  separate from the threads serving HTTP and other pipelines.
- The pipelines that match a room are resolved when it opens and when a
  pipeline is activated or deactivated, never per message.
- Runs started by a room write no run record; they count into the room's
  counters, and a failed run is recorded at most once a minute per pipeline.
- Memory is bounded: per-session queues, `--max-sessions`, and a per-room state
  cap (`--max-state`, default 1 MB) past which a state write is refused.

## Coverage

Every common use is these nodes and flags; a change to rooms keeps every row.

| Use | Room · mode | How |
| --- | --- | --- |
| group chat, DM, support | opened `chat-:id` · event | open + ticket per member; `--event chat.send` → `sql.insert` → `ws.message.send` |
| history on join | any · event | `$connect` → `sql.query` → `ws.message.send --recipient session` |
| typing, presence | any · event | `--recipient others`; `ws.state.update` on `$connect` / `$disconnect` |
| notifications | fixed `notifications` · event | any pipeline: `ws.message.send --recipient user:<sub>` |
| live dashboard | fixed `dashboard` · event | `trigger.schedule` → `ws.state.put` |
| collaborative board, cursors | opened `board-:id` · event | `ws.state.update --batch` |
| quiz, voting, auction | opened `quiz-:id` · event | one event at a time per room: read state, decide, write |
| card game, chess | opened `table-:id` · event | `--private --max-sessions 4`; `ws.session.tag --tag seat:2`; hand `--visible-to seat:2` |
| `.io` game, MOBA | opened `match-:id` · tick | `--max-sessions 10 --hold 30s`; `$tick` resolves inputs; fog `--visible-to team:red` |
| MMORPG zone | opened `zone-:id` · tick | `--max-sessions 500`; tick tags cells, entities `--visible-to cell:…`; new zone = new ticket |
| matchmaking | — | webhook: `ws.room.list` → free seat by `meta` → `ws.ticket.create`, or `ws.room.open` |
| devices | fixed `devices` · event | `--auth api_key`; `--event reading` in; `--recipient user:<device>` out |
| no-login LAN | fixed `lobby` + opened `match-:id` | `--auth none` lobby; its `--event start` opens matches |
| kick, ban | any | guarded webhook → `ws.session.close --user <sub>` |

## Clients and upgrades

Rooms are built to get faster without any app changing. Three layers, each
hidden from the one above:

| Layer | Apps see | Free to change |
| --- | --- | --- |
| Pipelines | room names, events, ticks, tags, state as JSON values | how the server runs, schedules, stores and encodes rooms |
| Client library | `zeb/rooms` — join, send, `on(event)`, `state`, reconnect | the frames on the wire |
| Wire protocol | `zebflow.rooms.v1`, named in `Sec-WebSocket-Protocol` | the next version (binary, compression) |

- **`zeb/rooms`** is a blessed RWE library ([RweLibraryManifest](./kinds/rwe-library-manifest/README.md)),
  served by the same binary as the room server, so a page's client and the
  server always speak the same version. It resumes a held seat on its own. The
  same module works in a page outside Zebflow (a game built with any engine) by
  URL.
  - **Where rooms live:** the server that renders a page writes that request's
    room base into it — the `ws` route of the host it was served on, or the
    platform form for a page under `/wh/{o}/{p}/`. With `ws` off there is none,
    and a join closes at once as `rooms-off`. A client outside a page passes
    `url`.
  - **Tickets:** `ticket` is a URL (POSTed with the page's cookies; the answer's
    `ticket` is used), a function returning one, or absent for a room that is
    not `--private`. It is fetched for every fresh join, never for a resume.
  - **Never retried:** the server's close codes (`4401`, `4404`, `4408`,
    `4409`, `4410`, `4429`, `1008`), `leave()`, `rooms-off` and a failed
    ticket — except a `4401` refusing a resume: the seat is gone, and one fresh
    join follows. Any other close is a drop: `away`, then a resume with
    backoff. `send` is refused unless joined and is never queued.
- **`zebflow.rooms.v1`:** a client sends `{ event, payload }`. The server sends
  `joined { session_id, room, state, resume? }` and `resync { state }` with the
  session's whole view, and after that only changes —
  `patch { tick?, ops: [ { op: "set" | "del", path: [...], value? } ] }` — plus
  `event { event, payload }`; in a tick room, one frame per tick carries both:
  `patch { tick, ops, events }`. A session that nothing changed for is sent
  nothing.
- **Each protocol version is published** as a help page beside the nodes', so a
  client in another language (a native game, a backend service) can speak it.
  A connection that names no protocol, or one this server does not speak, is
  refused before the upgrade with HTTP `426` and the supported list in
  `Sec-WebSocket-Protocol`; a version is replaced, not kept beside its
  successor.
- Pipelines never see frames, ticks' timing or encodings; a node's flags and
  answers change only through the node conventions.

## Limits

- A session that falls more than 256 frames behind receives `resync` with its
  current view; the `event` messages it missed are lost. An app that must not
  lose a message keeps it in state or in its database.
- Rooms live in the memory of the one process that serves the project; the
  platform form proxies to the office that holds it. One project served by
  several processes at once is not supported.
- Game logic runs in pipelines. Accelerators come later and change none of the
  above: compiled logic inside the tick (wasm), a built-in nearby grid, binary
  frames.
