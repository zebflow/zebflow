import { canonicalNodeKind } from "@/components/pipeline-editor/nodes/catalog";

const NUMERIC_FIELDS = new Set([
  "limit",
  "timeout_ms",
  "step_budget",
  "expires_in",
  "cost",
  "length",
]);

const ARRAY_FIELDS = new Set(["cases", "branches"]);

const JSON_FIELDS = new Set(["claims"]);

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
      next[key] = String(value || "")
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
      continue;
    }

    // JSON fields
    if (JSON_FIELDS.has(key)) {
      try {
        next[key] = JSON.parse(String(value || "{}"));
      } catch {
        next[key] = {};
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
