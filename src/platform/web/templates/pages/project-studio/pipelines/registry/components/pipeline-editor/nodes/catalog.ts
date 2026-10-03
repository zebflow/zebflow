import type { NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";

const NODE_KIND_COLORS: Record<string, string> = {
  "trigger.webhook": "#065f46",
  "trigger.mcp": "#155e75",
  "trigger.schedule": "#14532d",
  "trigger.manual": "#166534",
  "trigger.error": "#7f1d1d",
  "trigger.room": "#064e3b",
  "trigger.socket": "#064e3b",
  "script.result.run": "#1e3a8a",
  "http.response.fetch": "#7c2d12",
  "sekejap.query.run": "#0f766e",
  "table.data.convert": "#0f766e",
  "table.query.run": "#0f766e",
  "pg.query.run": "#7c3aed",
  "n.web.render": "#be185d",
  "web.site.generate": "#c2410c",
  "ai.text.generate": "#4338ca",
  "ai.audio.generate": "#4338ca",
  "logic.if": "#0e7490",
  "logic.match": "#0e7490",
  "logic.collect": "#0e7490",
  "logic.foreach": "#0e7490",
  "logic.reduce": "#0e7490",
  "logic.retry": "#0e7490",
  "ws.state.put": "#064e3b",
  "ws.state.update": "#064e3b",
  "ws.state.delete": "#064e3b",
  "ws.message.send": "#065f46",
  "auth.token.create": "#78350f",
  "browser.page.run": "#0369a1",
  "trigger.function": "#166534",
  "function.result.call": "#1e40af",
  "web.response.send": "#9d174d",
  "ms.layer.publish": "#0f766e",
  "ms.layer.unpublish": "#0f766e",
  "ms.layer.get": "#0f766e",
  "ms.layer.list": "#0f766e",
  "fs.folder.list": "#0c4a6e",
  "fs.file.head": "#0c4a6e",
  "fs.file.get": "#0c4a6e",
  "fs.file.put": "#0c4a6e",
  "fs.file.delete": "#0c4a6e",
  "fs.file.copy": "#0c4a6e",
  "fs.file.move": "#0c4a6e",
  "fs.folder.create": "#0c4a6e",
  "fs.archive.create": "#0c4a6e",
  "fs.archive.extract": "#0c4a6e",
  "fs.pdf.convert": "#0c4a6e",
  "fs.image.thumbnail": "#4a1d96",
  "fs.image.chromakey": "#4a1d96",
  "fs.image.render": "#4a1d96",
  "fs.barcode.render": "#4a1d96",
  "kv.entry.put": "#b45309",
  "kv.entry.get": "#b45309",
  "kv.entry.head": "#b45309",
  "kv.entry.delete": "#b45309",
  "kv.entry.expire": "#b45309",
  "kv.entry.increment": "#b45309",
  "kv.message.publish": "#b45309",
  "trigger.topic": "#b45309",
  "geo.dataset.inspect": "#0f766e",
  "geo.dataset.convert": "#0f766e",
};

export function nodeColor(kind: string): string {
  if (NODE_KIND_COLORS[kind]) return NODE_KIND_COLORS[kind];
  if (kind.startsWith("crypto.")) return "#6b21a8";
  if (kind.startsWith("x.")) return "#6d28d9";
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
  return canonicalNodeKind(kind).startsWith("trigger.");
}

export function triggerKindFromNodeKind(kind: string): string {
  const canonical = canonicalNodeKind(kind);
  return isTriggerNodeKind(canonical) ? canonical.slice("trigger.".length) : "";
}

/** Fallback category derivation from node kind prefix (used when ui_category is not set). */
export function categoryForNodeKind(kind: string): string {
  const canonical = canonicalNodeKind(kind);
  if (canonical.startsWith("trigger.")) return "trigger";
  if (canonical.startsWith("logic.") || canonical.startsWith("function.") || canonical.startsWith("ai.")) return "logic";
  if (canonical.startsWith("ms.")) return "data";
  if (canonical.startsWith("fs.")) return "files";
  if (canonical.startsWith("auth.") || canonical.startsWith("crypto.")) return "security";
  if (canonical.startsWith("web.") || canonical.startsWith("ws.") || canonical.startsWith("http.") || canonical.startsWith("browser.")) return "web";
  if (canonical.startsWith("geo.") || canonical.startsWith("kv.") || canonical.startsWith("mem.") || canonical.startsWith("pg.") || canonical.startsWith("sqlite.") || canonical.startsWith("sekejap.") || canonical.startsWith("table.")) return "data";
  // Installed nodes carry their own ui_category. This fallback only runs when a
  // bundle left it empty, and the kind never encodes the implementation.
  if (canonical.startsWith("x.")) return "installed";
  if (canonical === "script.result.run" || canonical === "logic.concept") return "logic";
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
    (canonicalKind === "trigger.webhook" ||
      canonicalKind === "trigger.schedule" ||
      canonicalKind === "trigger.manual")
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
  if (canonicalKind !== "logic.match") {
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
  if (canonicalKind === "logic.match") {
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
