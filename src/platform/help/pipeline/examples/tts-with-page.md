# TTS API + Frontend Page

## What this builds

A minimal end-to-end text-to-speech demo:

- one JSON API endpoint: `POST /api/tts`
- one frontend page: `GET /tts-demo`
- user types text
- page calls the API
- API runs `ai.tts`
- page plays the returned `audio_blob_base64`
- generated `.wav` is also persisted under project `files/private/`

This is the fastest real shape to try `n.ai.tts` in a browser.

---

## Credential

Create a `tts` credential first.

Current stable secret shape for local Piper:

```json
{
  "provider": "piper",
  "model_file": "voices/arin/arin-2449.onnx",
  "config_file": "voices/arin/arin-2449.onnx.json"
}
```

Optional:

```json
{
  "espeak_data_dir": "runtime/espeak-ng-data"
}
```

All file paths are **project `files/private/` relative**.

---

## Pipeline 1 — API

This endpoint accepts JSON like:

```json
{
  "text": "Halo, ini Arin dari Zebflow.",
  "slug": "my-demo"
}
```

Graph DSL:

```zf
[a] trigger.webhook --path /api/tts --method POST
[b] script -- "
if (!input.text || !String(input.text).trim()) {
  return { __status: 400, ok: false, error: 'text is required' };
}
return {
  text: String(input.text),
  slug: String(input.slug || Date.now())
};
"
[c] ai.tts --provider piper --credential arin-tts --text-expr "$input.text" --output-path-expr "'audio/tts-' + $input.slug + '.wav'" --return both
[d] web.response

[a] -> [b]
[b] -> [c]
[c] -> [d]
```

Response shape:

```json
{
  "audio": {
    "provider": "piper",
    "format": "wav",
    "mime_type": "audio/wav",
    "path": "private/audio/tts-my-demo.wav",
    "url": "/files/superadmin/default/private/audio/tts-my-demo.wav",
    "sample_rate": 22050,
    "samples": 93440,
    "bytes": 186924,
    "duration_ms": 4238,
    "credential_id": "arin-tts"
  },
  "audio_blob_base64": "UklGRi4A..."
}
```

---

## Pipeline 2 — Page

This page renders the frontend shell.

Graph DSL:

```zf
[a] trigger.webhook --path /tts-demo --method GET
[b] script -- "
return {
  title: 'Arin TTS Demo',
  api_url: '/api/tts',
  default_text: 'Halo, ini Arin dari Zebflow.'
};
"
[c] web.response --template pages/tts-demo.tsx

[a] -> [b]
[b] -> [c]
```

---

## Template — `pages/tts-demo.tsx`

```tsx
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
      title: input?.title ?? "TTS Demo",
      description: "Generate speech with n.ai.tts and play it in the browser.",
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
  const [text, setText] = useState(input?.default_text ?? "");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [audioUrl, setAudioUrl] = useState("");
  const [fileUrl, setFileUrl] = useState("");
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
      const response = await fetch(input?.api_url || "/api/tts", {
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
        payload?.audio_blob_base64,
        payload?.audio?.mime_type || "audio/wav"
      );
      const nextAudioUrl = URL.createObjectURL(blob);
      setAudioUrl(nextAudioUrl);
      setFileUrl(payload?.audio?.url || "");
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
          {input?.title ?? "Arin TTS Demo"}
        </h1>
        <p className="mt-3 max-w-2xl text-sm leading-6 text-stone-300">
          Type text, call <code className="rounded bg-stone-900 px-1 py-0.5">/api/tts</code>,
          play the returned audio blob immediately, and keep the generated wav file.
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
            placeholder="Type something for Arin..."
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
              Returns file + blob
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
              {fileUrl ? (
                <a
                  href={fileUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex rounded-xl border border-stone-700 px-3 py-2 text-sm text-stone-100"
                >
                  Open generated wav
                </a>
              ) : null}
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
              <div>bytes: {meta.bytes}</div>
              <div className="truncate">path: {meta.path}</div>
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
2. receive `audio_blob_base64`
3. create a browser `Blob`
4. play the audio immediately
5. expose the generated `.wav` URL for download/debugging

---

## Notes

- `audio_blob_base64` is the right field for immediate browser playback or websocket delivery.
- `audio.url` is the right field when you want a persisted downloadable file.
- For local Piper, the current stable requirement is just:
  - `model_file`
  - `config_file`
- `espeak_data_dir` remains supported as an override, but it is not required for the stable path.
