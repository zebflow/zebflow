---
name: brand-system
description: Giving a Zebflow site one coherent look from one decision — a seed colour and a mood word become the complete theme (every token, light and dark, contrast-checked, in globals.css), a type pairing, radius, density and shadow, written to docs/brand.md. Use before the first page of a site, when a brief names colours or a logo, or when pages look like they belong to different apps; never invent colours page by page.
license: MIT
metadata:
  version: "1"
  derived_from: "colour derivation designed with GPT-6-Astra (2026-09-14); implemented as the theme_generate tool"
---

# Brand system: one seed, one mood, the whole theme

A model choosing colours per page produces a site with no brand, and a
model computing contrast by hand gets it wrong. So the decision is made
once, from two inputs, and the arithmetic is the platform's: `theme_generate`
returns every token for light and dark, already fitted to WCAG contrast.
You place the result and write down why. Facts: `help(topic="web/tailwind")`
(token roles, fonts); the live tokens at `/dev/design-system`.

## 1. Two inputs, nothing else

From `docs/brief.md` (`business-brief`):

- **Seed** — the logo's main colour or the brief's colour, as `#RRGGBB`.
  If none is given, the mood's default (the tool knows them; pass no seed).
- **Mood** — one of `calm · warm · serious · energetic · playful · luxurious · technical`,
  from the brief's three adjectives:

| Adjectives like… | Mood |
|---|---|
| trustworthy, medical, quiet, careful | calm |
| friendly, local, food, family | warm |
| legal, financial, institutional, formal | serious |
| sport, startup, sale, bold | energetic |
| kids, creative, fun | playful |
| premium, boutique, heritage | luxurious |
| developer, data, tool, precise | technical |

Do not ask the user for more. Everything below follows.

## 2. Generate

```
theme_generate seed="#1e66d6" mood=calm
```

The answer carries:

- `light` and `dark` — all 38 colour tokens as hex;
- `css` — the `:root { … }` and `.dark { … }` blocks, `--radius` included;
- `style` — display and text fonts, sizes, line heights, radius, control
  height, card padding, grid and section gaps, card shadow, and the Google
  Fonts `fonts_href`;
- `contrast` — the measured pairs (text on background, muted on card,
  primary-foreground on primary, …) with the minimum each had to meet;
- `notes` — anything the tool changed (a seed too light for 3:1 on white
  is moved; a grey seed takes the mood's hue). Read them; they go in
  `docs/brand.md`.

Every text pair meets 4.5:1 and every mark 3:1 in both modes, or the tool
would have refused. A seed you love that came back moved is still your
brand: the original stays in the logo and the imagery; the token is what
buttons and links use.

## 3. Place it

1. `file_read rel_path="globals.css"`; replace the existing `:root { … }`
   and `.dark { … }` blocks with `css` from the tool, keep everything else
   in the file. Add the font variables under `:root`:
   `--font-sans: "<style.sans_font>", system-ui, sans-serif;` and
   `--font-display: "<style.display_font>", <style.display_fallback>;`.
2. In `PageShell` (or every page's `page.head.links`), load the fonts once:
   `{ rel: "preconnect", href: "https://fonts.googleapis.com" }`,
   `{ rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }`,
   `{ rel: "stylesheet", href: <style.fonts_href> }`.
3. Headings `font-display`; everything else `font-sans`. Sizes are fixed
   for the whole site from `style`: `h1` = `h1_px` (desktop/mobile), `h2` =
   `h2_px`, `h3` 24px, body 16px, small 14px; line heights from `style`.
4. Geometry from `style`: `--radius` is already in the CSS; controls are
   `control_height_px` tall; cards pad `card_padding_px`; grids gap
   `grid_gap_px`; sections gap `section_gap_px`; cards get `card_shadow` in
   light and a border in dark.

## 4. Write it down

`docs/brand.md`, from the tool's answer — a page, not an essay:

```markdown
# Brand — <name>
Seed <hex> · mood <mood> · notes: <the tool's notes, or "none">
Primary light <hex> / dark <hex> · primary-foreground <hex>
Type: <display> (headings) + <sans> (text) · h1 <d>/<m> px · body 16 / <line-height>
Geometry: radius <r>rem · controls <h>px · card padding <m>/<d> · grid gap <g> · sections <m>/<d> · shadow <light: …, dark: border>
Contrast: <the tool's table, one row per pair, light and dark>
Voice: <the brief's three adjectives> — buttons are verbs, headings ≤ 8 words, no exclamation marks.
Imagery: <photos of real work | illustration | none>; never stock people.
Logo: <file or "wordmark in the display font">
```

## 5. Rules that keep it a brand

- Pages name **roles**, never colours: `bg-primary text-primary-foreground`,
  `text-muted-foreground`, `border-border`. A palette class
  (`bg-slate-800`), a hex, or an inline colour in a page is a defect.
- Text uses `foreground` or a surface's `-foreground` companion; secondary
  text uses `muted-foreground`. `primary` is a fill, not guaranteed as text
  — links are `text-info underline` or foreground with an underline.
- Status is a fill plus its foreground plus a word or icon
  (`bg-warning/10 text-warning` + "Pending"), never a tint alone.
- One display font, one text font, one radius, one spacing scale
  (4/8/12/16/24/32/48/64) — for the whole site. A page that "needs" another
  is wrong, or the brand is; change the brand once, in `globals.css`.
- Regenerate rather than hand-tune: a new seed or mood is one
  `theme_generate` call and one paste, and the contrast holds again.

## Prove it

- `file_search pattern="bg-(slate|gray|zinc|neutral|stone|red|blue|green|amber|rose|sky)-|#[0-9a-fA-F]{6}|style=\\{\\{" glob="**/*.tsx"` finds nothing outside `globals.css`.
- `route_fetch path=/` — the body's `<link rel="stylesheet">` for fonts is present once; every `var(--…)` the page uses is defined in `globals.css`.
- Two archetypes side by side, light and dark (`design-verify` B4, Y7), look like one site.
- `docs/brand.md` carries the tool's contrast table and notes, so the next session knows what was decided and why.
