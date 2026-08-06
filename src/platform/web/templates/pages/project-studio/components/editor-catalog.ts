type EditorCompletionCatalog = {
  version?: string;
  class_completions?: any[];
  variants?: any[];
  components?: any[];
};

const STORE_KEY = "__zf_editor_completion_catalogs";

function emptyCatalog(): EditorCompletionCatalog {
  return {
    version: "empty",
    class_completions: [],
    variants: [],
    components: [],
  };
}

function catalogStore(): Record<string, EditorCompletionCatalog> {
  if (typeof window === "undefined") return {};
  const root = window as any;
  if (!root[STORE_KEY]) root[STORE_KEY] = {};
  return root[STORE_KEY];
}

function cacheKey(projectApiBase: string) {
  return String(projectApiBase || "").replace(/\/+$/, "") || "default";
}

function normalizeCatalog(raw: any): EditorCompletionCatalog {
  const catalog = raw?.catalog || raw || {};
  return {
    version: String(catalog.version || "project"),
    class_completions: Array.isArray(catalog.class_completions) ? catalog.class_completions : [],
    variants: Array.isArray(catalog.variants) ? catalog.variants : [],
    components: Array.isArray(catalog.components) ? catalog.components : [],
  };
}

export function getEditorCompletionCatalog(projectApiBase: string): EditorCompletionCatalog {
  const store = catalogStore();
  const key = cacheKey(projectApiBase);
  if (!store[key]) store[key] = emptyCatalog();
  return store[key];
}

export async function refreshEditorCompletionCatalog(projectApiBase: string): Promise<EditorCompletionCatalog> {
  const live = getEditorCompletionCatalog(projectApiBase);
  const base = cacheKey(projectApiBase);
  try {
    const response = await fetch(`${base}/editor/completion-catalog`, {
      headers: { Accept: "application/json" },
      cache: "no-store",
    });
    const payload = await response.json().catch(() => null);
    if (!response.ok || !payload?.ok) return live;
    const next = normalizeCatalog(payload);
    live.version = next.version;
    live.class_completions = next.class_completions;
    live.variants = next.variants;
    live.components = next.components;
  } catch (_) {}
  return live;
}

export async function loadEditorCompletionCatalog(projectApiBase: string): Promise<EditorCompletionCatalog> {
  const live = getEditorCompletionCatalog(projectApiBase);
  if ((live.class_completions || []).length > 0) return live;
  return refreshEditorCompletionCatalog(projectApiBase);
}
