export type StudioClipboardPayload = {
  kind: string;
  text?: string;
  source?: string;
  ts?: number;
  data?: any;
};

type StudioClipboardApi = {
  readText: () => Promise<string>;
  writeText: (text: string, meta?: Partial<StudioClipboardPayload>) => Promise<StudioClipboardPayload>;
  read: () => StudioClipboardPayload | null;
  write: (payload: StudioClipboardPayload) => Promise<StudioClipboardPayload>;
};

const CLIPBOARD_EVENT = "zf:clipboard";

function normalizePayload(payload: Partial<StudioClipboardPayload> | null | undefined): StudioClipboardPayload {
  return {
    kind: String(payload?.kind || "text/plain"),
    text: typeof payload?.text === "string" ? payload.text : "",
    source: typeof payload?.source === "string" ? payload.source : "studio",
    ts: typeof payload?.ts === "number" ? payload.ts : Date.now(),
    data: payload?.data,
  };
}

async function writeSystemClipboard(text: string) {
  if (typeof navigator === "undefined" || !navigator.clipboard?.writeText) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch (_) {}
}

async function readSystemClipboard(): Promise<string> {
  if (typeof navigator === "undefined" || !navigator.clipboard?.readText) return "";
  try {
    return await navigator.clipboard.readText();
  } catch (_) {
    return "";
  }
}

export function ensureStudioClipboard(): StudioClipboardApi | null {
  if (typeof window === "undefined") return null;

  const existing = (window as any).__zf_clipboard as StudioClipboardApi | undefined;
  if (existing) return existing;

  let current: StudioClipboardPayload | null = null;

  const api: StudioClipboardApi = {
    async readText() {
      const system = await readSystemClipboard();
      if (system) return system;
      return current?.text || "";
    },
    async writeText(text: string, meta: Partial<StudioClipboardPayload> = {}) {
      const payload = normalizePayload({
        kind: "text/plain",
        text,
        ...meta,
      });
      current = payload;
      await writeSystemClipboard(payload.text || "");
      window.dispatchEvent(new CustomEvent(CLIPBOARD_EVENT, { detail: payload }));
      return payload;
    },
    read() {
      return current;
    },
    async write(payload: StudioClipboardPayload) {
      const normalized = normalizePayload(payload);
      current = normalized;
      if (normalized.kind === "text/plain") {
        await writeSystemClipboard(normalized.text || "");
      }
      window.dispatchEvent(new CustomEvent(CLIPBOARD_EVENT, { detail: normalized }));
      return normalized;
    },
  };

  (window as any).__zf_clipboard = api;
  return api;
}
