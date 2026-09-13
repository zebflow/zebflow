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

Sekejap has no `UPSERT`/`ON CONFLICT`. "Don't duplicate on re-run" means:
look each item up by its natural key first, then branch — `UPDATE` if it's
already there, `INSERT` if it isn't. All three scrapers follow the same
shape: fetch → parse into an array → `logic.foreach` → look up → branch.

### feed-scraper — fetch and parse JSON feed

```zf
register scraping/feed-scraper --
[trig] trigger.schedule --cron "*/30 * * * *"
[fetch] http.request --url "https://example.com/feed.json" --method GET
[parse] script -- "const items = (input.response.body.items || []).map(i => ({ id: i.guid || i.url, title: i.title, url: i.url, summary: (i.description || '').slice(0,500), published_at: new Date(i.pubDate).getTime(), source: 'example-feed', fetched_at: Date.now() })); return { items: items.filter(i => i.id && i.title) };"
[each] logic.foreach --items-expr "input.items"
[find] sekejap.query --params "{{ [$item.id] }}" -- "SELECT _key FROM scraped_items WHERE _key = $1"
[known] logic.if --expr "input.rows.length > 0"
[update] sekejap.query --read-only false --params "{{ [$item.title, $item.url, $item.summary, $item.published_at, $item.fetched_at, $item.id] }}" -- "UPDATE scraped_items SET title = $1, url = $2, summary = $3, published_at = $4, fetched_at = $5 WHERE _key = $6"
[insert] sekejap.query --read-only false --params "{{ [$item.id, $item.title, $item.url, $item.summary, $item.published_at, $item.source, $item.fetched_at] }}" -- "INSERT INTO scraped_items (_key, title, url, summary, published_at, source, fetched_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"

[trig] -> [fetch]
[fetch] -> [parse]
[parse] -> [each]
[each]:item -> [find]
[find] -> [known]
[known]:true -> [update]
[known]:false -> [insert]
```

### api-paginated-scraper — multi-page API fetch

```zf
register scraping/api-paginated-scraper --
[trig] trigger.schedule --cron "0 3 * * *"
[fetch] http.request --url "https://api.example.com/articles?page=1&per_page=100" --method GET
[parse] script -- "const items = (input.response.body.data || []).map(a => ({ id: String(a.id), title: a.title, author: (a.author && a.author.name) || null, category: a.category, url: a.url, body: (a.content || '').slice(0,2000), fetched_at: Date.now() })); return { items };"
[each] logic.foreach --items-expr "input.items"
[find] sekejap.query --params "{{ [$item.id] }}" -- "SELECT _key FROM articles WHERE _key = $1"
[known] logic.if --expr "input.rows.length > 0"
[update] sekejap.query --read-only false --params "{{ [$item.title, $item.author, $item.category, $item.url, $item.body, $item.fetched_at, $item.id] }}" -- "UPDATE articles SET title = $1, author = $2, category = $3, url = $4, body = $5, fetched_at = $6 WHERE _key = $7"
[insert] sekejap.query --read-only false --params "{{ [$item.id, $item.title, $item.author, $item.category, $item.url, $item.body, $item.fetched_at] }}" -- "INSERT INTO articles (_key, title, author, category, url, body, fetched_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"

[trig] -> [fetch]
[fetch] -> [parse]
[parse] -> [each]
[each]:item -> [find]
[find] -> [known]
[known]:true -> [update]
[known]:false -> [insert]
```

Paginate by wiring another `trigger.schedule`/`http.request` per page, or
loop pages inside `[parse]` with an outer `http.request` per iteration — there
is no built-in "fetch all pages" node.

### html-scraper — fetch HTML and extract with script

```zf
register scraping/html-scraper --
[trig] trigger.schedule --cron "0 * * * *"
[fetch] http.request --url "https://example.com/prices" --method GET --response-type text
[parse] script -- "const html = input.response.body; const matches = [...html.matchAll(/<div class=\"product\"[^>]*>([\s\S]*?)<\/div>/g)]; const items = matches.map((m,i) => { const nameMatch = m[1].match(/<h3>([^<]+)<\/h3>/); const priceMatch = m[1].match(/\$([0-9.]+)/); return { id: 'product-' + i, name: nameMatch ? nameMatch[1] : null, price: priceMatch ? parseFloat(priceMatch[1]) : null }; }); return { items: items.filter(p => p.name && p.price !== null).map(p => ({ ...p, fetched_at: Date.now() })) };"
[each] logic.foreach --items-expr "input.items"
[find] sekejap.query --params "{{ [$item.id] }}" -- "SELECT _key FROM product_prices WHERE _key = $1"
[known] logic.if --expr "input.rows.length > 0"
[update] sekejap.query --read-only false --params "{{ [$item.name, $item.price, $item.fetched_at, $item.id] }}" -- "UPDATE product_prices SET name = $1, price = $2, fetched_at = $3 WHERE _key = $4"
[insert] sekejap.query --read-only false --params "{{ [$item.id, $item.name, $item.price, $item.fetched_at] }}" -- "INSERT INTO product_prices (_key, name, price, fetched_at) VALUES ($1, $2, $3, $4)"

[trig] -> [fetch]
[fetch] -> [parse]
[parse] -> [each]
[each]:item -> [find]
[find] -> [known]
[known]:true -> [update]
[known]:false -> [insert]
```

`--response-type text` keeps the body a raw string instead of trying to parse
HTML as JSON.

### scraped-items-list — browse page

```
| trigger.webhook --path /data/items --method GET
| script -- "return { limit: Math.min(parseInt((input.query && input.query.limit) || '50', 10) || 50, 200) }"
| sekejap.query -- "SELECT * FROM scraped_items ORDER BY fetched_at DESC LIMIT {{ input.limit }}"
| script -- "return { items: input.rows, count: input.rows.length }"
| web.response --template pages/scraped-items.tsx
```

### scraped-item-detail — single item

```zf
register scraping/scraped-item-detail --
[a] trigger.webhook --path /data/items/:id --method GET
[b] sekejap.query --params "{{ [input.params.id] }}" -- "SELECT * FROM scraped_items WHERE _key = $1"
[c] logic.if --expr "input.rows.length > 0"
[d] script -- "return { item: input.rows[0] };"
[e] web.response --template pages/scraped-item-detail.tsx
[f] web.response --location /data/items

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
- `http.request` — outbound HTTP to fetch external pages/APIs
- `script` — HTML/JSON parsing, normalization
- `logic.foreach` — one lookup-and-branch run per parsed item
- `sekejap.query` — `SELECT` to check existence, `UPDATE`/`INSERT` to write; no `--table`/`--op`
- `web.response` — display scraped data

---

## Tips

**Dedup without upsert:** each scraper looks its item up by natural key
(`_key`) before writing — `UPDATE` on a hit, `INSERT` on a miss. Re-running
the scraper never creates a duplicate row as long as the key stays stable.

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
> happens next. Branch with `logic.if` and let `web.response` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
