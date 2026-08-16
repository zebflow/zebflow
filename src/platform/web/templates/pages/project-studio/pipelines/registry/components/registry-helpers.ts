export function pipelineNavLastSegment(virtualPath) {
  const parts = String(virtualPath || "").split("/").filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "/";
}

export function expandFolderPaths(scopeFolders, editorBase) {
  const pathMap = new Map();
  for (const f of scopeFolders) {
    const vp = String(f?.virtual_path ?? "");
    if (!vp || vp === "/") continue;
    if (!pathMap.has(vp)) {
      pathMap.set(vp, { virtual_path: vp, count: 0, href: `${editorBase}?path=${vp}` });
    }
    pathMap.get(vp).count += (f?.count ?? 0);
    const parts = vp.split("/").filter(Boolean);
    for (let i = 1; i < parts.length; i++) {
      const ancestor = "/" + parts.slice(0, i).join("/");
      if (!pathMap.has(ancestor)) {
        pathMap.set(ancestor, { virtual_path: ancestor, count: 0, href: `${editorBase}?path=${ancestor}` });
      }
      pathMap.get(ancestor).count += (f?.count ?? 0);
    }
  }
  return Array.from(pathMap.values()).sort((a, b) => a.virtual_path.localeCompare(b.virtual_path));
}

export function getDirectChildFolders(allFolders, currentPath) {
  const normalized = String(currentPath || "/");
  return allFolders.filter((f) => {
    const vp = String(f?.virtual_path ?? "");
    if (vp === normalized) return false;
    const lastSlash = vp.lastIndexOf("/");
    const parent = lastSlash <= 0 ? "/" : vp.slice(0, lastSlash);
    return parent === normalized;
  });
}

export function peSanitizeSegment(raw) {
  return String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "") || "pipeline";
}

export function peNormalizeVirtualPath(raw) {
  const trimmed = String(raw || "/").trim();
  if (!trimmed || trimmed === "/") return "/";
  return `/${trimmed.replace(/^\/+|\/+$/g, "")}`;
}

export function pePipelineDocument(graph, metadata = {}) {
  return {
    apiVersion: "zebflow.com/v1",
    kind: "Pipeline",
    metadata: { ...metadata, name: graph.id },
    spec: graph,
  };
}

export function pePipelineGraph(document) {
  if (
    document?.apiVersion !== "zebflow.com/v1" ||
    document?.kind !== "Pipeline" ||
    !document?.spec ||
    typeof document.spec !== "object"
  ) {
    throw new Error("Pipeline source must use the zebflow.com/v1 Pipeline contract");
  }
  return document.spec;
}

export function peEmptyPipelineDocument(name, triggerKind) {
  const id = peSanitizeSegment(name);
  let graph;
  if (triggerKind === "schedule") {
    graph = { id, entry_nodes: ["trigger_schedule"],
      nodes: [{ id: "trigger_schedule", kind: "n.trigger.schedule", input_pins: [], output_pins: ["out"], config: { cron: "*/5 * * * *", timezone: "UTC" } }], edges: [] };
  } else if (triggerKind === "function") {
    graph = { id, entry_nodes: ["script_entry"],
      nodes: [{ id: "script_entry", kind: "n.script", input_pins: ["in"], output_pins: ["out"], config: { source: "return input;" } }], edges: [] };
  } else if (triggerKind === "manual") {
    graph = { id, entry_nodes: ["trigger_manual"],
      nodes: [{ id: "trigger_manual", kind: "n.trigger.manual", input_pins: [], output_pins: ["out"], config: {} }], edges: [] };
  } else {
    graph = { id, entry_nodes: ["trigger_webhook"],
      nodes: [{ id: "trigger_webhook", kind: "n.trigger.webhook", input_pins: [], output_pins: ["out"], config: { path: `/${id}`, method: "GET" } }], edges: [] };
  }
  return pePipelineDocument(graph);
}
