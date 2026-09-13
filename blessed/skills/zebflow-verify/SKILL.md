---
name: zebflow-verify
description: Proving that something in a Zebflow project works before reporting it done — fetching routes, reading bodies for RWE component error, invocations, and driving the page in a headless browser (console errors, clicks that must change state, forms, uploads). Use before every completion claim about a route or a page, and when a click "does nothing".
license: MIT
metadata:
  version: "1"
---

# Verify on the running instance

Three things lie in Zebflow, each in its own way: a **200** that carries a
component error; **server HTML** that is right while hydration died; and a
**click that reported success** while the handler was not attached yet. None
of them is visible in a status code. Verification is reading what came back
and asserting the state an action should produce.

## 1. The route

```
pipeline_list status=all                     → the pipeline is active, not draft or stale
curl -s -i http://<host>/wh/{owner}/{project}{path}
```

Read, do not skim:

- the status you meant (`200`, `303` with `Location`, `400` with the message);
- for a page, `grep -c "RWE component error"` is `0` — a throwing component is
  replaced by that comment and the response is still 200;
- the data is in the HTML (the title you inserted, the row count);
- the failure paths: the unauthenticated request, the invalid body.

Then `pipeline_get_invocations file_rel_path=…`: one run, no error, the nodes
you expected in the trace. A schedule that rendered HTML, a webhook that ran
a node twice, a script that returned `null` into a query — this is where they
show.

Use a cookie jar for browser-style calls (`--cookie-jar` on `POST /login`,
`-b` after) and `-H "Authorization: Bearer …"` for API-style calls; the two
get different answers on purpose (303 vs 401).

## 2. The page in a browser

Server HTML says nothing about hydration. Open the page with the console
visible, or drive it headlessly:

```js
// probe.mjs — run from a folder with @playwright/test installed
import { chromium } from "@playwright/test";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
const errors = [];
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
page.on("pageerror", (e) => errors.push(String(e)));
await page.goto(process.argv[2]);
await page.waitForTimeout(800);                            // hydration
await page.getByRole("button", { name: "Add post" }).click();
await page.waitForSelector('[role="dialog"]');            // assert the state the click should produce
console.log(JSON.stringify({ errors, dialogs: await page.locator('[role="dialog"]').count() }));
await browser.close();
```

Rules of evidence:

- **A console error is a failure**, whatever the page looks like. The usual
  causes: a component using a name it did not import (`X is not defined`),
  `window`/`document` during render, a library called on the server.
- **Never trust a click that reported success.** Assert what it should have
  changed — a dialog open, a row added, a URL changed, a toast shown. A
  click before hydration is swallowed silently; wait for hydration (a short
  pause, or wait for an element only hydration renders) and click again.
- **If clicks stop working, the browser session is wedged**, not the app:
  close it and navigate again before concluding anything.
- **Scroll matters.** Playwright's mouse drags and clicks miss elements
  outside the viewport; use a tall viewport or `scrollIntoViewIfNeeded()`.
- **Keys differ by platform.** On macOS `Shift+Home` selects to the document
  start; use `Shift+ArrowLeft` × n for a portable selection.
- Check 375 px width and the dark theme for anything user-facing.

## 3. Forms, uploads, editors

- A form: submit it and follow the redirect; the target page shows the new
  row. Submit it wrong and the error is shown next to the field.
- An upload: `page.waitForEvent("filechooser")` around the click, `setFiles`,
  then the `<img>` has `naturalWidth > 0` and `pipeline_get_invocations` shows
  `fs.save`.
- The editor: type `/`, choose a block, type text, select it and press bold;
  the rendered `DocumentView` beside it (or the stored JSON) contains what you
  typed, in the block you chose.

## 4. After a change to a shared component

Every page that inlined the component recompiles on the next request. Fetch
**each** route that imports it (`file_deps rel_path=components/x.tsx` lists
them) — the one you did not fetch is the one that broke.

## 5. What to write down

In `MEMORY.md`: what was verified, how (the URL, the probe, the invocation
id), and what was not. "Verified" without a how is a claim; the next session
will re-verify it, and you have saved nothing.
