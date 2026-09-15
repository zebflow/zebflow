---
name: growth-rules
description: Making a Zebflow site findable and effective once it works — per-page metadata, one call to action, form follow-through, sitemap and robots pipelines, a designed 404, performance and accessibility musts, the events worth counting. Use after a page's route works and before calling it done; a checklist a pipeline and a page can satisfy, not marketing advice.
license: MIT
metadata:
  version: "1"
---

# Growth rules: the site works — now let it be found and used

Everything here is mechanical. None of it needs taste; all of it is skipped
when nobody checks. Facts: `help(topic="web")` for `page.head`,
`help(topic="pipeline/web")` for responses, `ui-decide` for archetypes.

## 1. Every page: metadata

In the page file, `export const page = { head: { … } }`, or `getPage(input)`
when it comes from data:

| Field | Rule |
|---|---|
| `title` | ≤ 60 chars, the page's subject first, the site name last: `"Book an appointment — Northside Physio"`; the home page is `"<Site> — <what it is in 5 words>"` |
| `description` | 120–155 chars, one sentence a searcher would want to click; not the title again |
| `canonical` | the page's own URL without query string; lists: the first page only |
| `og.title`, `og.description`, `og.image`, `og.type` | title and description as above; image 1200×630 (a real photo or a generated card, never blank); `article` for articles, `website` otherwise |
| `robots` | `noindex` on auth, settings, admin, thank-you, search-result and paginated pages ≥ 2; nothing on the rest |
| `jsonld` | an object (or array) from the page's data: `Person` on a profile, `Article`/`NewsArticle` on an article, `Event` on an event, `Organization` on home/about, `BreadcrumbList` on anything below the root — never a string, the renderer serialises it |
| `alternates` | when the page exists in more than one language: `{ en, id, "x-default" }` |
| `titleSuffix` | in the shell's page config once (`" — <Site>"`), not per page |

`canonical`, `og.image` and `alternates` are written as `/path`; the
renderer makes them absolute for the host the request came in on. `og.title`,
`og.description`, `og.url`, `og.type` and `twitter.card` default from
title, description, canonical and the image — set them only to differ.

A page with no `head` is a defect, not a default. Three profiles cover
every route:

| Profile | Which routes | Set |
|---|---|---|
| **PUBLIC** | landing, list (curated), detail (public things), article, pricing, public forms | unique title + description, `canonical`, full `og`, no `robots` |
| **NOINDEX** | auth, settings, dashboard, thank-you, search results, private records, paginated ≥ 2 | safe generic title, `robots: "noindex,follow"`, no canonical or og |
| **MISSING** | the 404 page | `"Page not found — <Brand>"`, `robots: "noindex,follow"`, real HTTP 404 |

Title patterns: landing `<Offer> in <Region> — <Brand>`; detail `<Thing> — <Brand>`;
article `<Title> — <Brand>`; pricing `Plans and pricing — <Brand>`; auth `Sign in — <Brand>`.
`noindex` must stay crawlable — never also block it in `robots.txt`.

## 2. Every page: one call to action

- The primary action from `docs/brief.md` is reachable from every page — it
  lives in `PageShell`'s header, so pages do not each remember it.
- Above the fold on landing and pricing; in the title row on lists; under
  the header on details. Once per screen; a second button is the secondary
  action or nothing.
- Button text is the verb of the action ("Book appointment", "Get a
  quote"), never "Submit", "Click here", "Learn more".

## 3. Every form: follow through

1. `POST` validates, writes, then **redirects** (`web.response --location
   /thank-you --status 303`) — never renders the success on the POST URL
   (refresh would resubmit).
2. The thank-you page says what happens next and when ("We reply within one
   working day"), offers the next step, and is `robots: noindex`.
3. Errors come back on the form with the input preserved and the field
   named (`ui-decide` form recipe); status `400`.
4. Spam: a honeypot field (`<input name="website" class="hidden">`, refuse
   when filled) before any captcha.
5. The submission is stored before any mail is sent; mail failure must not
   lose the lead.
6. A direct visit to the thank-you URL proves nothing: show neutral guidance
   ("Looking for the form? It is here") rather than a false confirmation,
   and never count it as a conversion.

| The form does… | Success lands on | Proof required before the redirect |
|---|---|---|
| enquiry, quote, interest | `<form-route>/thank-you` | the row exists |
| booking or payment | a dedicated confirmation route | the server's or provider's result — never `?success=true` |
| sign in / recovery | the intended route / a generic confirmation | the verified auth result; no open redirect targets |
| settings | stays, inline "Saved." | persistence succeeded |
| search / filter | stays on the list, URL updated | not a conversion |

## 4. The three pipelines every site has

```
| trigger.webhook --path /sitemap.xml --method GET
| sekejap.query -- "SELECT slug, updated_at FROM posts WHERE status = 'published'"
| script -- "const h = ctx.trigger.headers; const base = (h['x-forwarded-proto'] || 'http') + '://' + (h['x-forwarded-host'] || h.host); const urls = ['/', '/services', '/about', '/contact'].concat(input.rows.map(r => '/blog/' + r.slug)); return { xml: '<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">' + urls.map(u => '<url><loc>' + base + u + '</loc></url>').join('') + '</urlset>' }"
| web.response --body "{{ input.xml }}" --header Content-Type=application/xml
```

```
| trigger.webhook --path /robots.txt --method GET
| web.response --message "User-agent: *\nDisallow: /admin\nDisallow: /login\nSitemap: /sitemap.xml" --header Content-Type=text/plain
```

```
| trigger.weberror --code 404
| web.response --status 404 --template pages/not-found.tsx
```

The 404 page is the **empty** archetype with a search or the main links;
its status must be 404 (a 200 "not found" page poisons search results).
The site's own address is never written down (`addressing.md`): a pipeline
that needs it reads `$trigger.headers.host` (and `x-forwarded-proto` behind
a proxy), as the sitemap does above.

## 5. Performance budget

| Thing | Budget | How |
|---|---|---|
| images | ≤ 200 KB each, sized to their slot, `width`/`height` set, `loading="lazy"` below the fold — except the one hero image, which loads eagerly | `fs.thumbnail --format webp` at upload; never the original in a list |
| fonts | ≤ 2 families, ≤ 4 files, `display=swap` | `page.head.links` with the Google Fonts CSS URL; the variable in `globals.css` |
| page HTML | ≤ 100 KB | paginate lists at 20; do not inline a table of 500 rows |
| third-party scripts | none unless the brief names one | analytics via a single `page.head.links`/script entry |

## 6. Accessibility musts

- exactly one `h1`; `h2`/`h3` in order; landmarks: `header`, `nav`, `main`, `footer` (PageShell gives them).
- every `img` has `alt` (empty `alt=""` for decoration); every input has a `Label`; every icon-only button has `aria-label`.
- links say where they go ("View all bookings", not "here").
- colour is never the only signal: status has a word or an icon beside the tint.
- focus is visible (zeb/ui's `ring` — do not remove it); the tab order follows the reading order.
- language set: `page.html.lang = "en"` (or the site's).

## 7. Count what the brief said matters

Name the events from `docs/brief.md` Q10 and record them server-side —
no client script needed: an `INSERT INTO events (name, path, at)` in the
POST pipeline of the primary action, and in the thank-you page's pipeline.
One `list` page in the admin shows them by day. Add a third-party
analytics tag only if the brief asks.

## Prove it

- `route_fetch` each v1 page and read its `seo` object — the rules are numbers:
  `seo.title` present and distinct per page · `seo.description_chars` 120–155 (public pages) ·
  `seo.h1_count == 1` · `seo.canonical_absolute == true` · `seo.og.image` set and `seo.og.absolute == true` on shareable pages ·
  `seo.jsonld_types` contains the archetype's type (`Person`, `Article`, `Event`, `Organization`) ·
  `seo.hreflang` lists every language the page has · `seo.images_without_alt == 0` · `seo.robots` is `noindex…` exactly on the NOINDEX routes.
  Then `route_fetch` the `og:image` URL itself: 200 and an image content type.
- `route_fetch path=/sitemap.xml` → 200 XML listing every public page; `/robots.txt` → 200 with the Sitemap line; an unknown path → 404 with the designed page.
- Submit the primary form once: 303 → thank-you (noindex), the row exists, refreshing the thank-you does nothing.
- One screenshot at 390px of each archetype: the CTA visible on landing without scrolling.
- Write the metadata table (route · title · description · canonical) into `docs/growth.md`.
