# RWE Adapter Guide

This guide explains how non-Rust stacks can use Zebflow RWE as a service.

## Core Idea

Expose Zebflow RWE behind HTTP/IPC and exchange JSON payloads using:

- `CompileTemplateRequest` / `CompileTemplateResponse`
- `RenderTemplateRequest` / `RenderTemplateResponse`
- `ProtocolError`

All contracts are in `src/rwe/protocol.rs`.

## Minimal API Shape

1. `POST /rwe/compile`
   - request: `CompileTemplateRequest`
   - response: `CompileTemplateResponse`

2. `POST /rwe/render`
   - request: `RenderTemplateRequest`
   - response: `RenderTemplateResponse`

## FastAPI Example (Client Side)

```python
from fastapi import FastAPI
import httpx

app = FastAPI()
RWE_HOST = "http://127.0.0.1:8787"

@app.post("/preview")
async def preview():
    compile_req = {
        "meta": {
            "version": "rwe.v1",
            "rwe_engine": "rwe.noop",
            "language_engine": "language.deno_sandbox",
        },
        "template": {
            "id": "page.preview",
            "source_path": None,
            "markup": "<template><html><body><h1>{{input.title}}</h1></body></html></template>",
        },
        "options": {},
    }

    async with httpx.AsyncClient(timeout=10) as client:
        c = await client.post(f"{RWE_HOST}/rwe/compile", json=compile_req)
        c.raise_for_status()
        compiled = c.json()["compiled"]

        render_req = {
            "meta": compile_req["meta"],
            "compiled": compiled,
            "state": {"title": "FastAPI + Zebflow RWE"},
            "ctx": {"route": "/preview", "request_id": "req-1", "metadata": {}},
        }
        r = await client.post(f"{RWE_HOST}/rwe/render", json=render_req)
        r.raise_for_status()
        return r.json()["output"]["html"]
```

## Why This Works

1. protocol is versioned (`rwe.v1`)
2. compile and render are decoupled
3. external framework can cache compiled artifacts
4. RWE internals stay Rust-native while integration stays language-agnostic
