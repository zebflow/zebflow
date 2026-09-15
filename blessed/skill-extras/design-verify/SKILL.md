---
name: design-verify
description: Judging whether a Zebflow page is well made, not just working — a pass/fail checklist over a screenshot and the DOM at desktop and phone width (contrast, one h1 and heading order, tap targets, text size, line length, no horizontal scroll, focus, colour as the only signal, states, consistent spacing), and how to report what failed. Use after zebflow-verify says the route works and before you say a page is done.
license: MIT
metadata:
  version: "1"
---

# Design verify: look at it the way a visitor will

`zebflow-verify` proves the route answers and hydrates. This proves it is
usable and consistent. Every check is pass/fail; none needs taste. Run it
on each archetype once (`ui-decide`), and on any page after a layout change.

## 1. Capture

Two screenshots per page, full height, from the headless browser
(`zebflow-verify` §2 has the script): **1280×800** and **390×844**. Fetch
the DOM once (`route_fetch` or `page.content()`). Read the screenshot as an
image — do not judge layout from HTML.

## 2. Checks

Mark each PASS or FAIL. A FAIL names the element (its text or selector),
the measurement, and the rule.

**Structure (DOM)**
| # | Check | Rule |
|---|---|---|
| S1 | one `h1` | exactly one; it states the page's subject, not the site name |
| S2 | heading order | `h2` under `h1`, `h3` under `h2`; no level skipped |
| S3 | landmarks | `header`, `nav`, `main`, `footer` present once each (`PageShell`) |
| S4 | images | every `img` has `alt`; decorative ones `alt=""`; `width`/`height` set |
| S5 | controls | every input has a label; every icon-only button an `aria-label`; every link text says where it goes |
| S6 | metadata | from `route_fetch`'s `seo`: `title` present and specific, `description_chars` 120–155 on public pages, `canonical_absolute`, `og.absolute`, the archetype's type in `jsonld_types`, `images_without_alt == 0` |

**Legibility (screenshot + computed styles)**
| # | Check | Rule |
|---|---|---|
| L1 | body text size | ≥ 16px on phone, ≥ 14px on desktop for any text a person must read |
| L2 | line length | body paragraphs ≤ 75 characters per line (`max-w-prose` / `max-w-3xl`) |
| L3 | contrast | text on its background ≥ 4.5:1 (≥ 3:1 for ≥ 24px). Check the pairs: `foreground/background`, `muted-foreground/background`, `muted-foreground/card`, `primary-foreground/primary`, placeholder text, disabled text is exempt |
| L4 | line height | body 1.5–1.7; headings 1.1–1.3 |
| L5 | colour as the only signal | every status, error and success has a word or icon beside its tint |

**Layout (screenshot)**
| # | Check | Rule |
|---|---|---|
| Y1 | no horizontal scroll at 390px | `document.documentElement.scrollWidth <= 390` |
| Y2 | tap targets | every button, link and input ≥ 44×44 px on phone, ≥ 8px between neighbours |
| Y3 | above the fold | landing/auth/pricing: the `h1` and the primary button visible at 1280×800 and 390×844 without scrolling |
| Y4 | width | content inside the archetype's max width; nothing edge-to-edge but the shell |
| Y5 | spacing scale | gaps come from one scale (4/8/12/16/24/32/48/64); no odd values; section padding equal above and below |
| Y6 | alignment | text left-aligned in one column; numbers right-aligned in tables; nothing centred except auth and empty states |
| Y7 | consistency | two pages of the same archetype: same title row, same widths, same button styles, same footer — compare their screenshots side by side |

**Behaviour (browser)**
| # | Check | Rule |
|---|---|---|
| B1 | focus visible | tab through the page: every focused control shows the `ring`; the order follows the reading order |
| B2 | states exist | list: empty state renders (query for a row that cannot exist); form: error state renders with input preserved; detail: not-found renders the 404 archetype |
| B3 | no console errors | as `zebflow-verify` |
| B4 | dark mode | add `dark` to `<html>`: nothing unreadable, no raw colours that failed to switch |
| B5 | motion | nothing moves without being asked; no autoplay, no layout shift when fonts load (`display=swap` + `size-adjust` or system fallback close in width) |

## 3. Fix order

Fix in this order, because each earlier item changes the later ones:
S (structure) → Y1 (overflow) → L1–L3 (legibility) → Y3–Y7 (layout) → B.
A page failing S1 or Y1 is not shipped, whatever else passes.

## 4. Report

Write `docs/design-verify.md`, one section per page, shaped like this:

> **/book — form archetype — 2026-09-14**
>
> | Check | Result | Detail |
> |---|---|---|
> | S1 one h1 | PASS | "Book an appointment" |
> | L3 contrast | FAIL | `text-muted-foreground` on `bg-card` = 3.9:1 (needs 4.5) — help text under Email |
> | Y2 tap targets | FAIL | "Cancel" link 31px tall at 390px |
> | Y7 consistency vs /contact | PASS | |
>
> Fixed: L3 (muted-foreground darkened in globals.css: #525d68 → #475260), Y2 (link → Button variant="ghost").

Then re-run the failed checks only and update the table. A page is done
when every row is PASS and the table is in the repo.

## Prove it

- The two screenshots exist for each archetype the site uses.
- `docs/design-verify.md` has a table per page with no FAIL left.
- One deliberate break (remove the `h1`) makes S1 fail — the check is
  real, not decorative.
