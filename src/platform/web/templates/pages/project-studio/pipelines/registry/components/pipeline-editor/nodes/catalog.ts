import type { NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";

export const nodeCategories: Record<string, string[]> = {
  trigger: [
    "n.trigger.webhook",
    "n.trigger.mapserver",
    "n.trigger.mcp",
    "n.trigger.schedule",
    "n.trigger.manual",
    "n.trigger.ws",
    "n.trigger.memsubscribe",
    "n.trigger.weberror",
    "n.trigger.function",
  ],
  data: [
    "n.sekejap.query",
    "n.sekejap.mutate",
    "n.sqlite.query",
    "n.sqlite.mutate",
    "n.table.convert",
    "n.table.query",
    "n.pg.query",
    "n.mem.set",
    "n.mem.get",
    "n.mem.exists",
    "n.mem.del",
    "n.mem.expire",
    "n.mem.incr",
    "n.mem.publish",
  ],
  logic: [
    "n.script",
    "n.logic.if",
    "n.logic.match",
    "n.logic.collect",
    "n.logic.foreach",
    "n.logic.reduce",
    "n.logic.retry",
    "n.function.call",
    "n.ai.agent",
    "n.ai.tts",
  ],
  web: ["n.http.request", "n.browser.run", "n.web.render", "n.web.response", "n.web.static.generate", "n.web.docs.generate", "n.ws.sync_state", "n.ws.emit"],
  security: ["n.auth.token.create", "n.crypto"],
  files: ["n.fs.save", "n.fs.compress", "n.fs.decompress", "n.fs.pdf.convert", "n.fs.thumbnail"],
};

const NODE_KIND_COLORS: Record<string, string> = {
  "n.trigger.webhook": "#065f46",
  "n.trigger.mapserver": "#0f766e",
  "n.trigger.mcp": "#155e75",
  "n.trigger.schedule": "#14532d",
  "n.trigger.manual": "#166534",
  "n.trigger.weberror": "#7f1d1d",
  "n.trigger.ws": "#064e3b",
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
  "n.auth.token.create": "#78350f",
  "n.crypto": "#6b21a8",
  "n.browser.run": "#0369a1",
  "n.trigger.function": "#166534",
  "n.sekejap.mutate": "#0f766e",
  "n.function.call": "#1e40af",
  "n.web.response": "#9d174d",
  "n.fs.save": "#0c4a6e",
  "n.fs.compress": "#0c4a6e",
  "n.fs.decompress": "#0c4a6e",
  "n.fs.pdf.convert": "#0c4a6e",
  "n.fs.thumbnail": "#4a1d96",
  "n.mem.set": "#b45309",
  "n.mem.get": "#b45309",
  "n.mem.exists": "#b45309",
  "n.mem.del": "#b45309",
  "n.mem.expire": "#b45309",
  "n.mem.incr": "#b45309",
  "n.mem.publish": "#b45309",
  "n.trigger.memsubscribe": "#b45309",
};

export const NODE_KIND_ICONS: Record<string, string> = {
  "n.ai.agent": "/assets/node-icons/zebflow/n.ai.agent.svg",
  "n.auth.token.create": "/assets/node-icons/zebflow/n.auth.token.create.svg",
  "n.crypto": "/assets/node-icons/zebflow/n.crypto.svg",
  "n.fs.save": "/assets/node-icons/zebflow/n.fs.save.svg",
  "n.function.call": "/assets/node-icons/zebflow/n.function.call.svg",
  "n.http.request": "/assets/node-icons/zebflow/n.http.request.svg",
  "n.logic.collect": "/assets/node-icons/zebflow/n.logic.collect.svg",
  "n.logic.foreach": "/assets/node-icons/zebflow/n.logic.foreach.svg",
  "n.logic.if": "/assets/node-icons/zebflow/n.logic.if.svg",
  "n.logic.match": "/assets/node-icons/zebflow/n.logic.match.svg",
  "n.logic.reduce": "/assets/node-icons/zebflow/n.logic.reduce.svg",
  "n.logic.retry": "/assets/node-icons/zebflow/n.logic.retry.svg",
  "n.mem.set": "/assets/node-icons/zebflow/n.mem.set.svg",
  "n.pg.query": "/assets/node-icons/zebflow/n.pg.query.svg",
  "n.sekejap.query": "/assets/node-icons/zebflow/n.sekejap.query.svg",
  "n.table.convert": "/assets/node-icons/zebflow/n.table.convert.svg",
  "n.table.query": "/assets/node-icons/zebflow/n.table.query.svg",
  "n.script": "/assets/node-icons/zebflow/n.script.svg",
  "n.sqlite.mutate": "/assets/node-icons/zebflow/n.sqlite.mutate.svg",
  "n.sqlite.query": "/assets/node-icons/zebflow/n.sqlite.query.svg",
  "n.fs.thumbnail": "/assets/node-icons/zebflow/n.fs.thumbnail.svg",
  "n.trigger.function": "/assets/node-icons/zebflow/n.trigger.function.svg",
  "n.trigger.manual": "/assets/node-icons/zebflow/n.trigger.manual.svg",
  "n.trigger.mapserver": "/assets/node-icons/zebflow/n.trigger.mapserver.svg",
  "n.trigger.mcp": "/assets/node-icons/zebflow/n.trigger.manual.svg",
  "n.trigger.memsubscribe": "/assets/node-icons/zebflow/n.trigger.memsubscribe.svg",
  "n.trigger.schedule": "/assets/node-icons/zebflow/n.trigger.schedule.svg",
  "n.trigger.webhook": "/assets/node-icons/zebflow/n.trigger.webhook.svg",
  "n.trigger.weberror": "/assets/node-icons/zebflow/n.trigger.weberror.svg",
  "n.trigger.ws": "/assets/node-icons/zebflow/n.trigger.ws.svg",
  "n.web.docs.generate": "/assets/node-icons/zebflow/n.web.docs.generate.svg",
  "n.web.response": "/assets/node-icons/zebflow/n.web.response.svg",
  "n.web.static.generate": "/assets/node-icons/zebflow/n.web.static.generate.svg",
  "n.ws.emit": "/assets/node-icons/zebflow/n.ws.emit.svg",
  "n.ws.sync_state": "/assets/node-icons/zebflow/n.ws.sync_state.svg",
};

export function nodeColor(kind: string): string {
  return NODE_KIND_COLORS[kind] || "#334155";
}

export function nodeIcon(kind: string): string {
  return NODE_KIND_ICONS[canonicalNodeKind(kind)] || "";
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

export function categoryForNodeKind(kind: string): string {
  const canonical = canonicalNodeKind(kind);
  for (const [category, kinds] of Object.entries(nodeCategories)) {
    if ((kinds || []).includes(canonical)) return category;
  }
  if (canonical.startsWith("n.trigger.")) return "trigger";
  if (canonical.startsWith("n.logic.") || canonical.startsWith("n.function.") || canonical.startsWith("n.ai.")) return "logic";
  if (canonical.startsWith("n.fs.")) return "files";
  if (canonical.startsWith("n.auth.") || canonical.startsWith("n.crypto")) return "security";
  if (canonical.startsWith("n.web.") || canonical.startsWith("n.ws.") || canonical.startsWith("n.http.") || canonical.startsWith("n.browser.")) return "web";
  if (canonical.startsWith("n.mem.") || canonical.startsWith("n.pg.") || canonical.startsWith("n.sqlite.") || canonical.startsWith("n.sekejap.") || canonical.startsWith("n.table.")) return "data";
  return "other";
}

export function groupedCatalogEntries(catalog: Map<string, NodeCatalogEntry>): Record<string, NodeCatalogEntry[]> {
  const grouped: Record<string, NodeCatalogEntry[]> = {};
  const seen = new Set<string>();
  for (const [category, kinds] of Object.entries(nodeCategories)) {
    for (const kind of kinds || []) {
      const entry = catalog.get(kind);
      if (!entry || seen.has(entry.kind)) continue;
      (grouped[category] ||= []).push(entry);
      seen.add(entry.kind);
    }
  }
  Array.from(catalog.values())
    .filter((entry) => entry?.kind && !seen.has(entry.kind))
    .sort((a, b) => String(a.kind).localeCompare(String(b.kind)))
    .forEach((entry) => {
      const category = categoryForNodeKind(entry.kind);
      (grouped[category] ||= []).push(entry);
      seen.add(entry.kind);
    });
  return grouped;
}

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
      canonicalKind === "n.trigger.mapserver" ||
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
