// PipelineGraph.tsx — TypeScript type source for the PipelineGraph Preact wrapper.
//
// This file is NOT compiled into the bundle (the bundle is maintained manually).
// It serves as the type reference for template authors using `import { PipelineGraph }
// from "zeb/graphui"`.
//
// The runtime implementation lives in:
//   libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs (PipelineGraph export)
// SSR stub lives in:
//   src/rwe/runtime/preact_ssr_init.js (globalThis.PipelineGraph)

import { forwardRef } from "zeb";

export interface PipelineNodeData {
  graphNodeId: number;
  zfKind: string;
  zfPipelineNodeId: string;
  zfConfig: Record<string, unknown>;
  title?: string;
  x: number;
  y: number;
  inputs: { name: string }[];
  outputs: { name: string }[];
  /** Live graph node object — modify zfConfig/zfPipelineNodeId on it directly */
  _raw: unknown;
}

export interface PipelineGraphHandle {
  /** Place a new node of the given kind at the canvas center */
  addNode(
    kind: string,
    entry: {
      title?: string;
      color?: string;
      icon?: string;
      input_pins?: string[];
      output_pins?: string[];
    }
  ): void;
  /** Collect current canvas state as a zebflow pipeline JSON object */
  collectPipeline(): object;
  /** Reflow the current canvas as a left-to-right layered pipeline graph */
  autoTidy(options?: {
    baseX?: number;
    baseY?: number;
    rankGapX?: number;
    nodeGapY?: number;
  }): unknown[];
  /** Raw graphApp escape hatch for advanced imperative use */
  getApp(): unknown;
}

export interface PipelineGraphProps {
  /** Pipeline JSON to display/edit. Changing this prop reloads the scene. */
  pipeline?: object | null;
  /** Disable editing (pan/zoom still work). Default: false */
  readOnly?: boolean;
  /** Snap nodes to grid. Default: true */
  snapToGrid?: boolean;
  /** Grid cell size in pixels. Default: 30 */
  gridSize?: number;
  /** Override per-kind node colours. Merged with DEFAULT_NODE_KIND_COLORS. */
  kindColors?: Record<string, string>;
  /** Override per-kind SVG icon URLs shown in node headers. */
  kindIcons?: Record<string, string>;
  /** Called when the user clicks the "E" (edit) button on a node */
  onNodeEdit?: (node: PipelineNodeData) => void;
  /** Called once after the graphui app is created and the first scene loads */
  onReady?: (app: unknown) => void;
  className?: string;
  id?: string;
}

declare const PipelineGraph: ReturnType<
  typeof forwardRef<PipelineGraphHandle, PipelineGraphProps>
>;

export default PipelineGraph;
export { PipelineGraph };
