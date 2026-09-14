---
name: ui-decide
description: Choosing the layout of a page in a Zebflow project without inventing one — a decision table from what the page is for to one of ten archetypes (landing, list, detail, form, dashboard, auth, settings, article, pricing, empty/404), each with a copyable recipe on zeb/ui inside one PageShell. Use before writing any page .tsx, and when two pages that should match do not.
license: MIT
metadata:
  version: "1"
---

# UI decide: ten layouts, no eleventh

Good sites are made of a few layouts used consistently, not many used once.
Pick the archetype from the table, copy its recipe, fill it with the copy
from `docs/brief.md`. Facts: `help(topic="web/ui")` for the components,
`help(topic="web/tailwind")` for theme roles, `zebflow-engineering` for
`PageShell` and where files go.

## 1. Read first

`docs/brief.md` (the CTA and tone), `docs/brand.md` if it exists (radius,
density, type), and `shared/components/page-shell.tsx`. If there is no
`PageShell`, make it before any page (recipe at the end).

## 2. Pick the archetype — first match wins

Go down the table; the first row that describes the page's main job is
the archetype. The order matters: it resolves the ties.

| # | The page's main job | Archetype | Typical routes |
|---|---|---|---|
| 1 | say a thing is missing, done, or empty | **empty / 404** | `/404`, `/thank-you`, a list with no rows |
| 2 | sign in, sign up, recover, confirm | **auth** | `/login`, `/register`, `/reset` |
| 3 | change existing preferences or configuration | **settings** | `/account`, `/admin/settings` |
| 4 | collect information to start or finish something | **form** | `/contact`, `/book`, `/admin/posts/new` |
| 5 | compare plans and choose one | **pricing** | `/pricing`, `/plans` |
| 6 | be read top to bottom | **article** | `/about`, `/blog/:slug`, `/docs/:page` |
| 7 | watch several numbers or a work queue and act | **dashboard** | `/admin`, `/app` |
| 8 | understand one thing fully and take its action | **detail** | `/posts/:slug`, `/products/:id` |
| 9 | browse, search or filter a collection | **list** | `/posts`, `/products`, `/admin/bookings` |
| 10 | understand the offer and take the primary action | **landing** | `/`, `/services/<x>` |

Ties, decided: a list with zero results is still a **list** (with its empty
state inside); a product page with a buy button is **detail**, not form; a
settings page full of inputs is **settings**; a thank-you page is
**empty/404** in its "done" mode and answers 200, a missing page answers 404.
A page that seems to need two archetypes is two pages, or one archetype
with a section borrowed from another (a landing may end in a **form**
section; a detail may include a **list** of related items). Never a third layout.

## 2b. Rules of the shell every recipe assumes

| Item | Rule |
|---|---|
| Ownership | one `PageShell` owns header, nav, footer, skip link and the single `<main id="main">`; pages add none of these |
| Header | 64px; wordmark if no logo; ≤ 5 top-level links; current page `aria-current="page"`; the primary CTA on the right |
| Mobile nav | below 768px a labelled button opens a `Sheet`; it closes on selection |
| Gutters | 16px below 768px, 24px to 1023px, 32px from 1024px |
| Breakpoints | mobile < 768, tablet 768–1023, desktop ≥ 1024 (`md:` and `lg:`) |
| Fold | at 390×844 and 1440×900 the `h1` and the page's main action (or first useful content) are visible without scrolling |
| Emphasis | one primary-styled action per decision area; the rest outline, secondary or link |
| Semantics | native headings, lists, links, sections, forms around zeb/ui parts; a `Card` groups, it is not a default wrapper |
| Data | the first useful content is server-rendered from `input`; never invented records, ratings or testimonials |

## 3. Recipes

Every recipe assumes `<PageShell>` provides header, nav with the primary
CTA, footer and the toast region. Content width: `max-w-6xl` for landing,
list, dashboard, pricing; `max-w-3xl` for detail, form, article, settings;
`max-w-sm` for auth. All grids collapse to one column below `md:`.

**landing** — `max-w-6xl`
1. Hero: `h1` (the promise, ≤ 8 words), one sentence, primary `Button`, secondary link. Above the fold, with the strongest proof item beside or under it.
2. Proof strip: 3–4 numbers or logos (`Card`-less, `text-muted-foreground`).
3. What we do: 3 `Card`s max, icon + title + one sentence, each linking to a detail page.
4. How it works or Who it is for: 3 numbered steps, or 2 columns text + image (`AspectRatio`).
5. Testimonial or case: one, with name and role (`Avatar`).
6. FAQ: `Accordion`, 4–6 items.
7. Final CTA: repeat the hero's action in one line.
States: none. Mobile: hero image below text; steps stack.

**list** — `max-w-6xl`
1. Title row: `h1`, count (`Badge`), primary action button (`New …`) on the right.
2. Filter bar: `Input` (search) + `Select`/`ToggleGroup` for one or two facets. Not more.
3. Rows: `Table` for data with ≥ 4 columns; `Card` grid (`md:grid-cols-3`) for things with an image; `Item` list otherwise. Each row is a link to its detail.
4. `Pagination` at the bottom; 20 per page.
States: empty → the **empty** archetype inline with the primary action; loading → `Skeleton` rows; error → `Alert variant="destructive"` above the rows.
Mobile: table becomes stacked cards (hide secondary columns with `hidden md:table-cell`).

**detail** — `max-w-3xl`
1. `Breadcrumb` back to the list.
2. Header: `h1`, meta line (date, author, status `Badge`), the item's action button.
3. Body: the content; images in `AspectRatio`; long text at `leading-7`.
4. Related: a 3-item **list** section.
States: not found → **empty/404**. Mobile: action button full width under the header.

**form** — `max-w-3xl` (or `max-w-md` for ≤ 4 fields)
1. `h1` + one sentence saying what happens after submit.
2. `Field`s in one column: `Label` + `Input`/`Textarea`/`Select`/`RadioGroup`/`Checkbox`; required marked; help text under the field, not in a placeholder.
3. One primary `Button` with a verb ("Book appointment", never "Submit"); a quiet cancel link.
4. On error: `Alert variant="destructive"` at the top *and* the field marked (`aria-invalid`); keep the user's input.
5. On success: redirect to a thank-you or the created item (`growth-rules`).
Mobile: unchanged; buttons full width.

**dashboard** — `max-w-6xl`, or full width with a navigation rail (`bg-sidebar`, a `nav` of links; there is no Sidebar component)
1. Title row with a date/scope `Select`.
2. KPI row: 3–4 `Card`s, one number each, a delta in `text-success`/`text-destructive` with an arrow (never colour alone).
3. Main: a chart (`zeb/d3`, or a `Table` when numbers speak for themselves) two-thirds wide, a **list** of recent items one-third.
4. Actions: the operator's three most common, as buttons in the title row.
States: empty → "Nothing yet — <first action>"; loading → `Skeleton` cards. Mobile: KPI cards 2×2, main stacks.

**auth** — `max-w-sm`, centred vertically
1. Logo/name, `h1` ("Sign in"), one line.
2. `Field`s: email, password; a "forgot?" link under password.
3. Primary `Button` full width; secondary route ("Create an account") under it.
4. Error: `Alert variant="destructive"` above the fields, same message for wrong email and wrong password.
No PageShell nav on auth pages; a minimal header with the logo only.

**settings** — `max-w-3xl`
1. `h1`, then `Tabs` when there are ≥ 3 groups; otherwise stacked sections.
2. Each section: title, one sentence of *why*, fields, its own Save button; a status line under it ("Saved." / the error).
3. Danger zone last, `border-destructive/30`, confirm via `AlertDialog` with the consequence in words.
Mobile: tabs scroll horizontally.

**article** — `max-w-3xl`, `leading-7`
1. `h1`, meta line, optional cover in `AspectRatio 16/9`.
2. Body: `h2`/`h3` only; paragraphs ≤ 75ch; images with captions; `CodeBlock` for code.
3. Author card and "Next / Previous" at the end.
Mobile: unchanged.

**pricing** — `max-w-6xl`
1. `h1`, one sentence, a `ToggleGroup` for monthly/yearly if it applies.
2. Plan `Card`s (2–4), the recommended one marked with a `Badge` and `border-primary`; price large; 5–7 features with a check; one `Button` each, same verb.
3. Comparison `Table` (optional), FAQ `Accordion`.
Mobile: cards stack; recommended first.

**empty / 404** — `max-w-md`, centred
1. `Empty` from zeb/ui: an icon, a title ("No bookings yet" / "That page does not exist"), one sentence, the primary action (`Button`) or the way home (`Link`).
2. For 404: the pipeline is `trigger.weberror --code 404 | web.response --status 404 --template pages/not-found.tsx`.

## 3b. States and mobile, per archetype

| Archetype | Empty | Loading | Error | Mobile |
|---|---|---|---|---|
| landing | omit optional sections with no approved material | static; reserve media size | broken optional media leaves the copy readable | one column, copy before media, CTA full width |
| list | `Empty` + "clear filters" or the first-item action | 6 `Skeleton` rows | `Alert` + Retry; keep the filter values | filters in a `Sheet`; rows become cards |
| detail | missing record → real 404 | skeleton title/media/summary | `Alert` + Retry for a failure — never call it "not found" | summary and action before the long text |
| form | blank fields with hints and defaults | submit shows `Spinner` + "Sending…"; no double submit | field errors + summary; values kept; focus the first invalid | one column; inputs 16px; submit full width |
| dashboard | KPIs show "—" or a true zero; the queue says what to set up | skeleton per region | region-level `Alert` + Retry; keep what loaded | KPIs 2×2, the rest stacks |
| auth | clean form | submit spinner + "Signing in…" | one generic message for wrong email and wrong password | one column, no illustration |
| settings | real defaults shown | skeleton group; Save spins | keep edits; inline error; retry save | tabs wrap or become a `Select`; danger last |
| article | unknown slug → 404 | server-rendered | Retry on fetch failure | one column; code blocks scroll inside their own box |
| pricing | no approved prices → "Contact for pricing" only if the brief allows | skeleton plans | `Alert` + Retry; stale buy buttons disabled | stack in comparison order; each card repeats the billing basis |
| empty/404 | — | none (never a fake timer) | a working home link | one column, full-width action |

Form errors are not empty states. A missing thing is a 404; a broken
service is a 5xx with an `Alert`, never a "not found" page. Loading states
exist only where data is actually deferred — a server-rendered region
records "loading: not applicable".

## 4. The PageShell recipe

`shared/components/page-shell.tsx`: `header` (name/logo linking to `/`,
nav links for the v1 pages, the primary CTA `Button` on the right, a
`Sheet` menu below `md:`), `main` with `className` from the archetype
width, `footer` (name, address/contact, secondary links, year), and
`<Toaster />`. Props: `title?`, `width?` (`"narrow" | "wide" | "auth"`),
`children`. Nothing else — a shell that takes ten props is a page.

## 5. Write it down

`docs/ui-decisions.md`: one row per route — route · archetype (and mode)
· purpose in five words · primary action · width. Under it, for each route,
the section order you kept and the empty/error fixture you tested. This
is what the next session reads before adding a page.

## Prove it

- Every page file imports `PageShell` and one archetype's recipe is
  recognisable in it (`file_search pattern="PageShell"`); `docs/ui-decisions.md` names every route.
- Two pages of the same archetype, side by side in screenshots, look like
  the same site: same widths, same title row, same button verbs' style.
- At 390px wide: no horizontal scroll, the CTA is visible without
  scrolling on landing and auth, tables have become cards.
- Every list has its empty state; every form has its error and success
  paths; every detail has its not-found (`design-verify` checks these).
