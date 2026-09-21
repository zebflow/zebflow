/**
 * A preview panel's size on the canvas: what a node declares in
 * `config.preview.<which>.width` / `.height`, and how a drag on the canvas
 * writes a new one back where Save Draft will find it.
 */

export type PreviewSize = { width: number; height: number };

/** A declared panel size — both numbers positive — or nothing. */
export function declaredPreviewSize(declaration: any): PreviewSize | null {
  const width = Number(declaration?.width);
  const height = Number(declaration?.height);
  if (!(width > 0) || !(height > 0)) return null;
  return { width: Math.round(width), height: Math.round(height) };
}

/**
 * The canvas reported a panel dragged to a new size. Write it where a Save
 * Draft will find it — the live graph node's `zfConfig`, which
 * `collectPipeline()` reads, exactly as a dragged node's x/y and a note's
 * geometry reach the file — and into the loaded graph's copy of the node, so
 * the next `buildPreviewData` draws the panel at that size instead of
 * snapping it back. The loaded graph is written in place on purpose: giving
 * the canvas a new `pipeline` prop reloads the scene and would throw away
 * every unsaved drag with it. Nothing goes to the server until Save Draft.
 * Answers false when the node declares no such preview.
 */
export function applyPreviewSize(
  graph: any,
  liveNodes: any[],
  nodeId: string,
  which: "in" | "out",
  size: PreviewSize
): boolean {
  const width = Math.round(Number(size?.width));
  const height = Math.round(Number(size?.height));
  if (!(width > 0) || !(height > 0)) return false;
  const write = (config: any): boolean => {
    const cell = config?.preview?.[which];
    if (!cell || typeof cell !== "object") return false;
    cell.width = width;
    cell.height = height;
    return true;
  };
  const loaded = (Array.isArray(graph?.nodes) ? graph.nodes : []).find(
    (node: any) => String(node?.id || "") === nodeId
  );
  const live = (Array.isArray(liveNodes) ? liveNodes : []).find(
    (node: any) => String(node?.zfPipelineNodeId || "") === nodeId
  );
  const wroteLoaded = write(loaded?.config);
  const wroteLive = write(live?.zfConfig);
  return wroteLoaded || wroteLive;
}
