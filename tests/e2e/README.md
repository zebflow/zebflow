# Zebflow UI smoke tests

Browser tests for the Studio. They cover the one thing `cargo test` cannot:
that a page **works**, not merely that its template **compiles**.

The Rust suites stay authoritative for everything reachable by `curl` —
`tests/platform` is faster and already covers the HTTP API, the office
protocol, and pipeline execution. Nothing here should duplicate them.

## Running

```bash
cd e2e
npm install
npx playwright install chromium   # once
npm test
```

The suite talks to a dev server on `http://localhost:10610`. If one is already
running it is adopted as-is; if the port is free, `dev.sh` is started for you
and shut down afterwards. Override the target with `ZEBFLOW_BASE_URL`.

Credentials default to `superadmin` / `admin123` and can be overridden with
`ZEBFLOW_OWNER` and `ZEBFLOW_PASSWORD`.

## What is covered

- `specs/studio-pages.spec.ts` — every Studio surface renders, carries its own
  title, and logs no console error or uncaught exception. Plus settings tab
  navigation, asserted on the resulting URL.
- `specs/project-lifecycle.spec.ts` — the path a new user walks: create a
  project, confirm the scaffold is a flat repository with the samples and **no
  scaffolded folders**, confirm a sample pipeline 404s until activated and
  answers afterwards, then confirm the sample page renders with `globals.css`
  applied and its counter hydrates.

The lifecycle spec creates a real project and deletes it in `afterAll`, so a
failed run leaves nothing behind.

## The one rule that matters

**Never assert that a click succeeded — assert the state it should produce.**

Zebflow renders on the server and hydrates after. A click that lands before the
handler is attached is swallowed with no error at all, and Playwright still
reports the click as successful. `clickUntil()` in `fixtures.ts` exists for
this: it retries the click until the observed state changes, so a slow compile
is not mistaken for a dead button, and a genuinely dead button still fails.

This is not hypothetical. It cost real debugging time: a wedged browser session
once reported successful clicks while delivering zero DOM events, which looked
exactly like broken hydration until a known-good control page proved otherwise.

## Keeping it honest

If you add a spec, break one of its assertions on purpose once and watch it
fail. A test that has never failed has not been shown to test anything.
