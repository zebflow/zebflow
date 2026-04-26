export type EditorPreferences = {
  vim: boolean;
};

const STORAGE_KEY = "zf-editor-preferences";
const EVENT_NAME = "zf:editor-preferences";

const DEFAULT_EDITOR_PREFERENCES: EditorPreferences = {
  vim: false,
};

function normalizeEditorPreferences(raw: any): EditorPreferences {
  return {
    vim: !!raw?.vim,
  };
}

function rememberEditorPreferences(prefs: EditorPreferences) {
  if (typeof window === "undefined") return prefs;
  (window as any).__zf_editor_preferences = prefs;
  return prefs;
}

export function getDefaultEditorPreferences(): EditorPreferences {
  return { ...DEFAULT_EDITOR_PREFERENCES };
}

export function readEditorPreferences(): EditorPreferences {
  if (typeof window === "undefined") {
    return getDefaultEditorPreferences();
  }

  const cached = (window as any).__zf_editor_preferences;
  if (cached) {
    return normalizeEditorPreferences(cached);
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return rememberEditorPreferences(getDefaultEditorPreferences());
    }
    return rememberEditorPreferences(normalizeEditorPreferences(JSON.parse(raw)));
  } catch (_) {
    return rememberEditorPreferences(getDefaultEditorPreferences());
  }
}

export function writeEditorPreferences(next: Partial<EditorPreferences> | EditorPreferences): EditorPreferences {
  const current = readEditorPreferences();
  const prefs = rememberEditorPreferences(normalizeEditorPreferences({ ...current, ...next }));

  if (typeof window !== "undefined") {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
    } catch (_) {}
    window.dispatchEvent(new CustomEvent(EVENT_NAME, { detail: prefs }));
  }

  return prefs;
}

export function subscribeEditorPreferences(listener: (prefs: EditorPreferences) => void) {
  if (typeof window === "undefined") {
    return () => {};
  }

  const handleCustom = (event: Event) => {
    const prefs = normalizeEditorPreferences((event as CustomEvent)?.detail);
    rememberEditorPreferences(prefs);
    listener(prefs);
  };

  const handleStorage = (event: StorageEvent) => {
    if (event.key !== STORAGE_KEY) return;
    listener(readEditorPreferences());
  };

  window.addEventListener(EVENT_NAME, handleCustom as EventListener);
  window.addEventListener("storage", handleStorage);
  return () => {
    window.removeEventListener(EVENT_NAME, handleCustom as EventListener);
    window.removeEventListener("storage", handleStorage);
  };
}

export async function prepareCodeMirrorRuntime(runtime: any) {
  const prefs = readEditorPreferences();
  if (prefs.vim && typeof runtime?.enableVimSupport === "function") {
    try {
      await runtime.enableVimSupport();
    } catch (error) {
      console.warn("[project-studio] failed enabling vim support", error);
    }
  }
  return prefs;
}
