# Forum with Real-Time Chat

## What this builds

A forum with threaded discussion rooms. Each room has a WebSocket connection for live chat. Messages are persisted in Sekejap. JWT auth for posting. Public read, auth-gated write.

---

## Pipelines

1. `GET /forum` → list rooms → render listing
2. `GET /forum/:room` → fetch room + recent messages → render room page
3. `POST /api/forum/rooms` → create room → return JSON
4. `POST /api/forum/:room/messages` → auth → save message → return JSON
5. `WS /ws/{owner}/{project}/rooms/:room` → WebSocket room (built-in route, no pipeline needed)
6. `WS event: chat.message` → auth check → save message → broadcast to room

---

## DSL

### forum-list — public room listing

```
| trigger.webhook --path /forum --method GET
| sekejap.query --table forum_rooms --op scan
| script -- "return { rooms: input.sort((a,b)=>b.last_activity-a.last_activity) }"
| web.render --template-path pages/forum-home.tsx --route /forum
```

### forum-room — room view with recent messages

```
| trigger.webhook --path /forum/:room --method GET
| sekejap.query --table forum_rooms --op get --key "{{input.params.room}}"
| script -- "return { room: input }"
| sekejap.query --table forum_messages --op scan
| script -- "return { ...input, messages: input.messages?.filter(m => m.room === input.room?.id).slice(-50) || [] }"
| web.render --template-path pages/forum-room.tsx --route /forum/:room
```

### api-room-create — create a new room

```
| trigger.webhook --path /api/forum/rooms --method POST
| script -- "const b = input.body; if (!b.name) return { error: 'name required', __status: 400 }; return { id: b.name.toLowerCase().replace(/[^a-z0-9]+/g,'-'), name: b.name, created_at: Date.now(), last_activity: Date.now() }"
| sekejap.query --table forum_rooms --op upsert
| script -- "return { ok: true, id: input.id }"
```

### ws-chat-message — WebSocket chat handler

```
| trigger.ws --room "{{input.room_id}}" --event chat.message
| script -- "if (!input.payload.user) return null; return { id: Date.now().toString(), room: input.room_id, user: input.payload.user, text: input.payload.text, ts: Date.now() }"
| sekejap.query --table forum_messages --op upsert
| ws.emit --to all --event chat.message --payload_path /
```

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints
- `trigger.ws` — WebSocket event handler
- `sekejap.query` — rooms and messages storage (scan, get, upsert)
- `script` — validation, transforms, auth checks
- `web.render` — TSX templates
- `ws.emit` — broadcast message to all room participants

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
