# SekejapQL Pipeline Query Format

SekejapQL is a JSON pipeline query language. A query is a JSON object with a `pipeline` array of stage operations.

## Format

```json
{
  "pipeline": [
    {"op": "collection", "name": "my_collection"},
    {"op": "where_eq", "field": "status", "value": "active"},
    {"op": "take", "n": 100}
  ]
}
```

## Operators

### `collection`
Select all documents from a named collection.
```json
{"op": "collection", "name": "user"}
```

### `where_eq`
Filter documents where a field equals a value (hash index).
```json
{"op": "where_eq", "field": "owner", "value": "alice"}
```

### `where_range`
Filter documents where a field is in a range (range index).
```json
{"op": "where_range", "field": "created_at", "from": 1700000000, "to": 1800000000}
```

### `take`
Limit output to N documents.
```json
{"op": "take", "n": 50}
```

### `skip`
Skip N documents (for pagination).
```json
{"op": "skip", "n": 10}
```

## Mutation Format

Mutations use a different format:
```json
{"mutation": "put_json", "data": {"_id": "user/alice", "_collection": "user", "owner": "alice", ...}}
{"mutation": "delete_node", "id": "user/alice"}
```

## Platform Collections

The platform metadata DB has these collections:
- `user` — user accounts
- `project` — project records
- `project_credential` — encrypted credentials
- `project_db_connection` — DB connections
- `pipeline_meta` — pipeline catalog entries
- `project_policy` — RBAC policies
- `project_policy_binding` — RBAC bindings
- `mcp_session` — persisted MCP sessions

## Admin DB API

```
GET  /api/admin/db/collections          — list collections with counts
POST /api/admin/db/query                — run raw SekejapQL pipeline
GET  /api/admin/db/node/{slug}          — get single node by slug
DELETE /api/admin/db/node/{slug}        — delete single node by slug
```
