import { canonicalNodeKind } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/catalog";

const NUMERIC_FIELDS = new Set([
  "limit",
  "timeout_ms",
  "budget",
  "max_repairs",
  "expires_in",
  "cost",
  "length",
]);

const ARRAY_FIELDS = new Set(["cases", "branches"]);

const JSON_FIELDS = new Set(["claims"]);

// ── Node previews ────────────────────────────────────────────────────────────
//
// `config.preview = { in?: {as, path?, width?, height?}, out?: {…} }` —
// presentation the engine never reads, same standing as `config.ui`. The
// dialog holds the two halves flat as `formState.preview_in` /
// `formState.preview_out`; these functions are the only place that shape and
// the stored one meet. `width` / `height` are the canvas panel's size, set by
// dragging its corner; the dialog carries them through untouched and only
// drops them with the whole cell, when the kind goes off.

export const PREVIEW_KINDS = [
  "image",
  "video",
  "audio",
  "pdf",
  "json",
  "text",
  "table",
  "html",
];

export type PreviewCellValue =
  | { as: string; path?: string; width?: number; height?: number }
  | null;

/** A stored panel size: both numbers positive, else nothing. */
export function previewCellSize(raw: unknown): { width: number; height: number } | null {
  const width = Number((raw as any)?.width);
  const height = Number((raw as any)?.height);
  if (!(width > 0) || !(height > 0)) return null;
  return { width: Math.round(width), height: Math.round(height) };
}

/** Read whatever is stored into the `{as, path, width, height}` shape, or null for off. */
export function normalizePreviewCell(raw: unknown): PreviewCellValue {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const as = String((raw as any).as || "").trim().toLowerCase();
  if (!PREVIEW_KINDS.includes(as)) return null;
  const path = String((raw as any).path || "").trim();
  return { as, ...(path ? { path } : {}), ...(previewCellSize(raw) || {}) };
}

/** `config.preview` → the two flat form values. */
export function splitPreviewConfig(config: any): {
  preview_in: PreviewCellValue;
  preview_out: PreviewCellValue;
} {
  const preview = config && typeof config === "object" ? (config as any).preview : null;
  return {
    preview_in: normalizePreviewCell(preview?.in),
    preview_out: normalizePreviewCell(preview?.out),
  };
}

function slugifyPin(raw: string, fallback = "case"): string {
  const out = String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return out || fallback;
}

function normalizeMatchCase(item: unknown, index: number) {
  if (typeof item === "string") {
    const value = item.trim();
    return value ? { value, pin: slugifyPin(value, `case-${index + 1}`), label: value } : null;
  }
  const source = item && typeof item === "object" ? item as Record<string, unknown> : {};
  const value = String(source.value || "").trim();
  if (!value) return null;
  const pin = slugifyPin(String(source.pin || value), `case-${index + 1}`);
  const label = String(source.label || value).trim() || value;
  return { value, pin, label };
}

function normalizeMatchDefault(raw: unknown) {
  if (typeof raw === "string") {
    const pin = slugifyPin(raw, "default");
    return { pin, label: pin === "default" ? "Default" : pin };
  }
  const source = raw && typeof raw === "object" ? raw as Record<string, unknown> : {};
  const pin = slugifyPin(String(source.pin || "default"), "default");
  const label = String(source.label || "Default").trim() || "Default";
  return { pin, label };
}

function normalizeMatchRoutes(raw: unknown) {
  const source = raw && typeof raw === "object" && !Array.isArray(raw)
    ? raw as Record<string, unknown>
    : {};
  const cases = Array.isArray(source.cases)
    ? source.cases.map(normalizeMatchCase).filter(Boolean)
    : [];
  return {
    cases,
    default: normalizeMatchDefault(source.default),
  };
}

function deriveTemplateIdFromPath(rawPath: string): string {
  return String(rawPath || "")
    .trim()
    .replace(/^pages\//, "")
    .replace(/\.(tsx|jsx|ts|js)$/i, "")
    .replace(/[\\/]+/g, ".")
    .replace(/[^a-zA-Z0-9._-]+/g, "_");
}

/**
 * Extract a typed node config object from a form state map.
 *
 * @param kind - canonical node kind (e.g. "n.trigger.webhook")
 * @param formState - flat map of field name → raw string/boolean value
 * @returns cleaned config object ready for the node's `zfConfig`
 */
export function extractNodeConfig(
  kind: string,
  formState: Record<string, unknown>
): Record<string, unknown> {
  const values = { ...formState };

  // Fallback: unknown kind with config_json textarea
  if (values.config_json && !String(kind || "").startsWith("n.")) {
    try {
      return JSON.parse(String(values.config_json)) as Record<string, unknown>;
    } catch {
      return {};
    }
  }

  const next: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(values)) {
    // Skip internal UI fields
    if (key.startsWith("__")) continue;

    // config_json fallback for known kinds that use it
    if (key === "config_json") {
      try {
        Object.assign(next, JSON.parse(String(value || "{}")));
      } catch {
        next[key] = value;
      }
      continue;
    }

    // Preview halves fold back into one `config.preview` object; both off
    // leaves no key at all.
    if (key === "preview_in" || key === "preview_out") {
      const cell = normalizePreviewCell(value);
      if (!cell) continue;
      const slot = key === "preview_in" ? "in" : "out";
      const preview = (next.preview as Record<string, unknown>) || {};
      preview[slot] = cell;
      next.preview = preview;
      continue;
    }

    if (key === "match_routes") {
      const routes = normalizeMatchRoutes(value);
      next.cases = routes.cases;
      next.default = routes.default;
      continue;
    }

    // template_path_select → multiple derived fields
    if (key === "template_path_select") {
      const selected = String(value || "").trim();
      if (!selected) continue;
      next.template_path = selected;
      next.template_rel_path = selected;
      next.template_id = deriveTemplateIdFromPath(selected);
      continue;
    }

    // Numeric fields
    if (NUMERIC_FIELDS.has(key) && value !== "") {
      const asNum = Number(value);
      next[key] = Number.isFinite(asNum) ? asNum : value;
      continue;
    }

    // Array fields (one item per line)
    if (ARRAY_FIELDS.has(key)) {
      if (Array.isArray(value)) {
        next[key] = value
          .map((item, index) => key === "cases" ? normalizeMatchCase(item, index) : String(item || "").trim())
          .filter(Boolean);
      } else {
        next[key] = String(value || "")
          .split("\n")
          .map((s, index) => key === "cases" ? normalizeMatchCase(s, index) : s.trim())
          .filter(Boolean);
      }
      continue;
    }

    // JSON fields — value is already an object when coming from claims/kv editors
    if (JSON_FIELDS.has(key)) {
      if (typeof value === "object" && value !== null && !Array.isArray(value)) {
        next[key] = value;
      } else {
        try {
          next[key] = JSON.parse(String(value || "{}"));
        } catch {
          next[key] = {};
        }
      }
      continue;
    }

    // Skip empty strings
    if (value === "") continue;

    next[key] = value;
  }

  return next;
}

/**
 * Sanitize a raw slug string to lowercase alphanumeric + . _ -
 */
export function sanitizeSlug(raw: string): string {
  return String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "") || "node";
}

/**
 * Ensure a slug is unique among existing node slugs (excluding the node itself).
 */
export function ensureUniqueSlug(
  existingNodes: any[],
  currentNodeId: number,
  wantedRaw: string
): string {
  const wantedBase = sanitizeSlug(wantedRaw);
  if (!wantedBase) return "node";
  const used = new Set(
    (existingNodes || [])
      .filter((n) => n.id !== currentNodeId)
      .map((n) => sanitizeSlug(n.zfPipelineNodeId || ""))
      .filter((s) => s.length > 0)
  );
  let candidate = wantedBase;
  let seq = 1;
  while (used.has(candidate)) {
    candidate = `${wantedBase}-${seq}`;
    seq++;
  }
  return candidate;
}
