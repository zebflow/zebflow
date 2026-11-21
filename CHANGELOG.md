# Changelog

What changed in each Zebflow release, newest first, in three groups:
**New** (what you can do now), **Changed** (what behaves differently) and
**Fixed**. The commits hold the detail; this file says what it means for
you.

From 0.12, a release that adds a capability raises the middle number
(0.13.0); a release that only fixes raises the last (0.13.4)
([versioning](docs/contracts/versioning.md)).

## 0.13.4 (2026-11-22)

### Fixed

- Checking a component (`template_check`, and the check in Zebflow Studio's
  editor) no longer reports a name clash that the pages using it do not have.
  A page renames a component's private names when it includes it; the check
  now includes the component in a stand-in page the same way, instead of
  checking it as if it were the page. A clash in a page's own names is still
  reported, because there it is real.

- A page now serves the new code right after a file it uses changes, whoever
  changed it. Before, a change made through Zebflow control, a git sync or an
  import left a page that had already been served running the old code until
  something else refreshed it. Every repository change now refreshes the
  pages built from that file (a git sync or an import: every page of the
  project).
- A write through Zebflow control refused because the file changed since it
  was read (`expect_hash`) is reported as a refusal, not as a failure.

### Changed

- The page help and the page contract state the naming rule plainly: a
  page's own top-level names must not match a name exported by any file it
  imports; a component's private names never clash.

## 0.13.3 (2026-11-21)

### New

- `template_check` (Zebflow Studio MCP and Zebflow control): check whether a
  page or component compiles, without saving it, with every file it imports.
- `file_stat`, and `expect_hash` on `file_write`, `file_edit` and
  `pipeline_register`: write only if nobody changed the file since you read
  it. A write that finds a newer version is refused and writes nothing.
- Zebflow control may create `trigger.repo` pipelines, watching a folder
  inside its own fence only.

### Changed

- Node pages in docs sites (`--reference zebflow`) have Flags and Examples
  sections, so a page can be navigated; the flags every node takes are listed
  once, on the Nodes page.

0.13.2 was not released; its changes are in 0.13.3.

## 0.13.1 (2026-11-21)

### New

- `sekejap.query.run --write --statements`: up to 100 statements in one
  transaction. Every write lands, or none does; the failing statement is
  named. Schema changes run on their own, not inside a batch.

## 0.13.0 (2026-11-21)

### New

- `trigger.repo --path <folder>`: run a pipeline when files under a folder of
  the project's repository change, by anyone, through Zebflow Studio, MCP,
  Zebflow control, an import or a git sync. It waits for 2 quiet seconds, so
  a burst of saves is one run, and its own writes never start it again.
- `web.site.generate --reference zebflow`: a docs site gets a page for every
  node and every worked example, generated from the running Zebflow and
  linked both ways.

### Changed

- `zeb/markdown` always loads, whatever a project's library list says.

### Fixed

- Docs sites no longer treat a link shown inside code as a real link (it
  stopped the build), and no longer print a page's title twice.

## 0.12.3 (2026-11-20)

### New

- Project workers: `import sortWorker from "@/workers/sort.worker.ts"`
  gives a page the address of that file, built to run in the background with
  `new Worker(sortWorker, { type: "module" })`. A save of the worker or any
  file it imports takes effect on the next page load.

## 0.12.2 (2026-11-20)

### Fixed

- Listing an uploaded library in a project's library list no longer stops the
  built-in `zeb/*` libraries from loading in the browser.

## 0.12.1 (2026-11-20)

### Fixed

- `linux.command.run` could not start `ssh` on servers that run Zebflow as
  user 1000; the image now has an account for that user.

## 0.12.0 (2026-11-20)

### New

- `linux.command.run`: run a command on a Linux machine over SSH, with an
  `ssh` credential that pins the machine's host key, never logs in as root
  and may limit the programs allowed.
- Zebflow control (`zebflow.command.run` with a `zebflow_control`
  credential, from 0.11.31): a project changes its own files, pipelines and
  credentials inside a fenced folder.
- Project libraries (from 0.11.33): upload a library built with zeb-bundler
  in the Hub; pages import its entries by name. Every file is scanned on
  upload and checked again before it is served.
- Zebflow project MCP: a published tool receives its caller's verified
  sign-in as `mcp.auth` (from 0.11.32).
