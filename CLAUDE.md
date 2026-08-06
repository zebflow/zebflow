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

---

### 2. Log in and get a session cookie

```bash
curl -s --cookie-jar /tmp/zf.txt \
  -X POST http://localhost:10610/login \
  -d "identifier=superadmin&password=admin123" \
  -o /dev/null -w "HTTP %{http_code}"
# → HTTP 303  (redirect to /home — that's correct)
```

Cookie jar `/tmp/zf.txt` now contains `zebflow_session=superadmin`.

**Shortcut** — the session value is literally the owner slug, so you can skip the cookie jar:

```bash
COOKIE="zebflow_session=superadmin"
curl -H "Cookie: $COOKIE" http://localhost:10610/...
```

---

### 3. Key API paths (authenticated with cookie)

```bash
COOKIE="zebflow_session=superadmin"
BASE="http://localhost:10610"
OWNER="superadmin"
PROJECT="default"

# Project settings – RWE section
curl -H "Cookie: $COOKIE" $BASE/api/projects/$OWNER/$PROJECT/settings/rwe

# Clear template compile cache (our new endpoint)
curl -H "Cookie: $COOKIE" -X POST \
  $BASE/api/projects/$OWNER/$PROJECT/rwe/cache/clear
# → {"ok":true,"cleared":true}

# List pipelines
curl -H "Cookie: $COOKIE" \
  $BASE/api/projects/$OWNER/$PROJECT/pipelines

# List templates
curl -H "Cookie: $COOKIE" \
  $BASE/api/projects/$OWNER/$PROJECT/templates

# MCP session info (also returns the Bearer token)
curl -H "Cookie: $COOKIE" \
  $BASE/api/projects/$OWNER/$PROJECT/mcp/session
```

---

### 4. Call MCP tools directly (for testing agent paths)

```bash
# Get token from MCP session endpoint
TOKEN=$(curl -s -H "Cookie: $COOKIE" \
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
      "name":"template_write",
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

```bash
# Snapshot current page state
# Navigate first, then snapshot
```

Use `mcp__playwright__browser_navigate` → `mcp__playwright__browser_snapshot` → interact.

If browser is stuck with "already in use" error: call `browser_close` once, then retry `browser_navigate`.

---

### 6. Workflow rules

- **NEVER run `git commit`** — user commits manually
- **NEVER add `Co-Authored-By`**
- After changes: `cargo check` to verify compile, then test via curl or Playwright
- Rebuild = just restart `./dev.sh` (it kills and rebuilds automatically)
- Template cache is cleared automatically on every template save (UI and MCP). Manual button in Settings → Policy tab as fallback.

---

### 7. Testing `n.img.thumbnail` / `n.file.save` pipelines

```bash
# Register the test pipeline via DSL endpoint
cat > /tmp/test_pipeline.json << 'EOJSON'
{"dsl": "register pipelines/test/img-thumb-test -- | trigger.webhook --path /test/img-thumb --method POST | n.file.save --field photo --access private --folder test-uploads | n.img.thumbnail --width 200 --height 200 --fit cover --format jpg --quality 80 --access public --folder test-thumbs --delete-source"}
EOJSON
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d @/tmp/test_pipeline.json \
  http://localhost:10610/api/projects/superadmin/default/pipelines/dsl

# Activate
cat > /tmp/activate.json << 'EOJSON'
{"dsl": "activate pipeline pipelines/test/img-thumb-test"}
EOJSON
curl -s -b /tmp/zf.txt -X POST -H "Content-Type: application/json" \
  -d @/tmp/activate.json \
  http://localhost:10610/api/projects/superadmin/default/pipelines/dsl

# Create a test PNG (200x200 red image via python3)
python3 -c "
import struct, zlib
def create_png(w, h):
    def chunk(t, d): c=t+d; return struct.pack('>I',len(d))+c+struct.pack('>I',zlib.crc32(c)&0xffffffff)
    raw = b''.join(b'\\x00'+b'\\xff\\x00\\x00'*w for _ in range(h))
    sig = b'\\x89PNG\\r\\n\\x1a\\n'
    return sig+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')
open('/tmp/test_img.png','wb').write(create_png(200,200))
print('OK')
"

# POST the image (use direct URL to avoid shell quoting issues)
curl -s -b /tmp/zf.txt -X POST -F photo=@/tmp/test_img.png \
  http://localhost:10610/wh/superadmin/default/test/img-thumb

# Expected response includes: {"saved":{...},"thumbnail":{"width":200,"height":200,"format":"jpg",...}}
# Verify source was deleted (--delete-source) by checking the files/ dir
```

Note on DSL pipeline registration: always write JSON to a temp file and use `-d @file` to avoid shell quoting issues with `--` flags in the body.
