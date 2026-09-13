# Zebflow – Claude Code Guide

## Absolute Zebflow Mutation Rule

- Zebflow project resources **must be changed through Zebflow APIs, MCP tools, or platform service methods**.
- Do **not** create, edit, move, or delete project resources by writing directly into mounted project internals such as `users/{owner}/{project}/repo/`, `data/`, or `files/` unless the task is explicitly about repairing storage internals.
- Pipelines must be created or updated through `POST /api/projects/{owner}/{project}/pipelines/definition`, MCP `pipeline_register` / `pipeline_patch`, or `PlatformService.projects.upsert_pipeline_definition`.
- Templates, docs, credentials, storage objects, hub assets, and settings must use their matching API/MCP/service boundary.
- Before giving a UI link to a created resource, verify it through the matching read API, for example `pipelines/by-id` for a pipeline.
- If an agent is unsure which API/MCP/service owns a resource, it must stop and inspect the platform route/service first. Guessing by filesystem layout is a bug.

## Absolute UI Rule

- For Zebflow UI work, use **Zeb React** and **Zeb Tailwind** only.
- Do **not** bypass Zeb/RWE behavior with page-local hacks, DOM workarounds, `globalThis` hook aliases, or non-standard fallback patterns.
- If even one strange behavior appears in a Zeb/RWE flow that should normally work, treat it as a **foundational RWE problem**.
- When that happens: **report it immediately, stop the workaround path, isolate the root cause, and fix the foundation first**.

## Absolute File Size and Modularity Rule

Platform `.tsx` / `.ts` files under `src/platform/web/templates/` must stay
small enough to read in one sitting. A file nobody can hold in their head is a
file nobody can safely change.

- **Hard limit: 400 lines.** A file over that is a defect to be split, not a
  style preference. Splitting is part of the change that would exceed it, not
  a later cleanup.
- **One component per file** once it passes ~150 lines. Small helpers may share
  a file with the component that uses them; two components of substance may not.
- **A helper written twice is a helper in the wrong place.** Lift it to the
  nearest folder both callers share:
  - shared by siblings → that folder's `components/`
  - shared platform-wide → `templates/components/lib/` (pure functions) or
    `templates/components/ui/` (components)
  - Then update **every** caller. Leaving one copy behind is how they drift.
- **Extraction seam test: if a component needs more than ~6 props, the seam is
  wrong.** Split by what the piece *owns*, not by where the JSX happens to
  break. A component taking thirty props is the parent with extra steps, and it
  will silently lose one — that exact mistake shipped `pendingMove is not
  defined` to a live page.
- **Only lift a helper that is genuinely the same.** Two functions sharing a
  name but not a rule are two functions: the hub's `slugify` strips every
  non-alphanumeric and caps at 80, while connections keeps `._-`. Merging them
  changes what a caller means.
- **A shared module puts all of its exports into every importer's bundle.** The
  compiler inlines the whole module, so an unused export that collides with a
  local name is a `SyntaxError` at render, not a warning at build.
- **A component must import every name it uses.** The same flat bundle means a
  component that calls the entry page's `setSelectedTable`, or uses `cx`
  without importing it, runs perfectly — on that page. Move the state into a
  hook, or import the component somewhere else, and it throws
  `X is not defined` in the browser with nothing failing at build time. Pass it
  as a prop or import it; never inherit it.
- **Verify after each extraction**, not at the end, in this order:
  1. `cargo test --test rwe platform_templates_parse` — under a second, and
     catches both a syntax error (with the line) and a borrowed binding.
  2. `./dev.sh` — templates are in the binary, so nothing else is real yet.
  3. Load the affected routes and grep the body for `RWE component error`.
  4. `cd tests/e2e && npm test`.

  Steps 3 and 4 exist because a page whose component threw still answers 200,
  and a page whose hydration died still serves correct server markup. Neither
  shows up in a status code.

## How to Test Changes

### 1. Start the dev server

```bash
./dev.sh
```

Kills whatever is on port 10610, then does `cargo run`. Wait ~20-40 s for the build.
Health check: `curl http://localhost:10610/health` → `{"status":"ok",...}`

Server prints on startup: `Flow: /login -> /home -> /projects/{owner}/{project}`
Build time: ~30s on first run, ~8s on incremental.

**Default credentials** (set in `dev.sh`):
- Username: `superadmin`
- Password: `admin123`
- Default project: `default`

**Platform templates are compiled into the binary.** Editing anything under
`src/platform/web/templates/` changes nothing the browser can see until
`./dev.sh` rebuilds (~26 s). Reloading the page after a template edit shows the
previous build and proves nothing. The template cache and the deno module cache
are both per-process, so the restart clears them too.

---

### 2. Log in and get a session cookie

```bash
curl -s --cookie-jar /tmp/zf.txt \
  -X POST http://localhost:10610/login \
  -d "identifier=superadmin&password=admin123" \
  -o /dev/null -w "HTTP %{http_code}"
# → HTTP 303  (redirect to /home — that's correct)
```

Cookie jar `/tmp/zf.txt` now holds `zebflow_session=<random token>`. The value is
a freshly minted opaque token, **not** the owner slug — there is no shortcut, so
always pass the jar with `-b /tmp/zf.txt`:

```bash
curl -s -b /tmp/zf.txt http://localhost:10610/...
```

---

### 3. Key API paths (authenticated with cookie)

```bash
BASE="http://localhost:10610"
OWNER="superadmin"
PROJECT="default"

# Project settings – RWE section
curl -s -b /tmp/zf.txt $BASE/api/projects/$OWNER/$PROJECT/settings/rwe

# Clear template compile cache
curl -s -b /tmp/zf.txt -X POST \
  $BASE/api/projects/$OWNER/$PROJECT/rwe/cache/clear
# → {"ok":true,"cleared":true}

# List pipelines
curl -s -b /tmp/zf.txt \
  $BASE/api/projects/$OWNER/$PROJECT/pipelines

# The repository tree. Scoped, because a sidebar opens one folder at a time:
#   (no params)            everything, however deep — the pipeline pages
#   ?path=docs&depth=1     one folder's children — what the tree asks for
#   ?fields=path           file paths only — what the quick-open palette reads
curl -s -b /tmp/zf.txt \
  "$BASE/api/projects/$OWNER/$PROJECT/repo?path=pipelines&depth=1"
# → {"default_file":null,"path":"pipelines","items":[...]}

# MCP session info (also returns the Bearer token)
curl -s -b /tmp/zf.txt \
  $BASE/api/projects/$OWNER/$PROJECT/mcp/session
```

**UI routes**, which are not guessable from the template paths. Every one below
was checked against a running server:

```
/home                     /hub                    /dev/design-system
/projects/{o}/{p}                      dashboard
/projects/{o}/{p}/files                /projects/{o}/{p}/hub
/projects/{o}/{p}/credentials          /projects/{o}/{p}/infrastructure
/projects/{o}/{p}/editor

/projects/{o}/{p}/pipelines/registry   also: /webhooks /schedules /manual /functions
                                       (there is no bare /pipelines — it 404s)

/projects/{o}/{p}/db/connections       the database index
/projects/{o}/{p}/db/{kind}/{slug}/tables    also: /query /schema /graph /mart
                                             /maintenance

/projects/{o}/{p}/settings             also: /policy /logs /automatons
                                       /libraries /dependencies /nodes /files
```

Connection slugs differ per machine — read them rather than assuming:

```bash
curl -s -b /tmp/zf.txt \
  $BASE/projects/$OWNER/$PROJECT/db/connections \
  | grep -oE '/db/[a-z]+/[A-Za-z0-9._-]+/tables' | sort -u
```

---

### 4. Call MCP tools directly (for testing agent paths)

```bash
# Get token from MCP session endpoint
TOKEN=$(curl -s -b /tmp/zf.txt \
  $BASE/api/projects/$OWNER/$PROJECT/mcp/session \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['session']['token'])")

MCP_URL="$BASE/api/projects/$OWNER/$PROJECT/mcp"

# Call any MCP tool
curl -s -X POST "$MCP_URL" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{
    "jsonrpc":"2.0","id":1,"method":"tools/call",
    "params":{
      "name":"file_write",
      "arguments":{
        "rel_path":"components/my-component.tsx",
        "content":"export default function Foo() { return <div>hello</div>; }"
      }
    }
  }'
```

**IMPORTANT**: MCP requires `Accept: application/json, text/event-stream` — without it you get 406.

---

### 5. Playwright (browser testing)

**The UI smoke suite lives in `tests/e2e/`** — run it after any Studio or RWE
change:

```bash
cd tests/e2e && npm test
```

It adopts a dev server already on port 10610, or starts one itself. See
`tests/e2e/README.md` for what it covers and how to add a spec. Cargo only
compiles `tests/*.rs` at the top level, so this directory is invisible to
`cargo test` and has to be run by hand.

Every spec fails on a console error or an uncaught exception, which is what
makes it worth running: a page whose hydration died still answers 200.

For ad-hoc poking, use `mcp__playwright__browser_navigate` →
`mcp__playwright__browser_snapshot` → interact.

**Never trust a click that reports success.** Zebflow hydrates after the server
HTML arrives, so a click landing before the handler attaches is swallowed
silently — and a wedged browser session can report successful clicks while
delivering zero DOM events. Always assert the state the click should produce,
and if clicks stop working, `browser_close` and re-navigate before concluding
anything about the app.

If browser is stuck with "already in use" error: call `browser_close` once, then retry `browser_navigate`.

---

### 6. Workflow rules

- **NEVER run `git commit`** — user commits manually
- **NEVER add `Co-Authored-By`**
- After changes: `cargo check` to verify compile, then test via curl or Playwright
- Rebuild = just restart `./dev.sh` (it kills and rebuilds automatically)
- **A 200 is not a rendered page.** A component that throws is replaced in the
  server markup by `<!-- RWE component error: ... -->` and the response is still
  200. Grep the body for `RWE component error` before believing a page works.
- **Check templates before rebuilding.** Two tests read the template tree
  straight off disk and answer in well under a second, instead of costing a
  26 s build to find out:

  ```bash
  cargo test --test rwe platform_templates_parse
  ```

  - `every_platform_template_parses` — reports every unparseable file at once,
    with the line, rather than the server panicking on the first one it hits.
  - `no_template_borrows_a_binding_it_never_declared` — the compiler inlines
    component files into one flat bundle, so a component using a binding it
    never imported runs on the entry page's copy. It works until the page stops
    holding that binding, then throws `X is not defined` in the browser with
    nothing failing at build time. This test uses oxc's own scope analysis to
    refuse it.
- Template cache is cleared automatically on every template save (UI and MCP). Manual button in Settings → Policy tab as fallback.

---

### 7. Testing an upload → thumbnail pipeline

The nodes are **`n.fs.save`** and **`n.fs.thumbnail`**. (`n.file.save` and
`n.img.thumbnail` do not exist — nothing under `n.img.` or `n.file.` does.)
Neither takes an `--access` flag; visibility is not a node setting.

`n.fs.save` — `--field` (multipart field, default `file`), `--path` (exact
object path; otherwise folder + generated name), `--folder` (default
`uploads`), `--allowed-kinds` (default `images`), `--max-size` (MB, default
10), `--filename`.

`n.fs.thumbnail` — `--width` / `--height` (default 256), `--fit`
(cover|contain|fill), `--format` (jpg|png|webp), `--quality` (1–100, default
82), `--folder` (default `thumbnails`), `--source-key` (dot-path to the source
in the payload, default `saved.path`), `--delete-source`, `--filename`.

```bash
# Register. Always write the JSON to a file and use -d @file: the DSL is full
# of `--flags` and shell quoting mangles them.
cat > /tmp/reg.json << 'EOJSON'
{"dsl": "register pipelines/test/fs-thumb-check -- | trigger.webhook --path /test/fs-thumb --method POST | n.fs.save --field photo --folder test-uploads | n.fs.thumbnail --width 200 --height 200 --fit cover --format jpg --quality 80 --folder test-thumbs --delete-source"}
EOJSON
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d @/tmp/reg.json \
  http://localhost:10610/api/projects/superadmin/default/pipelines/dsl

# Activate
cat > /tmp/act.json << 'EOJSON'
{"dsl": "activate pipeline pipelines/test/fs-thumb-check"}
EOJSON
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d @/tmp/act.json \
  http://localhost:10610/api/projects/superadmin/default/pipelines/dsl

# A 200x200 red PNG to send
python3 -c "
import struct, zlib
def png(w, h):
    def chunk(t, d):
        c = t + d
        return struct.pack('>I', len(d)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    raw = b''.join(b'\x00' + b'\xff\x00\x00' * w for _ in range(h))
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))
open('/tmp/test_img.png', 'wb').write(png(200, 200))
"

# Post it
curl -s -b /tmp/zf.txt -X POST -F photo=@/tmp/test_img.png \
  http://localhost:10610/wh/superadmin/default/test/fs-thumb
```

The answer is the request payload (`body`, `files`, …) plus `thumbnail`, a
FileRef; with `--delete-source` there is **no** `saved` key — the source is
gone, so its key is dropped. The FileRef:

```json
{"thumbnail":{"__zf_type":"file_ref","backend":"zebfs",
  "ref":"test-thumbs/<uuid>.jpg","filename":"<uuid>.jpg","mime":"image/jpeg",
  "kind":"image","size":1723,"sha256":"sha256:...","lifecycle":"durable",
  "origin":"fs.thumbnail","trust":"sanitized",
  "width":200,"height":200,"format":"jpg"}}
```

Clean up after yourself — a test pipeline left active is a live webhook:

```bash
cat > /tmp/del.json << 'EOJSON'
{"dsl": "deactivate pipeline pipelines/test/fs-thumb-check"}
EOJSON
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d @/tmp/del.json \
  http://localhost:10610/api/projects/superadmin/default/pipelines/dsl
# Deleting goes through the pipelines API, not the repo file API — the repo
# route is /repo/file with the path in the query, and a pipeline is owned by
# the pipeline boundary either way.
curl -s -b /tmp/zf.txt -X DELETE -H "Content-Type: application/json" \
  -d '{"file_rel_path":"pipelines/test/fs-thumb-check.zf.json"}' \
  http://localhost:10610/api/projects/superadmin/default/pipelines/definition
# → {"ok":true}
```
