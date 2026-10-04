# Forum with Real-Time Chat

## What this builds

A forum with threaded discussion rooms. Each room has a WebSocket connection
for live chat. Messages persist in Sekejap. Public read, open write (add
`--auth jwt` to the webhook and WS triggers below to require sign-in).

---

## Pipelines

1. `GET /forum` → list rooms → render listing
2. `GET /forum/:room` → fetch room + recent messages → render room page
3. `POST /api/forum/rooms` → create room → return JSON
4. `WS /ws/{owner}/{project}/rooms/:room` → WebSocket room (built-in route, no pipeline needed)
5. `WS event: chat.message` → validate → save message → broadcast to room

---

## Tables

```sql
CREATE TABLE forum_rooms (_key TEXT PRIMARY KEY, name TEXT, created_at INTEGER, last_activity INTEGER)
CREATE TABLE forum_messages (_key TEXT PRIMARY KEY, room TEXT, user TEXT, text TEXT, ts INTEGER)
```

---

## DSL

### forum-list — public room listing

```
| trigger.webhook --route /forum --method GET
| sekejap.query.run -- "SELECT * FROM forum_rooms ORDER BY last_activity DESC"
| javascript.script.run -- "return { rooms: input.query.rows }"
| web.response.send --template pages/forum-home.tsx
```

### forum-room — room view with recent messages

Two queries, so the room lookup is carried forward by graph id instead of
being overwritten by the messages query:

```zf
register forum/room --
[a] trigger.webhook --route /forum/:room --method GET
[room] sekejap.query.run --param "1={{ input.webhook.params.room }}" -- "SELECT * FROM forum_rooms WHERE _key = $1"
[msgs] sekejap.query.run --param "1={{ $trigger.params.room }}" -- "SELECT * FROM forum_messages WHERE room = $1 ORDER BY ts DESC LIMIT 50"
[merge] javascript.script.run -- "return { room: $nodes.room.query.rows[0] || null, messages: input.query.rows.slice().reverse() };"
[b] web.response.send --template pages/forum-room.tsx

[a] -> [room]
[room] -> [msgs]
[msgs] -> [merge]
[merge] -> [b]
```

### api-room-create — create a new room

```zf
register forum/api-room-create --
[trig] trigger.webhook --route /api/forum/rooms --method POST
[has_name] logic.if --when "!!(input.webhook.body && input.webhook.body.name)"
[bad] web.response.send --status 400 --body "{{ { ok: false, error: 'name required' } }}"
[draft] javascript.script.run -- "const id = String($trigger.body.name).toLowerCase().replace(/[^a-z0-9]+/g,'-'); return { id, name: $trigger.body.name };"
[ins] sekejap.query.run --write --param "1={{ $nodes.draft.script.id }}" --param "2={{ $nodes.draft.script.name }}" --param "3={{ Date.now() }}" --param "4={{ Date.now() }}" -- "INSERT INTO forum_rooms (_key, name, created_at, last_activity) VALUES ($1, $2, $3, $4)"
[ok] javascript.script.run -- "return { ok: true, id: $nodes.draft.script.id };"
[resp] web.response.send --body "{{ $nodes.ok.script }}"

[trig] -> [has_name]
[has_name]:false -> [bad]
[has_name]:true -> [draft]
[draft] -> [ins]
[ins] -> [ok]
[ok] -> [resp]
```

### ws-chat-message — WebSocket chat handler

`trigger.room --room` is a literal filter, never an expression, so it is left
off here and every room's traffic reaches this one pipeline; the trigger
answers under `room` — `input.room.room_id` right after the trigger, or
`$trigger.room_id` anywhere later — says which room a given event came from.

```zf
register forum/ws-chat-message --
[a] trigger.room --event chat.message
[guard] logic.if --when "!!(input.room.payload && input.room.payload.user && input.room.payload.text)"
[save] javascript.script.run -- "return { id: Date.now().toString(), room: $trigger.room_id, user: $trigger.payload.user, text: $trigger.payload.text, ts: Date.now() };"
[ins] sekejap.query.run --write --param "1={{ $nodes.save.script.id }}" --param "2={{ $nodes.save.script.room }}" --param "3={{ $nodes.save.script.user }}" --param "4={{ $nodes.save.script.text }}" --param "5={{ $nodes.save.script.ts }}" -- "INSERT INTO forum_messages (_key, room, user, text, ts) VALUES ($1, $2, $3, $4, $5)"
[emit] ws.message.send --room "{{ $nodes.save.script.room }}" --event chat.message --body "{{ $nodes.save.script }}"

[a] -> [guard]
[guard]:true -> [save]
[save] -> [ins]
[ins] -> [emit]
```

A malformed event just stops at `[guard]` — the `false` pin has nothing wired
to it, and returning `null` from a script would not have stopped anything
(only branching does).

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints
- `trigger.room --event chat.message` — WebSocket event handler; `--room` omitted (it is a literal filter, not per-connection routing)
- `sekejap.query.run` — rooms and messages storage; plain `SELECT`/`INSERT`, no `--table`/`--op`
- `logic.if` — validate before saving
- `javascript.script.run` — shape rows, carry the room lookup forward via `$nodes`; its answer sits under `script` (`$nodes.<id>.script.<field>`), the rest of the payload is kept
- `web.response.send` — TSX templates
- `ws.message.send --room "{{ expr }}" --body "{{ expr }}"` — broadcast message to all room participants (the room and body are read at `$nodes.save.script`, since the script no longer replaces the payload)

---

## WebSocket Client Setup (in TSX template)

```tsx
const ws = new WebSocket(`/ws/${owner}/${project}/rooms/${roomId}`);
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.type === 'event' && msg.event === 'chat.message') {
    setMessages(prev => [...prev, msg.payload]);
  }
};
// Send a message:
ws.send(JSON.stringify({ event: 'chat.message', payload: { user, text } }));
```

---

## Templates Needed

- `pages/forum-home.tsx` — room listing
- `pages/forum-room.tsx` — chat interface with WebSocket

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
