# Web Scraping + Data Pipeline

## What this builds

Scheduled pipelines that fetch external web pages or APIs, parse/extract data with JavaScript, deduplicate, and upsert into Sekejap. Includes a display page to browse scraped data.

---

## Pipelines

1. `CRON every 30 min` → fetch RSS/JSON feed → parse → upsert items
2. `CRON daily` → fetch paginated API → normalize → deduplicate → store
3. `CRON every hour` → fetch HTML page → extract with script → store
4. `GET /data/items` → list scraped items → render page
5. `GET /data/items/:id` → single item detail → render page

---

## DSL

### feed-scraper — fetch and parse JSON feed

```
| trigger.schedule --cron "*/30 * * * *"
| http.request --url "https://example.com/feed.json" --method GET
| script -- "const items = input.body.items || []; return items.map(i => ({ id: i.guid || i.url, title: i.title, url: i.url, summary: i.description?.slice(0,500), published_at: new Date(i.pubDate).getTime(), source: 'example-feed', fetched_at: Date.now() }))"
| script -- "return input.filter(i => i.id && i.title)"
| sekejap.query --table scraped_items --op upsert
```

### api-paginated-scraper — multi-page API fetch

```
| trigger.schedule --cron "0 3 * * *"
| http.request --url "https://api.example.com/articles?page=1&per_page=100" --method GET
| script -- "const items = input.body.data || []; return { items: items.map(a => ({ id: a.id.toString(), title: a.title, author: a.author?.name, category: a.category, url: a.url, body: a.content?.slice(0,2000), fetched_at: Date.now() })), total: input.body.total }"
| sekejap.query --table articles --op upsert
| script -- "return { stored: input.items?.length || 0, total: input.total }"
```

### html-scraper — fetch HTML and extract with script

```
| trigger.schedule --cron "0 * * * *"
| http.request --url "https://example.com/prices" --method GET
| script -- "const html = input.body; const matches = [...html.matchAll(/<div class=\"product\"[^>]*>([\s\S]*?)<\/div>/g)]; return matches.map((m,i) => { const name = m[1].match(/<h3>([^<]+)<\/h3>/)?.[1] || 'unknown'; const price = m[1].match(/\$([0-9.]+)/)?.[1]; return { id: 'product-' + i, name, price: price ? parseFloat(price) : null, fetched_at: Date.now() } })"
| script -- "return input.filter(p => p.name && p.price !== null)"
| sekejap.query --table product_prices --op upsert
```

### scraped-items-list — browse page

```
| trigger.webhook --path /data/items --method GET
| sekejap.query --table scraped_items --op scan
| script -- "const items = input.sort((a,b)=>b.fetched_at-a.fetched_at).slice(0, parseInt(input.query?.limit || 50, 10)); return { items, count: items.length }"
| web.render --template-path pages/scraped-items.tsx --route /data/items
```

### scraped-item-detail — single item

```
| trigger.webhook --path /data/items/:id --method GET
| sekejap.query --table scraped_items --op get --key "{{input.params.id}}"
| script -- "if (!input) return { __redirect: '/data/items' }; return { item: input }"
| web.render --template-path pages/scraped-item-detail.tsx --route /data/items/:id
```

---

## Nodes Used

- `trigger.schedule` — cron-based scheduling
- `trigger.webhook` — browse/view endpoints
- `http.request` — outbound HTTP to fetch external pages/APIs
- `script` — HTML parsing with regex, data normalization, deduplication, filtering
- `sekejap.query` — store scraped data (upsert = idempotent by id, scan = list all)
- `web.render` — display scraped data

---

## Tips

**Deduplication:** `sekejap.query --op upsert` is idempotent by key — running the scraper multiple times won't duplicate records if `id` stays the same.

**Rate Limiting:** For sites that throttle, split into smaller cron windows or add delay via script:
```
| script -- "await new Promise(r => setTimeout(r, 1000)); return input"
```

**Error Handling:** Wrap HTTP response parsing in try/catch in script nodes. Return `null` to skip the upsert node.

---

## Templates Needed

- `pages/scraped-items.tsx` — paginated item listing with search
- `pages/scraped-item-detail.tsx` — full item display
