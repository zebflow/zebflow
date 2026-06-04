import type { NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";

const NODE_KIND_COLORS: Record<string, string> = {
  "n.trigger.webhook": "#065f46",
  "n.trigger.mcp": "#155e75",
  "n.trigger.schedule": "#14532d",
  "n.trigger.manual": "#166534",
  "n.trigger.weberror": "#7f1d1d",
  "n.trigger.ws": "#064e3b",
  "n.trigger.ws.client": "#064e3b",
  "n.script": "#1e3a8a",
  "n.http.request": "#7c2d12",
  "n.sekejap.query": "#0f766e",
  "n.table.convert": "#0f766e",
  "n.table.query": "#0f766e",
  "n.pg.query": "#7c3aed",
  "n.web.render": "#be185d",
  "n.web.static.generate": "#c2410c",
  "n.web.docs.generate": "#c2410c",
  "n.ai.agent": "#4338ca",
  "n.ai.tts": "#4338ca",
  "n.logic.if": "#0e7490",
  "n.logic.match": "#0e7490",
  "n.logic.collect": "#0e7490",
  "n.logic.foreach": "#0e7490",
  "n.logic.reduce": "#0e7490",
  "n.logic.retry": "#0e7490",
  "n.ws.sync_state": "#064e3b",
  "n.ws.emit": "#065f46",
  "n.ws.client.send": "#064e3b",
  "n.auth.token.create": "#78350f",
  "n.crypto": "#6b21a8",
  "n.browser.run": "#0369a1",
  "n.trigger.function": "#166534",
  "n.sekejap.mutate": "#0f766e",
  "n.function.call": "#1e40af",
  "n.web.response": "#9d174d",
  "n.ms.publish": "#0f766e",
  "n.ms.unpublish": "#0f766e",
  "n.ms.get": "#0f766e",
  "n.ms.list": "#0f766e",
  "n.fs.list": "#0c4a6e",
  "n.fs.head": "#0c4a6e",
  "n.fs.get": "#0c4a6e",
  "n.fs.put": "#0c4a6e",
  "n.fs.delete": "#0c4a6e",
  "n.fs.copy": "#0c4a6e",
  "n.fs.move": "#0c4a6e",
  "n.fs.mkdir": "#0c4a6e",
  "n.fs.save": "#0c4a6e",
  "n.fs.compress": "#0c4a6e",
  "n.fs.decompress": "#0c4a6e",
  "n.fs.pdf.convert": "#0c4a6e",
  "n.fs.thumbnail": "#4a1d96",
  "n.kv.set": "#b45309",
  "n.kv.get": "#b45309",
  "n.kv.exists": "#b45309",
  "n.kv.del": "#b45309",
  "n.kv.expire": "#b45309",
  "n.kv.incr": "#b45309",
  "n.kv.publish": "#b45309",
  "n.trigger.kv.subscribe": "#b45309",
};

export function nodeColor(kind: string): string {
  if (NODE_KIND_COLORS[kind]) return NODE_KIND_COLORS[kind];
  if (kind.startsWith("n.c.")) return "#6d28d9";
  if (kind.startsWith("n.wasm.")) return "#dc2626";
  return "#334155";
}

export function canonicalNodeKind(kind: string): string {
  const raw = String(kind || "").trim();
  if (raw.startsWith("x.n.")) {
    return `n.${raw.slice("x.n.".length)}`;
  }
  return raw;
}

export function isTriggerNodeKind(kind: string): boolean {
  return canonicalNodeKind(kind).startsWith("n.trigger.");
}

export function triggerKindFromNodeKind(kind: string): string {
  const canonical = canonicalNodeKind(kind);
  return isTriggerNodeKind(canonical) ? canonical.slice("n.trigger.".length) : "";
}

/** Fallback category derivation from node kind prefix (used when ui_category is not set). */
export function categoryForNodeKind(kind: string): string {
  const canonical = canonicalNodeKind(kind);
  if (canonical.startsWith("n.trigger.")) return "trigger";
  if (canonical.startsWith("n.logic.") || canonical.startsWith("n.function.") || canonical.startsWith("n.ai.")) return "logic";
  if (canonical.startsWith("n.ms.")) return "data";
  if (canonical.startsWith("n.fs.")) return "files";
  if (canonical.startsWith("n.auth.") || canonical.startsWith("n.crypto")) return "security";
  if (canonical.startsWith("n.web.") || canonical.startsWith("n.ws.") || canonical.startsWith("n.http.") || canonical.startsWith("n.browser.")) return "web";
  if (canonical.startsWith("n.kv.") || canonical.startsWith("n.mem.") || canonical.startsWith("n.pg.") || canonical.startsWith("n.sqlite.") || canonical.startsWith("n.sekejap.") || canonical.startsWith("n.table.")) return "data";
  if (canonical.startsWith("n.c.")) return "composite";
  if (canonical.startsWith("n.wasm.")) return "wasm";
  if (canonical === "n.script") return "logic";
  return "other";
}

// ── Backend-driven catalog grouping ──────────────────────────────────────────

export interface CategoryGroup {
  subcategory: string;
  label: string;
  entries: NodeCatalogEntry[];
}

/** Groups catalog entries by root category with subcategory structure, driven by backend `ui_category`. */
export function groupedCatalogEntries(catalog: Map<string, NodeCatalogEntry>): Record<string, CategoryGroup[]> {
  const byRoot: Record<string, Map<string, { label: string; entries: NodeCatalogEntry[] }>> = {};

  for (const entry of catalog.values()) {
    if (!entry?.kind) continue;
    const uiCat = entry.ui_category || categoryForNodeKind(entry.kind);
    const dotIdx = uiCat.indexOf(".");
    const root = dotIdx > 0 ? uiCat.slice(0, dotIdx) : uiCat;
    const sub = dotIdx > 0 ? uiCat.slice(dotIdx + 1) : "";
    const label = entry.ui_category_label || (sub ? sub.charAt(0).toUpperCase() + sub.slice(1) : "");

    if (!byRoot[root]) byRoot[root] = new Map();
    const subMap = byRoot[root];
    if (!subMap.has(sub)) subMap.set(sub, { label, entries: [] });
    subMap.get(sub)!.entries.push(entry);
  }

  const result: Record<string, CategoryGroup[]> = {};
  for (const [root, subMap] of Object.entries(byRoot)) {
    result[root] = Array.from(subMap.entries()).map(([sub, group]) => ({
      subcategory: sub,
      label: group.label,
      entries: group.entries,
    }));
  }
  return result;
}

/** Builds the kindTitles map from catalog entries (kind → definition title). */
export function buildKindTitles(catalog: Map<string, NodeCatalogEntry>): Record<string, string> {
  const titles: Record<string, string> = {};
  for (const [kind, entry] of catalog.entries()) {
    if (entry.title) titles[kind] = entry.title;
  }
  return titles;
}

/** Builds the kindIcons map from catalog entries (icon_url + icon_hash for cache-busting). */
export function buildKindIcons(catalog: Map<string, NodeCatalogEntry>): Record<string, string> {
  const icons: Record<string, string> = {};
  for (const [kind, entry] of catalog.entries()) {
    if (entry.icon_url) {
      icons[kind] = entry.icon_hash ? `${entry.icon_url}?h=${entry.icon_hash}` : entry.icon_url;
    }
  }
  return icons;
}

// ── Catalog builder ──────────────────────────────────────────────────────────

export function buildNodeCatalog(items: any[]): Map<string, NodeCatalogEntry> {
  const map = new Map<string, NodeCatalogEntry>();
  (Array.isArray(items) ? items : []).forEach((item) => {
    if (!item || !item.kind) return;
    const entry: NodeCatalogEntry = {
      ...(item as NodeCatalogEntry),
      fields: Array.isArray(item.fields) ? item.fields : undefined,
    };
    map.set(item.kind, entry);
  });
  return map;
}

// ── Pin normalization ────────────────────────────────────────────────────────

export function normalizeNodePins(
  kind: string,
  pinRole: "input" | "output",
  rawPins: string[],
  fallback: string[] = []
): string[] {
  const canonicalKind = canonicalNodeKind(kind);
  if (pinRole === "output" && canonicalKind === "n.web.render") return [];
  if (
    pinRole === "input" &&
    (canonicalKind === "n.trigger.webhook" ||
      canonicalKind === "n.trigger.schedule" ||
      canonicalKind === "n.trigger.manual")
  ) {
    return [];
  }
  const pins = Array.isArray(rawPins)
    ? rawPins.map((p) => String(p || "").trim()).filter((p) => p.length > 0)
    : [];
  return pins.length > 0 ? pins : fallback.slice();
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

export function normalizeMatchCases(rawCases: unknown): { value: string; pin: string; label: string }[] {
  if (typeof rawCases === "string") {
    return rawCases
      .split("\n")
      .map((line, index) => normalizeMatchCase(line, index))
      .filter(Boolean) as { value: string; pin: string; label: string }[];
  }
  if (!Array.isArray(rawCases)) return [];
  return rawCases
    .map((item, index) => normalizeMatchCase(item, index))
    .filter(Boolean) as { value: string; pin: string; label: string }[];
}

function normalizeMatchCase(item: unknown, index: number) {
  if (typeof item === "string") {
    const value = item.trim();
    return value ? { value, pin: slugifyPin(value, `case-${index + 1}`), label: value } : null;
  }
  const source = item && typeof item === "object" ? item as Record<string, unknown> : {};
  const value = String(source.value || "").trim();
  if (!value) return null;
  return {
    value,
    pin: slugifyPin(String(source.pin || value), `case-${index + 1}`),
    label: String(source.label || value).trim() || value,
  };
}

export function normalizeMatchDefault(rawDefault: unknown): { pin: string; label: string } {
  if (typeof rawDefault === "string") {
    const pin = slugifyPin(rawDefault, "default");
    return { pin, label: pin === "default" ? "Default" : pin };
  }
  const source = rawDefault && typeof rawDefault === "object"
    ? rawDefault as Record<string, unknown>
    : {};
  return {
    pin: slugifyPin(String(source.pin || "default"), "default"),
    label: String(source.label || "Default").trim() || "Default",
  };
}

export function deriveNodeOutputPins(
  kind: string,
  config: Record<string, unknown> = {},
  rawPins: string[] = [],
  fallback: string[] = []
): string[] {
  const canonicalKind = canonicalNodeKind(kind);
  if (canonicalKind !== "n.logic.match") {
    return normalizeNodePins(canonicalKind, "output", rawPins, fallback);
  }
  const cases = normalizeMatchCases(config?.cases);
  const defaultRoute = normalizeMatchDefault(config?.default);
  const pins: string[] = [];
  for (const item of cases) {
    if (!pins.includes(item.pin)) pins.push(item.pin);
  }
  if (!pins.includes(defaultRoute.pin)) pins.push(defaultRoute.pin);
  return pins.length > 0 ? pins : normalizeNodePins(canonicalKind, "output", rawPins, ["default"]);
}

export function deriveNodeOutputLabels(
  kind: string,
  config: Record<string, unknown> = {},
  outputPins: string[] = []
): Record<string, string> {
  const canonicalKind = canonicalNodeKind(kind);
  const labels: Record<string, string> = {};
  if (canonicalKind === "n.logic.match") {
    normalizeMatchCases(config?.cases).forEach((item) => {
      labels[item.pin] = item.label || item.value || item.pin;
    });
    const defaultRoute = normalizeMatchDefault(config?.default);
    labels[defaultRoute.pin] = defaultRoute.label || defaultRoute.pin;
  }
  outputPins.forEach((pin) => {
    if (!labels[pin]) labels[pin] = pin;
  });
  return labels;
}

export function normalizeGraphForEditor(graph: any): any {
  const source = graph && typeof graph === "object" ? graph : {};
  const nodes = Array.isArray(source.nodes) ? source.nodes : [];
  return {
    ...source,
    nodes: nodes.map((node: any) => {
      const kind = canonicalNodeKind(node?.kind);
      return {
        ...node,
        kind,
        input_pins: normalizeNodePins(kind, "input", node?.input_pins, ["in"]),
        output_pins: deriveNodeOutputPins(kind, node?.config || {}, node?.output_pins, ["out"]),
      };
    }),
  };
}
