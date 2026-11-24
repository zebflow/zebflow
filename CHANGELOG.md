# Changelog

What changed in each Zebflow release, newest first, in three groups:
**New** (what you can do now), **Changed** (what behaves differently) and
**Fixed**. The commits hold the detail; this file says what it means for
you.

From 0.12, a release that adds a capability raises the middle number
(0.13.0); a release that only fixes raises the last (0.13.4)
([versioning](docs/contracts/versioning.md)).

## 0.15.0 (2026-11-24)

### New

- Browser modules: declare a project folder in the RWE settings
  (`rwe.browser_modules`) and its files become real browser modules, for
  large client code such as a game or an editor. Each entry you name is built
  with everything it imports into one script; code two entries share is
  stored once; every file keeps its own names, so a hundred files may each
  export `build`. Modules may use relative imports, `import.meta.glob`,
  `import()` for parts loaded on demand, JSON and `?raw` text imports, and
  module workers. They never run on the server, and a change applies on the
  next page load with no restart. A page imports only the folder's entries.
  See `help("web/browser-modules")`.
- Studio's file tree marks browser module folders, entries and the files
  inside them, and a file's menu can make it an entry or stop it being one.
  File listings in the API and MCP carry the same information in a `module`
  field.
- Project pages answer with a `Server-Timing` header (compile, server render,
  styles, scripts), shown in the browser's developer tools.
- Shader files (`.glsl`, `.vert`, `.frag`, `.wgsl`) are accepted project files.

### Changed

- Faster pages. A page is bundled once instead of twice when it compiles;
  the styles step reuses its work across requests; text responses are sent
  gzip-compressed when the browser accepts it, each file compressed once;
  library and asset files carry an `ETag`, so a browser that already has one
  gets a short "not modified" instead of the whole file. On the dev server,
  Studio's editor page went from 2.2 s to 1.0 s per request, and the three.js
  library from 1.7 MB to 0.3 MB on the wire.
- `zeb/threejs` lists r187 as its version everywhere. The old r183 copy is
  removed; a project pinned to it uses r187.

## 0.14.0 (2026-11-24)

### New

- Studio's Hub page publishes a project release to a Hub kept in a GitHub
  repository: the official Hub (`github.com/zebflow/hub`) or one of your own.
  Pick the destination, give the repository, branch and a token that can
  write to it, review what will change, then publish: Zebflow writes the
  release files and the Hub's index, commits and pushes. The token is used
  for that push only and never stored or shown in a log.
- A Hub entry can carry tags, a cover image, a gallery of images with alt
  text, and a README.
- `PotoPlayer` takes `loading`: `"lazy"` (the default) fetches a story, its
  fonts and its engines only when the player comes within half a screen of
  view; `"eager"` fetches at once.

### Changed

- `zeb/potoru` 0.1.4 plays stories with Potoru's lean web player. A story
  that moves something along a path no longer downloads the 1.1 MB drawing
  engine: path following runs in plain JavaScript, and the engine loads only
  for text placed on a path. Players on one page share one player module and
  fetch each font once. On a page with five stories, the bytes downloaded
  fell from about 1.5 MB to about 0.5 MB.
- `zeb/threejs` and `zeb/threejs-vrm` carry three.js r187. r187 removed
  `CubeUVReflectionMapping`; code that names it must drop it.

## 0.13.5 (2026-11-22)

### Changed

- `cargo install zebflow` installs Zebflow again. Like `pip install zebflow`
  and `npm install -g zebflow`, it installs a small launcher that downloads
  the ready-made program for your computer from the GitHub release, checks it
  against the release's checksums, and runs it. Zebflow's source is no longer
  published on crates.io (it had grown past crates.io's size limit); to build
  Zebflow yourself, clone the repository.

### New

- `curl -fsSL https://zebflow.com/install.sh | sh` installs Zebflow on Linux,
  macOS and a 64-bit Raspberry Pi in one line.

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
