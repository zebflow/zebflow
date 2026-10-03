# Web Scraping + Data Pipeline

## What this builds

Scheduled pipelines that fetch external pages or APIs, parse/extract data
with JavaScript, and write into Sekejap without duplicating rows on re-run.
Includes a display page to browse the scraped data.

---

## Pipelines

1. `CRON every 30 min` → fetch JSON feed → parse → store items
2. `CRON daily` → fetch paginated API → normalize → store
3. `CRON every hour` → fetch HTML page → extract with script → store
4. `GET /data/items` → list scraped items → render page
5. `GET /data/items/:id` → single item detail → render page

---

## Tables

```sql
CREATE TABLE scraped_items (_key TEXT PRIMARY KEY, title TEXT, url TEXT, summary TEXT, published_at INTEGER, source TEXT, fetched_at INTEGER)
CREATE TABLE articles (_key TEXT PRIMARY KEY, title TEXT, author TEXT, category TEXT, url TEXT, body TEXT, fetched_at INTEGER)
CREATE TABLE product_prices (_key TEXT PRIMARY KEY, name TEXT, price REAL, fetched_at INTEGER)
```

---

## DSL

"Don't duplicate on re-run" means the item's natural key is the row's `_key`,
and each write is `INSERT … ON CONFLICT (_key) DO UPDATE`: a new key inserts,
a known one updates in place. All three scrapers follow the same shape:
fetch → parse into an array → `logic.foreach` → upsert.

### feed-scraper — fetch and parse JSON feed

```zf
register scraping/feed-scraper --
[trig] trigger.schedule --cron "*/30 * * * *"
[fetch] http.response.fetch --url "https://example.com/feed.json" --method GET
[parse] script.result.run -- "const items = (input.response.body.items || []).map(i => ({ id: i.guid || i.url, title: i.title, url: i.url, summary: (i.description || '').slice(0,500), published_at: new Date(i.pubDate).getTime(), source: 'example-feed', fetched_at: Date.now() })); return { items: items.filter(i => i.id && i.title) };"
[each] logic.foreach --items-expr "input.items"
[save] sekejap.query.run --write --param "1={{ $item.id }}" --param "2={{ $item.title }}" --param "3={{ $item.url }}" --param "4={{ $item.summary }}" --param "5={{ $item.published_at }}" --param "6={{ $item.source }}" --param "7={{ $item.fetched_at }}" -- "INSERT INTO scraped_items (_key, title, url, summary, published_at, source, fetched_at) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (_key) DO UPDATE SET title = EXCLUDED.title, url = EXCLUDED.url, summary = EXCLUDED.summary, published_at = EXCLUDED.published_at, source = EXCLUDED.source, fetched_at = EXCLUDED.fetched_at"

[trig] -> [fetch]
[fetch] -> [parse]
[parse] -> [each]
[each]:item -> [save]
```

### api-paginated-scraper — multi-page API fetch

```zf
register scraping/api-paginated-scraper --
[trig] trigger.schedule --cron "0 3 * * *"
[fetch] http.response.fetch --url "https://api.example.com/articles?page=1&per_page=100" --method GET
[parse] script.result.run -- "const items = (input.response.body.data || []).map(a => ({ id: String(a.id), title: a.title, author: (a.author && a.author.name) || null, category: a.category, url: a.url, body: (a.content || '').slice(0,2000), fetched_at: Date.now() })); return { items };"
[each] logic.foreach --items-expr "input.items"
[save] sekejap.query.run --write --param "1={{ $item.id }}" --param "2={{ $item.title }}" --param "3={{ $item.author }}" --param "4={{ $item.category }}" --param "5={{ $item.url }}" --param "6={{ $item.body }}" --param "7={{ $item.fetched_at }}" -- "INSERT INTO articles (_key, title, author, category, url, body, fetched_at) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (_key) DO UPDATE SET title = EXCLUDED.title, author = EXCLUDED.author, category = EXCLUDED.category, url = EXCLUDED.url, body = EXCLUDED.body, fetched_at = EXCLUDED.fetched_at"

[trig] -> [fetch]
[fetch] -> [parse]
[parse] -> [each]
[each]:item -> [save]
```

Paginate by wiring another `trigger.schedule`/`http.response.fetch` per page, or
loop pages inside `[parse]` with an outer `http.response.fetch` per iteration — there
is no built-in "fetch all pages" node.

### html-scraper — fetch HTML and extract with script

```zf
register scraping/html-scraper --
[trig] trigger.schedule --cron "0 * * * *"
[fetch] http.response.fetch --url "https://example.com/prices" --method GET --response-type text
[parse] script.result.run -- "const html = input.response.body; const matches = [...html.matchAll(/<div class=\"product\"[^>]*>([\s\S]*?)<\/div>/g)]; const items = matches.map((m,i) => { const nameMatch = m[1].match(/<h3>([^<]+)<\/h3>/); const priceMatch = m[1].match(/\$([0-9.]+)/); return { id: 'product-' + i, name: nameMatch ? nameMatch[1] : null, price: priceMatch ? parseFloat(priceMatch[1]) : null }; }); return { items: items.filter(p => p.name && p.price !== null).map(p => ({ ...p, fetched_at: Date.now() })) };"
[each] logic.foreach --items-expr "input.items"
[save] sekejap.query.run --write --param "1={{ $item.id }}" --param "2={{ $item.name }}" --param "3={{ $item.price }}" --param "4={{ $item.fetched_at }}" -- "INSERT INTO product_prices (_key, name, price, fetched_at) VALUES ($1, $2, $3, $4) ON CONFLICT (_key) DO UPDATE SET name = EXCLUDED.name, price = EXCLUDED.price, fetched_at = EXCLUDED.fetched_at"

[trig] -> [fetch]
[fetch] -> [parse]
[parse] -> [each]
[each]:item -> [save]
```

`--response-type text` keeps the body a raw string instead of trying to parse
HTML as JSON.

### scraped-items-list — browse page

```
| trigger.webhook --route /data/items --method GET
| script.result.run -- "return { limit: Math.min(parseInt((input.webhook.query && input.webhook.query.limit) || '50', 10) || 50, 200) }"
| sekejap.query.run -- "SELECT * FROM scraped_items ORDER BY fetched_at DESC LIMIT {{ input.limit }}"
| script.result.run -- "return { items: input.query.rows, count: input.query.rows.length }"
| web.response.send --template pages/scraped-items.tsx
```

### scraped-item-detail — single item

```zf
register scraping/scraped-item-detail --
[a] trigger.webhook --route /data/items/:id --method GET
[b] sekejap.query.run --param "1={{ input.webhook.params.id }}" -- "SELECT * FROM scraped_items WHERE _key = $1"
[c] logic.if --expr "input.query.rows.length > 0"
[d] script.result.run -- "return { item: input.query.rows[0] };"
[e] web.response.send --template pages/scraped-item-detail.tsx
[f] web.response.send --status 302 --header "Location=/data/items"

[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[d] -> [e]
[c]:false -> [f]
```

---

## Nodes Used

- `trigger.schedule` — cron-based scheduling
- `trigger.webhook` — browse/view endpoints
- `http.response.fetch` — outbound HTTP to fetch external pages/APIs
- `script` — HTML/JSON parsing, normalization
- `logic.foreach` — one upsert per parsed item
- `sekejap.query.run` — `INSERT … ON CONFLICT (_key) DO UPDATE` to write; no `--table`/`--op`
- `web.response.send` — display scraped data

---

## Tips

**Dedup by key:** each scraper writes its item under its natural key
(`_key`) with `ON CONFLICT (_key) DO UPDATE`. Re-running the scraper never
creates a duplicate row as long as the key stays stable. `ON CONFLICT` takes
`_key` only; a second unique column is a `UNIQUE` constraint, not a conflict
target.

**Rate limiting:** there is no in-pipeline delay node (`setTimeout` is blocked
in the script sandbox). Space requests out with narrower cron windows, or
fetch a smaller page size per run.

**Error handling:** wrap HTTP response parsing in try/catch inside the
`script` node. A script that throws stops the pipeline with that message in
the trace — it does not silently skip the next node the way returning `null`
would look like it does. To skip bad rows without stopping the whole run,
filter them out of the array before `logic.foreach` runs.

---

## Templates Needed

- `pages/scraped-items.tsx` — paginated item listing with search
- `pages/scraped-item-detail.tsx` — full item display

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
