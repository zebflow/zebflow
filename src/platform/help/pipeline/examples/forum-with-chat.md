# Forum with Real-Time Chat

## What this builds

A forum with threaded discussion rooms. Each room has a WebSocket connection
for live chat. Messages persist in Sekejap. Public read, open write (add
`--auth-type jwt` to the webhook and WS triggers below to require sign-in).

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
| trigger.webhook --path /forum --method GET
| sekejap.query -- "SELECT * FROM forum_rooms ORDER BY last_activity DESC"
| script -- "return { rooms: input.rows }"
| web.response --template pages/forum-home.tsx
```

### forum-room — room view with recent messages

Two queries, so the room lookup is carried forward by graph id instead of
being overwritten by the messages query:

```zf
register forum/room --
[a] trigger.webhook --path /forum/:room --method GET
[room] sekejap.query --params "{{ [input.params.room] }}" -- "SELECT * FROM forum_rooms WHERE _key = $1"
[msgs] sekejap.query --params "{{ [input.params.room] }}" -- "SELECT * FROM forum_messages WHERE room = $1 ORDER BY ts DESC LIMIT 50"
[merge] script -- "return { room: $nodes.room.rows[0] || null, messages: input.rows.slice().reverse() };"
[b] web.response --template pages/forum-room.tsx

[a] -> [room]
[room] -> [msgs]
[msgs] -> [merge]
[merge] -> [b]
```

### api-room-create — create a new room

```zf
register forum/api-room-create --
[trig] trigger.webhook --path /api/forum/rooms --method POST
[has_name] logic.if --expr "!!(input.body && input.body.name)"
[bad] web.response --status 400 --body "{{ { ok: false, error: 'name required' } }}"
[draft] script -- "const id = String(input.body.name).toLowerCase().replace(/[^a-z0-9]+/g,'-'); return { id, name: input.body.name };"
[ins] sekejap.query --read-only false --params "{{ [$nodes.draft.id, $nodes.draft.name, Date.now(), Date.now()] }}" -- "INSERT INTO forum_rooms (_key, name, created_at, last_activity) VALUES ($1, $2, $3, $4)"
[ok] script -- "return { ok: true, id: $nodes.draft.id };"

[trig] -> [has_name]
[has_name]:false -> [bad]
[has_name]:true -> [draft]
[draft] -> [ins]
[ins] -> [ok]
```

### ws-chat-message — WebSocket chat handler

`trigger.ws --room` is a literal filter, never an expression, so it is left
off here and every room's traffic reaches this one pipeline; `input.room_id`
(set by the trigger) says which room a given event came from.

```zf
register forum/ws-chat-message --
[a] trigger.ws --event chat.message
[guard] logic.if --expr "!!(input.payload && input.payload.user && input.payload.text)"
[save] script -- "return { id: Date.now().toString(), room: input.room_id, user: input.payload.user, text: input.payload.text, ts: Date.now() };"
[ins] sekejap.query --read-only false --params "{{ [$nodes.save.id, $nodes.save.room, $nodes.save.user, $nodes.save.text, $nodes.save.ts] }}" -- "INSERT INTO forum_messages (_key, room, user, text, ts) VALUES ($1, $2, $3, $4, $5)"
[emit] ws.emit --to all --event chat.message --payload "{{ $nodes.save }}"

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
- `trigger.ws --event chat.message` — WebSocket event handler; `--room` omitted (it is a literal filter, not per-connection routing)
- `sekejap.query` — rooms and messages storage; plain `SELECT`/`INSERT`, no `--table`/`--op`
- `logic.if` — validate before saving
- `script` — shape rows, carry the room lookup forward via `$nodes`
- `web.response` — TSX templates
- `ws.emit --payload "{{ expr }}"` — broadcast message to all room participants

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
> happens next. Branch with `logic.if` and let `web.response` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
