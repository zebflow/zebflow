# zeb/potoru

Potoru 2D animation for RWE templates: `PotoPlayer` plays `.poto` stories, `PotoSnapshot` draws a
still frame of one (thumbnails, galleries, storyboards), and `PotoEditor` / `compile()` compile
Potoru source folders in the browser. The same component names ship in `@potoru/react`
(see INTERFACE.md in the hand-off). Fully offline: the player, the compiler, the wasm engines and the
built-in fonts ship inside the package; nothing is fetched from a CDN.

The entry bundle (`0.1/runtime/potoru.bundle.mjs`, about 5.6 KB) holds only the Zebflow bridge.
The player code loads when the first `PotoPlayer` mounts, the snapshot code when the first
`PotoSnapshot` mounts, and the compiler code only on the first `compile()` call or when a
`PotoEditor` mounts. A page that only plays stories never downloads the compiler; a page of
snapshots never downloads the interaction or music engines.

## Import

```tsx
import { PotoPlayer, PotoSnapshot, PotoEditor, compile } from "zeb/potoru";
// also: mountPotoPlayer, mountPotoSnapshot, mountPotoEditor, and the `potoru` namespace
```

---

## `PotoPlayer`

```tsx
<PotoPlayer id="intro" src="/assets/stories/intro.poto" controls autoplay muted loop />
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
| `still` | `boolean` | `false` | Show the frame at `time` and do not play (for many thumbnails, `PotoSnapshot` is lighter) |
| `time` | `number` | `0` | Seconds, with `still` |
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

## `PotoSnapshot` (a still frame)

```tsx
{stories.map((story) => <PotoSnapshot key={story.id} src={story.url} time={1.5} aspect="16 / 9" fit="cover" />)}
```

One moment of a story, drawn once on a canvas — for thumbnails, galleries, storyboards and
scrubbers. No controls, no sound, no interaction engine, no playback loop. It draws again when `src`
or `time` change (a re-render with new props), and when the box resizes. It never loads the action
or score engines (interactive stories are drawn as their timeline shows them at `time`); a story
with path guides still loads the render wasm.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `src` | `string` | — | URL of a `.poto` package |
| `time` | `number` | `0` | Seconds into the story's first timeline (clamped to its length) |
| `libraries` | `string[]` or `string` | — | Linked libraries |
| `camera` | `string` | package default | A story camera id, or `"stage"` |
| `fit` | `"contain"` \| `"cover"` \| `"fill"` | `"contain"` | How the stage fits the box |
| `height` / `aspect` | | story aspect ratio | Box size, as for `PotoPlayer` |
| `id`, `className` | | | As for `PotoPlayer` |

Events: `zeb:potoru:ready` `{ id, kind: "snapshot", time, duration, name, width, height, capabilities }`
after each load, `zeb:potoru:error` `{ id, code, message }`. Snapshots share the player registry:
`window.__zebPotoru.get(id)` → `{ kind: "snapshot", seek(t), load(urlOrBytes), currentTime(), duration(), canvas }`.

---

## `PotoEditor`

```tsx
<PotoEditor id="lab" yaml={sourceFiles} height="600px" />
```

A **YAML / Script** toggle, a file list, a plain editor, a diagnostics list linked to file and
line, a live preview player and Compile / Download `.poto` buttons.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `yaml` | `{ [path]: string }` | `{}` | A Potoru format v4 source folder (path → YAML text) |
| `mode` | `"yaml"` \| `"script"` | `"yaml"` | `"script"` is EXPERIMENTAL (see Script below) |
| `script` | `string` | — | The script, in Script mode |
| `height` | `string` \| `number` | `"560px"` | CSS height |
| `id` | `string` | auto | Container id for `window.__zebPotoEditor.get(id)` |
| `className` | `string` | — | Classes on the container |

Events: `zeb:potoru-editor:compiled` (`{ id, ok, poto, diagnostics, files, sizes }`) and
`zeb:potoru-editor:error` (`{ id, ok: false, diagnostics, sizes }`).

```ts
const lab = window.__zebPotoEditor.get("lab");
lab.getFiles(); lab.setFiles(files); lab.setMode("script"); const result = await lab.compile(); lab.download();
```

---

## `compile(input)` — in the browser

```ts
const result = await compile({ yaml: filesMap });              // YAML
const scripted = await compile({ script: source });            // Script (experimental)
// result: { ok, poto?: Uint8Array, diagnostics: Diagnostic[], files?, scenes?, sizes: { poto, source, files, ms } }
// Diagnostic: { code, severity, message, file, line, column?, path?, fix? }
```

- **YAML — `{ yaml }`**: a format v4 source folder as a map of path → text (`Uint8Array` for
  binaries). Validated and compiled with Potoru's own format code, exactly as the `potoru` CLI's
  `export` does (same bytes). Options: `libraries` (`Uint8Array[]` of `.potolib`), `scene`,
  `state`, `locale`, `profile: "presentation"` (every root State becomes a slide). Interactive
  stories load the action wasm on demand. Built-in font aliases are read from `fonts/`.
- **Script — `{ script, yaml? }` (EXPERIMENTAL)**: JavaScript that uses the Potoru authoring API
  and returns a `Project`. It runs in a sandboxed iframe (`sandbox="allow-scripts"`, opaque origin,
  CSP `default-src 'none'; script-src 'unsafe-inline'`: no network, no cookies, no access to the
  page), which holds only the authoring API. The iframe posts back the source folder, which is then
  compiled as YAML (`result.files` is that folder). A hard timeout (`timeoutMs`, default 10000)
  removes the iframe. **JavaScript in 0.1, TypeScript later** — type annotations are a syntax error
  with a hint. `import { Project } from "@potoru/authoring"` is accepted; `Project`, `format` and
  `Potoru` (the whole API) are in scope, and `files` is the `yaml` map passed with it;
  `return project` at the end.

`compile` is also a global name in the browser (every `zeb/*` export is); `potoru.compile` is the
same function under the namespace.

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
import { mountPotoPlayer } from "zeb/potoru";
const player = await mountPotoPlayer(document.getElementById("slot"), { src: "/assets/a.poto", controls: true });
```

---

## Bundle details

| Property | Value |
|----------|-------|
| Sources | Potoru web player 0.1.0, format v4 compiler and authoring API (Apache-2.0) |
| Entry | `0.1/runtime/potoru.bundle.mjs` (bridge only) |
| On demand | `player-*.mjs` + shared `chunk-*.mjs`; `snapshot-*.mjs`; `compiler-*.mjs`; `potoru-authoring-sandbox.js` (Script mode); `*.wasm` (action: interactive stories; score + `score-worker.js`: music; render: path guides); `fonts/` |
| CDN fetches | **None** |
| Build tool | esbuild (code splitting); see the hand-off NOTES.md and `0.1/runtime/package.json` |
| Checksums | `FILES-SHA256.txt` |

Licences: Apache-2.0 (Potoru, `LICENSE`, `NOTICE.txt`); the Rust crates compiled into the wasm
engines and the OFL-1.1 fonts are listed with their licence texts in `THIRD-PARTY-NOTICES.txt` and
`0.1/runtime/fonts/*-OFL.txt`.
