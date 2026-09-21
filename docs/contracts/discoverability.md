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

## 6. Installability — one project, several apps, nothing hidden

A phone keeping the site is the third reader after search and answer
engines. An installable app is project content served by ordinary routes,
never files the platform invents; a project may declare several (the public
site at `/`, a member app at `/member/`), each a separate icon.

| Piece | Where it lives | Rule |
|---|---|---|
| manifest | `pwa/manifest.webmanifest` in the repo; `trigger.webhook --path /manifest.webmanifest \| web.response --file pwa/manifest.webmanifest` | a plain JSON file the author writes; typed by its extension. A second app is a second file and route with its own `id`, `scope` (ending in `/`) and `start_url` inside it |
| worker | `pwa/site.sw.ts` in the repo; `trigger.webhook --path /sw.js \| web.response --file pwa/site.sw.ts` | TypeScript, compiled by the page engine; every script `--file` serves starts with `self.__ZF = { version, source }`; `text/javascript`, `Cache-Control: no-cache`. It must answer at the scope root: a store object at `/_files/sw.js` controls nothing. Served from deeper, add `--header Service-Worker-Allowed=/` |
| icons | repo files `static/pwa/*.png` → `/_static/pwa/…`: they ship with the code (the repo write API takes bytes with `?encoding=base64`) | 192, 512, maskable 512, apple 180 (no transparency) |
| head | `page.head`: `links: [{rel:"manifest"}, {rel:"apple-touch-icon"}]`, `themeColor` | the renderer already emits these; the shell's shared head links carry them once |
| page side | a project component: registers the worker, keeps `beforeinstallprompt` and shows a quiet card, iOS hint, dismiss remembered | never a modal on first visit; a page places it on purpose |
| offline page | an ordinary page pipeline, e.g. `/offline`, precached by the worker | the worker answers it on a failed navigation |
| verification | `route_fetch` → `pwa: { installable, reasons[], icons, worker }` | Chrome's checklist without a browser: name, `start_url` inside `scope`, display, 192 + 512 icons, a worker whose effective scope covers `start_url` (`serviceworker.src` in the manifest, else `sw.js` under the scope). Headless Chromium never fires `beforeinstallprompt`; real Chrome is the judge of the button |

There is no PWA node and no PWA setting: `web.response --file` is the only
platform piece, and it serves any project file (robots, sitemap, an icon)
the same way. The whole set — folder, routes, page, component — is what a
hub package of kind `folder_bundle` carries, so "add a PWA" is an install,
and a generator can later ask the questions and write the same files.

Rules: the worker never caches a signed-in scope (two people on one phone
must not see each other's back office); every `start_url` renders for a
guest; a scope is a directory and ends in `/`. Not covered here: push (a
sender node next to `n.mail.send`, a subscriptions table, VAPID as a
credential), background sync, store packaging.

## 6a. The error page a visitor sees

An uncaught failure never answers a browser with JSON. On a page request a
5xx renders the project's own error page — the `trigger.weberror --code 500`
archetype, registered like the 404 — or, when the project has none, the
platform's neutral fallback: unbranded, no Studio styling, one sentence and the
first eight characters of the run id as "reference". With `errors: shown`
(`addressing.md` §2a) the same page carries the failure's code, message,
node id and a link to the run. A JSON request gets the JSON forms from §2a.
`route_fetch` reports which of the two a route answered with.

What the catcher takes, and what it leaves alone:

| Situation | Who answers |
|---|---|
| no route matched | `weberror` 404, else the platform's neutral page |
| a page needed a sign-in it did not have | `weberror` 401, else the redirect to the project's login |
| a pipeline failed (uncaught) | `weberror` `500`/`5xx`/`*`, else the neutral page; JSON form for a JSON request |
| a pipeline answered `_status ≥ 400` in its payload (legacy convention) | `weberror` for that code, else JSON |
| a pipeline answered through `web.response --status ≥ 400` with a message, body or template | **that response, as authored, always** — the catcher never replaces what an author wrote |

A request is a *JSON request* when its `Accept` names `application/json` and not `text/html`; everything else is a page request.

What the page receives as `input`, from `trigger.weberror`: `error_code`,
`error_message` (the reason phrase), `original_path`, `method`,
`request_id` (the run id, full; the page prints the first eight), and — only
when the effective `errors` is `shown` — `detail: { code, message, node_id,
run_url }`. Under `hidden` `detail` is absent, not empty, so a page cannot
leak it by accident.

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
