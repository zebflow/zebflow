import type { NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";

export const nodeCategories: Record<string, string[]> = {
  trigger: [
    "n.trigger.webhook",
    "n.trigger.mapserver",
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
    "n.logic.switch",
    "n.logic.branch",
    "n.logic.merge",
    "n.function.call",
    "n.ai.agent",
  ],
  web: ["n.http.request", "n.browser.run", "n.web.render", "n.web.response", "n.web.static.generate", "n.ws.sync_state", "n.ws.emit"],
  security: ["n.auth.token.create", "n.crypto"],
  files: ["n.file.save", "n.img.thumbnail"],
};

const NODE_KIND_COLORS: Record<string, string> = {
  "n.trigger.webhook": "#065f46",
  "n.trigger.mapserver": "#0f766e",
  "n.trigger.schedule": "#14532d",
  "n.trigger.manual": "#166534",
  "n.trigger.weberror": "#7f1d1d",
  "n.trigger.ws": "#064e3b",
  "n.script": "#1e3a8a",
  "n.http.request": "#7c2d12",
  "n.sekejap.query": "#0f766e",
  "n.pg.query": "#7c3aed",
  "n.web.render": "#be185d",
  "n.web.static.generate": "#c2410c",
  "n.ai.agent": "#4338ca",
  "n.logic.if": "#0e7490",
  "n.logic.switch": "#0e7490",
  "n.logic.branch": "#0e7490",
  "n.logic.merge": "#0e7490",
  "n.ws.sync_state": "#064e3b",
  "n.ws.emit": "#065f46",
  "n.auth.token.create": "#78350f",
  "n.crypto": "#6b21a8",
  "n.browser.run": "#0369a1",
  "n.trigger.function": "#166534",
  "n.sekejap.mutate": "#0f766e",
  "n.function.call": "#1e40af",
  "n.web.response": "#9d174d",
  "n.file.save": "#0c4a6e",
  "n.img.thumbnail": "#4a1d96",
  "n.mem.set": "#b45309",
  "n.mem.get": "#b45309",
  "n.mem.exists": "#b45309",
  "n.mem.del": "#b45309",
  "n.mem.expire": "#b45309",
  "n.mem.incr": "#b45309",
  "n.mem.publish": "#b45309",
  "n.trigger.memsubscribe": "#b45309",
};

export function nodeColor(kind: string): string {
  return NODE_KIND_COLORS[kind] || "#334155";
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
        output_pins: normalizeNodePins(kind, "output", node?.output_pins, ["out"]),
      };
    }),
  };
}
