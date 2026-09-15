# Discoverability

Status: **Implemented §1 (except the OG card) and §4** — 2026-09-14; §2, §3, §5 and the OG card proposed. How a project shows itself to the
machines that read it — search engines (SEO), answer engines (GEO: ChatGPT,
Claude, Perplexity, Gemini), and social previews — and how the agent that
built a page proves it is readable before calling it done.

## 0. The rule

**Every public page carries its facts as data, not only as prose, and the
platform writes the machine parts.** An author sets a handful of `head`
values from the page's own data; the renderer emits the tags, resolves the
URLs, and fills the defaults. Nothing about a site's address, sitemap or
robots is authored by hand (`addressing.md` §0).

## 1. What a page declares (`page.head`, resolved from `getPage(input)`)

| Field | Emits | Rule |
|---|---|---|
| `title`, `description`, `robots`, `canonical` | the matching tag | as today |
| `titleSuffix` | appended to `<title>` | set once in the shell's page config; `""` to opt out |
| `og { title, description, image, url, type, siteName, locale }`, `twitter { … }` | `og:*`, `twitter:*` | **defaults**: `og.title` ← `title`, `og.description` ← `description`, `og.url` ← `canonical` or the request URL, `og.type` ← `website`, `twitter.card` ← `summary_large_image` when an image exists; explicit values win |
| `og.image: { template, input }` | a 1200×630 PNG | the template renders to SVG through the RWE, the platform rasterises it (`resvg`), caches by hash under `data/cache/og/`, emits the absolute URL |
| `jsonld` | `<script type="application/ld+json">` | an object or an array of objects; serialised and escaped by the renderer; never a string |
| `alternates { en, id, x-default, … }` | `<link rel="alternate" hreflang>` | one per language; `html.lang` follows the served variant |
| `extra` | raw HTML | stays, as the escape hatch |

**Absolute URLs**: `canonical`, `og.url`, `og.image`, `twitter.image` and
`alternates` that start with `/` are prefixed with the request origin
(scheme + host, honouring `x-forwarded-*`), which the web layer passes into
`render()`. The project never stores a host.

## 2. What a page can be served as

| Request | Response |
|---|---|
| default | HTML, as today |
| `?format=md` or `Accept: text/markdown` on a `--template` route | the same page as Markdown: `<main>` only (the shell's landmarks are dropped), headings, paragraphs, lists, tables, links made absolute, images as their `alt`, followed by a facts block from `jsonld` |
| `web.response` with a content type picked from a list (JSON, HTML, Markdown, text, XML, CSV, iCalendar, RSS) or typed by hand | that content type (`--body` or a payload field) — design when built |

## 3. What the platform generates per project (the site surface — design when built)

`/sitemap.xml` (active public `GET` routes without parameters, minus
`noindex`, plus a `site.sitemap` function pipeline for dynamic slugs; split
past 1,000; `lastmod` only from real data) · `/robots.txt` (private routes
and switched-off surfaces disallowed; the answer-engine bots allowed or not
per bot from Settings → Addressing; the sitemap line) · `/llms.txt` (the
sitemap's pages with their descriptions) and `/llms-full.txt` (the same with
every page's Markdown twin), regenerated nightly.

## 4. What the builder gets back — build-time verification

`route_fetch` scans the HTML it fetched and adds to its report:

```json
"seo": { "title": "Jane Doe — RESEARCHSITE", "description_chars": 98, "canonical": "https://research.example/researchers/jane-doe",
         "h1_count": 1, "lang": "en", "robots": null,
         "og": { "title": true, "description": true, "image": "https://…/jane.webp", "absolute": true }, "twitter_card": "summary_large_image",
         "jsonld_types": ["Person", "BreadcrumbList"], "hreflang": ["en", "id", "x-default"] }
```

Nothing is added to the page. The agent compares numbers against the rules
in the `growth-rules` and `design-verify` skills (`h1_count == 1`,
`og.absolute == true`, a profile has `Person`) instead of reading HTML. The
same scan, run over the sitemap nightly, is the site-health table in §5.

## 5. What the operator sees — measurement

A per-project counter (not a log): every request whose `User-Agent` is a
known bot (Googlebot, bingbot, GPTBot, ClaudeBot, PerplexityBot,
Google-Extended, Applebot, CCBot, Bytespider, DuckDuckBot, …) increments
`hits[bot]` for the day, sets `last_seen`, and bumps that bot's top-50
paths; unknown agents are not recorded. One file per day under
`data/store/crawlers/`, kept 90 days. A panel shows: bot · hits · last seen
· top pages, the sitemap's and `llms-full.txt`'s last generation, and the
nightly site-health table from §4. What it deliberately is not: keyword
volumes, rankings, backlinks.

## 6. Installability — PWA as a mode (draft, build later)

A phone keeping the site is the third reader after search and answer
engines. It is one configuration per project, never hand-written files:

| Piece | Generated from | Rule |
|---|---|---|
| `/manifest.json` | project name, short name, the theme's `background`/`primary`, one square source icon | icons rasterised in every required size by the same SVG→PNG path as the OG card; `head.manifest`, `themeColor` and apple-touch icons filled from it by the renderer |
| `/sw.js` | the project's **mode** | **ssr**: network-first for pages, cache-first for `/_static`, an offline fallback page · **spa**: precache the shell and its assets, runtime cache for `/api/*` and `/_files`, client navigation as now · **static**: precache everything `web.static.generate` wrote; served by the site surface, versioned by the build hash |
| offline page | the **empty/404** archetype in `offline` mode | registered like the 404; shown by the worker when the network fails on a page |
| install prompt | a zeb/ui `InstallPrompt` (Chromium `beforeinstallprompt`, the iOS "Add to Home Screen" hint) | opt-in, usually in the shell; never a modal on first visit |
| verification | `route_fetch`'s report gains `pwa: { manifest, sw, icons, installable }` | the skills check it the way they check `seo` |

Settings: mode (off · ssr · spa · static), the icon, short name, display
(`standalone` / `browser`); everything else derived. Not covered: push
notifications, background sync, native store packaging.

## 7. Not covered

- Rankings, keywords, backlinks — external data; not the platform's.
- Paid social cards, AMP, Web Stories.
- Consent banners and third-party analytics — a project's choice, added as scripts.

## Order of work

§1 (`jsonld`, `alternates`, absolute URLs, defaults, `titleSuffix`) and §4
first — small, in the renderer and `route_fetch`, needed by the first
Researchsite profile page. §2 Markdown twin next. §1 OG card, then §3 and §5 once
a site exists to feed them; §6 after those, since it reuses the icon
rasteriser and the site surface.

## Evidence

- `src/rwe/core/render.rs` tests `head_urls_become_absolute_and_cards_are_defaulted`,
  `explicit_card_values_win_and_a_suffix_is_never_doubled`, `jsonld_and_alternates_are_emitted_and_escaped`,
  `without_a_host_relative_urls_are_left_alone`; `src/platform/services/ops.rs` `seo_facts_tests`.
- Renderer tests: a head with `jsonld` (object and array) emits one escaped
  block each; `alternates` emits one link per language plus `x-default`;
  relative `canonical`/`og.image` come out absolute for the passed origin;
  defaults fill `og:*` and `twitter:card` from title/description/image and
  yield to explicit values; `titleSuffix` applies once.
- `route_fetch` on a page with two `h1` reports `h1_count: 2`; on a page
  with a relative `og:image` reports `absolute: false`.
- `?format=md` on a rendered page returns `text/markdown` whose first line is
  the page's `h1`, with no header/footer text.
- Researchsite: every public page passes the §4 rules; `llms.txt` lists every
  public page; a shared article link shows a card with the generated image.
