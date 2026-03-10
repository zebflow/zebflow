# SekejapQL — SjTable Query & Write Guide

SjTable is the project-local document store (backed by SekejapDB). Each "table" is a named
collection with optional indexed fields.

## Query Format — SekejapQL text DSL

One operation per line, or pipe-separated on one line.

```
collection "sjtable__contacts"
where_eq "status" "active"
take 50
```

Pipe style:
```
collection "sjtable__contacts" | where_eq "status" "active" | take 50
```

**Collection name**: always `sjtable__` + table slug. Table named `contacts` → collection `sjtable__contacts`.

## All Operators

| Op | Syntax | Notes |
|---|---|---|
| `collection` | `collection "sjtable__name"` | Required starting op |
| `one` | `one "sjtable__name/row-id"` | Single row by slug |
| `where_eq` | `where_eq "field" "value"` | Exact match (O(1) if hash-indexed) |
| `where_gt` | `where_gt "field" 80` | Greater than |
| `where_lt` | `where_lt "field" 80` | Less than |
| `where_gte` | `where_gte "field" 80` | Greater or equal |
| `where_lte` | `where_lte "field" 80` | Less or equal |
| `where_between` | `where_between "field" 10 90` | Range inclusive |
| `where_in` | `where_in "field" "a" "b" "c"` | IN list |
| `sort` | `sort "field" desc` | asc (default) or desc |
| `skip` | `skip 20` | Pagination offset |
| `take` | `take 50` | Limit (default 100, max 500) |
| `select` | `select "name" "email"` | Return only these fields |
| `matching` | `matching "search terms"` | Full-text search (if fulltext_fields defined) |

## Querying via `run_db_query`

Pass SekejapQL text directly as the `sql` param. Both text DSL and JSON pipeline formats are accepted. Text DSL is preferred.

List all rows:
```
collection "sjtable__contacts"
take 100
```

Filter by field:
```
collection "sjtable__contacts"
where_eq "status" "active"
take 50
```

Range + sort:
```
collection "sjtable__contacts"
where_gte "score" 80
sort "score" desc
take 20
```

Multiple filters + projection:
```
collection "sjtable__orders"
where_eq "status" "pending"
where_gte "amount" 100
sort "created_at" desc
select "order_id" "customer" "amount" "status"
take 25
```

Get single row by id:
```
one "sjtable__contacts/alice-001"
```

Full-text search (requires `fulltext_fields` defined on table):
```
collection "sjtable__articles"
matching "quarterly report 2024"
take 10
```

## Creating a Table — DSL (preferred for agent)

```
create table contacts --title "Contacts" --fields "name:Text,email:Text,status:Text,score:Number,created_at:Number" --hash "email,status" --range "score,created_at"
```

- `--fields "f1:Kind,f2:Kind"` — comma-separated `name:Kind` pairs. Kinds: `Text`, `Number`, `Boolean`, `Json`
- `--hash "f1,f2"` — O(1) equality lookup fields
- `--range "f1,f2"` — O(log N) range query fields

After creation you can immediately query:
```
collection "sjtable__contacts" | take 50
```

List all tables:
```
get tables
```

## Creating a Table — HTTP API

`POST /api/projects/{owner}/{project}/tables`:

```json
{
  "table": "contacts",
  "title": "Contacts",
  "attributes": [
    {"name": "name", "kind": "Text"},
    {"name": "email", "kind": "Text"},
    {"name": "status", "kind": "Text"},
    {"name": "score", "kind": "Number"},
    {"name": "created_at", "kind": "Number"}
  ],
  "hash_indexed_fields": ["email", "status"],
  "range_indexed_fields": ["score", "created_at"]
}
```

## Inserting / Updating a Row — DSL (preferred for agent)

Use `run` with a `n.sjtable.query` node (operation=upsert):

```
run | trigger.manual | n.sjtable.query --table contacts --op upsert --row-id alice-001 -- {"name":"Alice","email":"alice@example.com","status":"active","score":95,"created_at":1741300000000}
```

The `row-id` is the row's unique key. Upserting the same id overwrites the row.

## Full Example: Create + Insert + Query

```
# 1. Create table
create table products --fields "name:Text,price:Number,category:Text,in_stock:Boolean" --hash "category" --range "price"

# 2. Insert rows
run | trigger.manual | n.sjtable.query --table products --op upsert --row-id prod-001 -- {"name":"Widget A","price":29.99,"category":"widgets","in_stock":true}
run | trigger.manual | n.sjtable.query --table products --op upsert --row-id prod-002 -- {"name":"Widget B","price":49.99,"category":"widgets","in_stock":false}

# 3. Query rows
collection "sjtable__products"
where_eq "category" "widgets"
where_gte "price" 30
sort "price" asc
take 20
```

## n.sjtable.query Pipeline Node Config

```
n.sjtable.query
  --table <name>                     Table slug (e.g. contacts)
  --op query|upsert                  Default: query
  --where-field <field>              (query) equality filter field
  --where-value-path <jsonpath>      (query) value from pipeline payload
  --limit <n>                        (query) max rows, default 100
  --row-id <id>                      (upsert) row unique key
  --row-id-path <jsonpath>           (upsert) row id from pipeline payload
  --data-path <jsonpath>             (upsert) row data from pipeline payload
  -- <json>                          (upsert) inline row data as JSON body
```

All flags have `_expr` variants for Deno expression evaluation:
`--table-expr 'ctx.input.table_name'`

## Platform Collections (metadata DB, read-only)

The platform catalog sekejap has these collections (NOT sjtable, use admin DB API):
- `user` — user accounts
- `project` — project records
- `project_credential` — credentials
- `project_db_connection` — DB connections
- `pipeline_meta` — pipeline registry
- `mcp_session` — MCP sessions
