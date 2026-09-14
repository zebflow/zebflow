---
name: business-brief
description: Turning "build me a website for X" into a brief the rest of the build can follow — who it is for, the one action a visitor must take, the pages of version one, tone, proof, constraints. Use before designing or writing any page for a business, an organisation, a product or a person; the output is docs/brief.md, which brand-system, ui-decide and growth-rules read first.
license: MIT
metadata:
  version: "1"
---

# Business brief: decide the site before drawing it

A site built without a brief is generic on purpose: nobody said who it was
for, so every choice was hedged. The brief is ten answers and one page. It
takes fifteen minutes and it is the difference between a template and a
site.

## When the brief is enough

You may start building when `docs/brief.md` answers all of: **audience**,
**primary action**, **v1 pages**, **tone**, **proof**. Everything else can be
"not yet". If any of those five is missing, ask the user one question —
the next unanswered one below — rather than guessing. Do not ask all ten at
once.

## The ten questions, in order

Ask only the ones the request left open. Write the answer as a sentence,
not a keyword.

| # | Question | A good answer looks like | If unanswered |
|---|---|---|---|
| 1 | **Who is this for?** One primary audience, named as a person | "clinic patients aged 30–70 in Northside who book online" | ask — nothing works without it |
| 2 | **What is the one thing a visitor should do?** | "book an appointment", "request a quote", "sign up", "read and subscribe" | ask — this decides the CTA on every page |
| 3 | **What is the second thing?** (at most one) | "call us" | default: none |
| 4 | **Which pages does version one need?** | 3–6 pages: home, services, about, contact … | derive from 2: the pages the action needs, plus home and contact |
| 5 | **What proves the claim?** | credentials, years, client names, numbers, reviews, photos of real work | ask — pages without proof read as ads |
| 6 | **Tone, in three adjectives** | "calm, competent, local" | default: "clear, warm, direct" |
| 7 | **What exists already?** | logo, colours, photos, copy, a domain, an old site, a brand guide | ask once; reuse everything real |
| 8 | **What must not happen?** | "no stock photos", "no popups", "never mention prices" | default: none |
| 9 | **Who maintains it, and how often?** | "the receptionist updates hours; monthly posts" | decides whether an admin area is v1 |
| 10 | **How will we know it worked?** | "5 bookings a week", "20 signups a month" | default: the primary action's count |

Two things you never ask the user: colours and layout. Those are derived
(`brand-system`, `ui-decide`) from the answers above.

## Write `docs/brief.md`

```markdown
# Brief — <name>

**For:** <audience as a person>. **They arrive** <from search / a card / a referral> **wanting** <the need in their words>.
**Primary action:** <verb phrase> → route `<path>`. **Secondary:** <verb phrase or "none">.
**Version one pages:** `/` home · `/services` · `/about` · `/contact` · <…>  (<= 6)
**Proof we can show:** <list: numbers, names, credentials, photos, reviews>
**Tone:** <adj>, <adj>, <adj>. **Never:** <constraints>.
**Assets that exist:** <logo / photos / copy / domain / old site>.
**Maintained by:** <who>, <how often> → admin area: <yes / not in v1>.
**Success:** <measure>.
**Open questions:** <what you assumed, so the user can correct it>
```

Twelve lines. If it runs longer, it is a plan, not a brief; move the rest
to `docs/plan.md`.

## Turn the brief into the first decisions

Once written, three things follow mechanically — do them before any page:

1. **Sitemap**: one line per v1 page: route, title, its purpose in five
   words, its CTA (primary or secondary — every page has one).
2. **Copy skeleton**: for each page, the headline and one sentence, in the
   audience's words from Q1, at the tone from Q6. Write it now; layouts are
   chosen around copy, not the other way round (`ui-decide`).
3. **Proof placement**: where each proof item from Q5 appears (home above
   the fold gets the strongest one).

Add these three under the brief as `## Sitemap`, `## Copy`, `## Proof`.

## Prove it

- `docs/brief.md` exists, answers the five required questions, and names
  a route for the primary action.
- Every page in the sitemap has a CTA; the primary action's route is
  reachable from every page (it goes in `PageShell`).
- The user has seen the brief before you build. Ask once: "Anything
  wrong here?" — a wrong brief costs an hour to fix now and a day later.
