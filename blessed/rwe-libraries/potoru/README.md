# zeb/potoru

Potoru 2D animation for RWE templates: a player for `.poto` stories and an in-browser compiler
from Potoru source folders. Fully offline: the player, the compiler, the wasm engines and the
built-in fonts ship inside the package; nothing is fetched from a CDN.

The entry bundle (`0.1/runtime/potoru.bundle.mjs`, about 4.5 KB) holds only the Zebflow bridge.
The player code loads when the first `PotoruPlayer` mounts; the compiler code loads only on the
first `compile()` call or when a `PotoruCompiler` mounts. A page that only plays stories never
downloads the compiler.

## Import

```tsx
import { PotoruPlayer, PotoruCompiler, potoru } from "zeb/potoru";
// also: mountPotoruPlayer, mountPotoruCompiler, compile
```

---

## `PotoruPlayer`

```tsx
<PotoruPlayer id="intro" src="/assets/stories/intro.poto" controls autoplay muted loop />
```

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `src` | `string` | — | URL of a `.poto` package (same origin, or CORS without credentials) |
| `libraries` | `string[]` or `string` | — | Linked libraries (`.potolib` URLs); a string is space-separated |
| `controls` | `boolean` | `false` | Show the control bar (seek / slides / restart, mute, captions, fullscreen) |
| `autoplay` | `boolean` | `false` | Start playing (play and slide modes; interactive stories always run) |
| `muted` | `boolean` | `false` | Start muted |
| `loop` | `boolean` | `false` | Loop (play mode) |
| `captions` | `boolean` | `false` | Show captions of narrated stories |
| `mode` | `"play"` \| `"slide"` \| `"interactive"` | from the story | Force a mode the story supports |
| `renderer` | `"canvas"` \| `"canvas-exact"` \| `"svg"` | `"canvas"` | Drawing back end |
| `height` | `string` \| `number` | story aspect ratio | CSS height of the box |
| `aspect` | `string` | — | CSS aspect ratio of the box (`"16 / 9"`) |
| `fit` | `"contain"` \| `"cover"` \| `"fill"` | `"contain"` | How the stage fits the box |
| `camera` | `string` | package default | A story camera id, or `"stage"` |
| `id` | `string` | auto | Container id for `window.__zebPotoru.get(id)` |
| `className` | `string` | — | Classes on the container |

### Events (on the container; they bubble)

| Event | `detail` |
|-------|----------|
| `zeb:potoru:ready` | `{ id, mode, duration, scenes, capabilities, name, slides }` |
| `zeb:potoru:report` | `{ id, from, event, payload, tick, scene }` — an event the story reports to the host |
| `zeb:potoru:ended` | `{ id }` |
| `zeb:potoru:error` | `{ id, code, message, capabilities?, diagnostic? }` — `code`: `fetch`, `integrity`, `needs-library`, `decode`, `engine`, `host-input`, `library-file`, `needs-newer-player`, `mount` |
| `zeb:potoru:play`, `:pause`, `:slide`, `:scene`, `:caption`, `:camera` | as the Potoru player reports them, plus `id` |

### Imperative API — `window.__zebPotoru`

```ts
const player = window.__zebPotoru.get("intro");    // same object as potoru.get("intro")
player.play(); player.pause(); player.seek(2.5); player.restart();
player.next(); player.prev(); player.goto(0);      // slides and multi-Scene stories
player.set("hero", "speed", 2);                    // host bridge: data the story declares input: true
player.fire("hero", "jump");                       // host bridge: events the story declares input: true
player.setCamera("square");
await player.load(bytesOrUrl, { libraries });      // another story (Uint8Array works)
player.currentTime(); player.duration(); player.mode; player.paused;
player.element;                                    // the underlying <potoru-player>
```

The player lives inside a shadow root of the container, so re-renders of the page never touch it.
Theme it with the Potoru custom properties (`--potoru-accent`, `--potoru-bar-shade`, …) on the
container.

---

## `PotoruCompiler` (authoring widget)

```tsx
<PotoruCompiler id="lab" files={sourceFiles} height="600px" />
```

A file list, a plain editor, a diagnostics list linked to file and line, a live preview player and
Compile / Download `.poto` buttons.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `files` | `{ [path]: string }` | `{}` | A Potoru format v4 source folder (path → YAML text) |
| `mode` | `"files"` \| `"script"` | `"files"` | `"script"` is EXPERIMENTAL (see Way B) |
| `script` | `string` | — | The script, in script mode |
| `height` | `string` \| `number` | `"560px"` | CSS height |
| `id` | `string` | auto | Container id for `window.__zebPotoruCompiler.get(id)` |
| `className` | `string` | — | Classes on the container |

Events: `zeb:potoru-compiler:compiled` (`{ id, ok, poto, diagnostics, files, sizes }`) and
`zeb:potoru-compiler:error` (`{ id, ok: false, diagnostics, sizes }`).

```ts
const lab = window.__zebPotoruCompiler.get("lab");
lab.getFiles(); lab.setFiles(files); const result = await lab.compile(); lab.download();
```

---

## `compile(input)` — in the browser

```ts
const result = await potoru.compile({ files });            // Way A
// result: { ok, poto?: Uint8Array, diagnostics: Diagnostic[], files?, scenes?, sizes: { poto, source, files, ms } }
// Diagnostic: { code, severity, message, file, line, column?, path?, fix? }
```

- **Way A — `{ files }`**: a format v4 source folder as a map of path → text (`Uint8Array` for
  binaries). Validated and compiled with Potoru's own format code, exactly as the `potoru` CLI's
  `export` does (same bytes). Options: `libraries` (`Uint8Array[]` of `.potolib`), `scene`,
  `state`, `locale`, `profile: "presentation"` (every root State becomes a slide). Interactive
  stories load the action wasm on demand. Built-in font aliases are read from `fonts/`.
- **Way B — `{ script }` (EXPERIMENTAL)**: JavaScript that uses the Potoru authoring API and
  returns a `Project`. It runs in a sandboxed iframe (`sandbox="allow-scripts"`, opaque origin,
  CSP `default-src 'none'; script-src 'unsafe-inline'`: no network, no cookies, no access to the
  page), which holds only the authoring API. The iframe posts back the source folder, and Way A
  compiles it (`result.files` is that folder). A hard timeout (`timeoutMs`, default 10000) removes
  the iframe. **JavaScript only in 0.1** — TypeScript annotations are a syntax error with a hint.
  `import { Project } from "@potoru/authoring"` is accepted; `Project`, `format` and `Potoru` (the
  whole API) are in scope; `return project` at the end.

`compile` is also a global name in the browser (every `zeb/*` export is); prefer `potoru.compile`.

---

## Patterns

### React to a story's reports

```tsx
useEffect(() => {
  const host = document.getElementById("game");
  const onReport = (e) => setScore(e.detail.payload);
  host?.addEventListener("zeb:potoru:report", onReport);
  return () => host?.removeEventListener("zeb:potoru:report", onReport);
}, []);
```

### Mount without the component

```ts
import { mountPotoruPlayer } from "zeb/potoru";
const player = await mountPotoruPlayer(document.getElementById("slot"), { src: "/assets/a.poto", controls: true });
```

---

## Bundle details

| Property | Value |
|----------|-------|
| Sources | Potoru web player 0.1.0, format v4 compiler and authoring API (Apache-2.0) |
| Entry | `0.1/runtime/potoru.bundle.mjs` (bridge only) |
| On demand | `player-*.mjs` + shared `chunk-*.mjs`; `compiler-*.mjs`; `potoru-authoring-sandbox.js` (Way B); `*.wasm` (action: interactive stories; score + `score-worker.js`: music; render: path guides); `fonts/` |
| CDN fetches | **None** |
| Build tool | esbuild (code splitting); see the hand-off NOTES.md and `0.1/runtime/package.json` |
| Checksums | `FILES-SHA256.txt` |

Licences: Apache-2.0 (Potoru, `LICENSE`, `NOTICE.txt`); the Rust crates compiled into the wasm
engines and the OFL-1.1 fonts are listed with their licence texts in `THIRD-PARTY-NOTICES.txt` and
`0.1/runtime/fonts/*-OFL.txt`.
