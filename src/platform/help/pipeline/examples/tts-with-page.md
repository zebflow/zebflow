# TTS API + Frontend Page

## What this builds

A minimal end-to-end text-to-speech demo:

- one JSON API endpoint: `POST /api/tts`
- one frontend page: `GET /tts-demo`
- user types text
- page calls the API
- API runs `ai.audio.generate`
- page plays the returned `audio.base64`
- nothing is written to the store (`--return inline`); `--return file`
  writes the `.wav` instead and answers its FileRef

This is the fastest real shape to try `ai.audio.generate` in a browser.

---

## Credential

Create a `tts` credential first.

Current stable secret shape for local Piper:

```json
{
  "provider": "piper",
  "model_file": "voices/narrator/narrator.onnx",
  "config_file": "voices/narrator/narrator.onnx.json"
}
```

Optional:

```json
{
  "espeak_data_dir": "runtime/espeak-ng-data"
}
```

All file paths are **Zebflow FS object paths**.

---

## Pipeline 1 — API

This endpoint accepts JSON like:

```json
{
  "text": "Halo, ini Narrator dari Zebflow.",
  "slug": "my-demo"
}
```

Graph DSL:

```zf
register tts/api --
[a] trigger.webhook --route /api/tts --method POST
[guard] logic.if --when "!!(input.webhook.body && input.webhook.body.text && String(input.webhook.body.text).trim())"
[bad]   web.response.send --status 400 --body "{{ { ok: false, error: 'text is required' } }}"
[b] javascript.script.run -- "
return {
  text: String($trigger.body.text),
  slug: String($trigger.body.slug || Date.now())
};
"
[c] ai.audio.generate --provider piper --credential narrator-tts --text "{{ input.script.text }}" --return inline
[d] web.response.send

[a] -> [guard]
[guard]:true -> [b]
[guard]:false -> [bad]
[b] -> [c]
[c] -> [d]
```

Response shape:

```json
{
  "audio": {
    "base64": "UklGRi4A...",
    "size": 186924,
    "provider": "piper",
    "format": "wav",
    "mime_type": "audio/wav",
    "sample_rate": 22050,
    "samples": 93440,
    "duration_ms": 4238
  }
}
```

With `--option lipsync=timed_words` (or `basic`, `audio_guided`,
`audio_segmented`) the answer also carries `audio.word_timings` and
`audio.lipsync` (metadata and mouth cues); without it neither key is there.

With `--return file` (the default) and `--filename "{{ 'tts-' + input.script.slug }}"`
the wav is written to the store and `audio` holds its FileRef (`ref`,
`store`, `filename`, `mime`, `kind`, `size`, `sha256`, …) beside `provider`,
`format` and the rest, instead of `base64`.

---

## Pipeline 2 — Page

This page renders the frontend shell.

Graph DSL:

```zf
[a] trigger.webhook --route /tts-demo --method GET
[b] javascript.script.run -- "
return {
  title: 'Narrator TTS Demo',
  api_url: '/api/tts',
  default_text: 'Halo, ini Narrator dari Zebflow.'
};
"
[c] web.response.send --template pages/tts-demo.tsx

[a] -> [b]
[b] -> [c]
```

---

## Template — `pages/tts-demo.tsx`

```tsx
import { useState } from "zeb/react";

export const page = {
  html: { lang: "en" },
  body: {
    className:
      "min-h-screen bg-stone-950 text-stone-100 antialiased",
  },
};

export function getPage(input) {
  return {
    head: {
      title: input?.script?.title ?? "TTS Demo",
      description: "Generate speech with ai.audio.generate and play it in the browser.",
    },
  };
}

function decodeBase64ToBlob(base64, mimeType) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return new Blob([bytes], { type: mimeType || "audio/wav" });
}

export default function Page(input) {
  const [text, setText] = useState(input?.script?.default_text ?? "");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [audioUrl, setAudioUrl] = useState("");
  const [meta, setMeta] = useState(null);

  async function handleSubmit(event) {
    event.preventDefault();
    setLoading(true);
    setError("");

    if (audioUrl) {
      URL.revokeObjectURL(audioUrl);
      setAudioUrl("");
    }

    try {
      const response = await fetch(input?.script?.api_url || "/api/tts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          text,
          slug: "demo-" + Date.now(),
        }),
      });
      const payload = await response.json();
      if (!response.ok || payload?.ok === false) {
        throw new Error(payload?.error || "TTS request failed");
      }

      const blob = decodeBase64ToBlob(
        payload?.audio?.base64,
        payload?.audio?.mime_type || "audio/wav"
      );
      const nextAudioUrl = URL.createObjectURL(blob);
      setAudioUrl(nextAudioUrl);
      setMeta(payload?.audio || null);
    } catch (err) {
      setError(String(err?.message || err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-4xl flex-col px-6 py-10">
      <div className="mb-8">
        <p className="text-xs font-semibold uppercase tracking-[0.3em] text-amber-400">
          Zebflow Demo
        </p>
        <h1 className="mt-3 text-4xl font-semibold tracking-tight">
          {input?.script?.title ?? "Narrator TTS Demo"}
        </h1>
        <p className="mt-3 max-w-2xl text-sm leading-6 text-stone-300">
          Type text, call <code className="rounded bg-stone-900 px-1 py-0.5">/api/tts</code>,
          and play the returned audio immediately.
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-[1.3fr_0.7fr]">
        <form
          onSubmit={handleSubmit}
          className="rounded-3xl border border-stone-800 bg-stone-900/70 p-5 shadow-2xl shadow-black/20"
        >
          <label className="mb-2 block text-sm font-medium text-stone-200">
            Text
          </label>
          <textarea
            value={text}
            onInput={(e) => setText(e.target.value)}
            rows={10}
            placeholder="Type something for Narrator..."
            className="w-full rounded-2xl border border-stone-700 bg-stone-950 px-4 py-3 text-sm outline-none focus:border-amber-400"
          />

          {error ? (
            <div className="mt-4 rounded-2xl border border-red-800 bg-red-950/50 px-4 py-3 text-sm text-red-200">
              {error}
            </div>
          ) : null}

          <div className="mt-5 flex items-center gap-3">
            <button
              type="submit"
              disabled={loading}
              className="inline-flex h-11 items-center justify-center rounded-2xl bg-amber-400 px-5 text-sm font-semibold text-stone-950 disabled:opacity-60"
            >
              {loading ? "Generating..." : "Generate Voice"}
            </button>
            <span className="text-xs text-stone-400">
              Returns the audio inline
            </span>
          </div>
        </form>

        <aside className="rounded-3xl border border-stone-800 bg-stone-900/70 p-5">
          <h2 className="text-sm font-semibold uppercase tracking-[0.24em] text-stone-400">
            Result
          </h2>

          {audioUrl ? (
            <div className="mt-4 space-y-4">
              <audio controls src={audioUrl} className="w-full" />
            </div>
          ) : (
            <p className="mt-4 text-sm text-stone-400">
              No audio yet. Submit the form first.
            </p>
          )}

          {meta ? (
            <div className="mt-6 rounded-2xl bg-stone-950 p-4 text-xs text-stone-300">
              <div>sample_rate: {meta.sample_rate}</div>
              <div>duration_ms: {meta.duration_ms}</div>
              <div>size: {meta.size}</div>
            </div>
          ) : null}
        </aside>
      </div>
    </main>
  );
}
```

---

## What to expect

Open:

- `/tts-demo`

Type text, click **Generate Voice**, and the page should:

1. `POST` to `/api/tts`
2. receive `audio.base64`
3. create a browser `Blob`
4. play the audio immediately

---

## Notes

- `--return inline` and `audio.base64` are right for immediate browser playback or websocket delivery.
- `--return file` keeps the `.wav`: `audio` is then its FileRef. It is private
  like every stored file; to offer it for download, expose its folder
  (`audio/`) in Studio → Files and link to it on the project's file host.
- For local Piper, the current stable requirement is just:
  - `model_file`
  - `config_file`
- `espeak_data_dir` remains supported as an override, but it is not required for the stable path.

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
