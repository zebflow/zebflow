# Pipeline DSL — web.auto Language

`web.auto` is Zebflow's browser automation sub-language. It describes sequences of browser actions
that can be transpiled to **two targets**:

| Target | Context | Use case |
|---|---|---|
| **in-app** | InteractionRunner (on-site) | Automated UI walkthroughs, demos, guided onboarding |
| **external** | Headless browser (Playwright/Chromium) | Scraping, testing, automating external sites |

---

## Syntax

`web.auto` actions are written one per line, indented under a `--script` block or inline body:

```
<action> [<selector or url>] [--opt value]
```

End the block with `end` or close the `--` body delimiter.

---

## Action Reference

| Action | Args | Description |
|---|---|---|
| `navigate <url>` | url | Go to a URL |
| `wait <ms>` | milliseconds | Wait for duration |
| `wait-for <selector>` | css selector | Wait until element visible |
| `click <selector>` | css selector | Click an element |
| `double-click <selector>` | css selector | Double-click |
| `fill <selector> <value>` | selector, text | Type into input |
| `select <selector> <value>` | selector, option | Select dropdown option |
| `check <selector>` | css selector | Check a checkbox |
| `uncheck <selector>` | css selector | Uncheck a checkbox |
| `press <key>` | key name | Press a keyboard key |
| `scroll <selector>` | css selector | Scroll element into view |
| `hover <selector>` | css selector | Hover over element |
| `capture <name>` | variable name | Save current page snapshot to named var |
| `assert <selector> <text>` | selector, expected | Assert element text equals |
| `screenshot <file>` | filename | Take screenshot (external only) |

---

## In-app Context (InteractionRunner)

Used for guided in-app experiences: demos, onboarding, highlight sequences.

```zf
run onboard-demo --trigger manual \
  -- trigger.manual \
  | web.auto --context app \
    -- navigate /dashboard \
    wait 500 \
    wait-for .zf-dashboard-loaded \
    click [data-tour="step-pipelines"] \
    wait 1000 \
    fill [data-tour="pipeline-name"] demo-pipeline \
    click [data-tour="create-btn"] \
    assert .pipeline-toast "Pipeline created"
```

In-app actions animate on the live UI — cursor moves, highlights, simulated clicks.
Great for automated product tours or reproducible UI walkthroughs.

---

## External Context (Headless Browser)

Used for scraping, external site testing, or automating third-party services.

```zf
run scrape-hn --trigger manual \
  -- trigger.manual \
  | web.auto --context external \
    -- navigate https://news.ycombinator.com \
    wait-for .itemlist \
    capture hn_page \
  | script --lang js -- "return parseHN(input.hn_page);" \
  | sekejap.query --table hn_stories --op upsert
```

External context spins up a headless Chromium instance, executes actions, and returns
captured data (text, snapshots, screenshots) as output for the next pipeline node.

---

## Registering a Periodic web.auto Pipeline

```zf
register daily-price-check --path /jobs \
  | trigger.schedule --cron "0 9 * * *" --timezone "Asia/Jakarta" \
  | web.auto --context external \
    -- navigate https://store.example.com/products \
    wait-for .product-list \
    capture prices \
  | script --lang js -- "return extractPrices(input.prices);" \
  | sekejap.query --table daily_prices --op upsert
```

---

## Combining web.auto + web.render

A common pattern: scrape external data on a schedule, store it, serve it as a reactive page.

```zf
# Job: scrape and store
register scrape-blog --path /jobs \
  | trigger.schedule --cron "0 * * * *" --timezone UTC \
  | web.auto --context external \
    -- navigate https://example.com/feed \
    wait-for article \
    capture articles \
  | script --lang js -- "return { rows: parseArticles(input.articles) };" \
  | sekejap.query --table cached_articles --op upsert

# Page: serve stored articles reactively
register articles-page --path /pages \
  | trigger.webhook --path /articles --method GET \
  | sekejap.query --table cached_articles --op query \
  | web.render --template articles-home --route /articles
```

---

## web.auto --help

```zf
web.auto --help
```

Shows all available actions and flags for the current context.

```zf
web.auto --context app --help     # in-app action list
web.auto --context external --help  # external (headless) action list
```

---

## Notes

- **Selectors**: prefer `data-*` attributes (`[data-tour="step"]`) over class names for stability.
- **Waits**: always `wait-for` before interacting with dynamic content.
- **Captures**: captured data is passed as `input.<name>` to the next node.
- **External screenshots** are saved to `files/private/` and returned as file references.
- **In-app context** requires the user to have the relevant page open; use with `run` for interactive demos.
