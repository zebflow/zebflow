# Real-Time Game (WebSocket State Sync)

## What this builds

A multiplayer game where all clients share synchronized state through WebSocket rooms. State is managed server-side and synced to all participants. Players join rooms, send moves/actions, state updates are broadcast.

---

## Pipelines

1. `GET /game` → render game lobby page
2. `GET /game/:room` → render game room with initial state
3. `POST /api/game/rooms` → create room with initial state
4. `WS event: player.join` → add player to room state → broadcast
5. `WS event: player.move` → validate move → update game state → broadcast
6. `WS event: player.leave` → remove player → broadcast

---

## DSL

### game-lobby — lobby page

```
| trigger.webhook --path /game --method GET
| sekejap.query --table game_rooms --op scan
| script -- "return { rooms: input.filter(r => r.status === 'waiting') }"
| web.render --template-path pages/game-lobby.tsx --route /game
```

### game-room — room with initial state

```
| trigger.webhook --path /game/:room --method GET
| sekejap.query --table game_rooms --op get --key "{{input.params.room}}"
| script -- "if (!input) return { __redirect: '/game' }; return { room: input }"
| web.render --template-path pages/game-room.tsx --route /game/:room
```

### api-room-create — create game room

```
| trigger.webhook --path /api/game/rooms --method POST
| script -- "const id = 'room-' + Math.random().toString(36).slice(2,8); return { id, name: input.body.name || id, status: 'waiting', players: {}, board: {}, created_at: Date.now() }"
| sekejap.query --table game_rooms --op upsert
| script -- "return { ok: true, room_id: input.id }"
```

### ws-player-join — player joins room

```
| trigger.ws --room "{{input.room_id}}" --event player.join
| script -- "return { path: '/players/' + input.payload.player_id, value: { id: input.payload.player_id, name: input.payload.name, score: 0, joined_at: Date.now() } }"
| ws.sync_state --op merge --path /players --value_path /value
| ws.emit --to all --event state.updated --payload_path /
```

### ws-player-move — player makes a move

```
| trigger.ws --room "{{input.room_id}}" --event player.move
| script -- "const { player_id, move } = input.payload; if (!player_id || !move) return null; return { player_id, move, ts: Date.now() }"
| ws.sync_state --op merge --path /last_move --value_path /
| ws.emit --to all --event player.moved --payload_path /
```

### ws-player-leave — player disconnects

```
| trigger.ws --room "{{input.room_id}}" --event player.leave
| script -- "return { player_id: input.payload.player_id }"
| ws.emit --to all --event player.left --payload_path /
```

---

## Nodes Used

- `trigger.webhook` — HTTP lobby and room pages
- `trigger.ws` — WebSocket event handlers (join, move, leave)
- `sekejap.query` — persist room state and history
- `script` — move validation, state transforms
- `web.render` — TSX templates
- `ws.sync_state` — merge patches into the server-side room state object
- `ws.emit` — broadcast events to all players in the room

---

## State Sync Protocol

Server-side room state is a JSON object. Clients receive:
```json
{ "type": "joined", "state": { "players": {}, "board": {}, "last_move": null } }
{ "type": "state_patch", "op": "merge", "path": "/players", "value": {...} }
{ "type": "event", "event": "player.moved", "payload": {...} }
```

---

## Templates Needed

- `pages/game-lobby.tsx` — room list + create room
- `pages/game-room.tsx` — game board + player list + WebSocket connection
