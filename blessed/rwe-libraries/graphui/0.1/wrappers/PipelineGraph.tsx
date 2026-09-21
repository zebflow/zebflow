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

import { forwardRef } from "zeb/react";

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
  /** Collect current canvas state as a zebflow pipeline JSON object (nodes, edges, notes) */
  collectPipeline(): object;
  /** Place a canvas note at the centre and open it for editing; returns the note */
  addNote(text?: string, color?: string): unknown;
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

/** One half of a node's canvas preview, already resolved by the host. */
export interface PreviewCell {
  as: "image" | "video" | "audio" | "pdf" | "json" | "text" | "table" | "html";
  status: "ok" | "none" | "error";
  /** Media URL for image/video/audio/pdf/html. */
  src?: string;
  /** Text body for json/text. */
  text?: string;
  /** Rows for table (the first 8 are drawn). */
  rows?: Record<string, unknown>[];
  /** Message shown in red when status is "error". */
  error?: string;
  /**
   * A small caption on the cell — "temporary — not saved" when `src` is the
   * host's snapshot of a file the run deleted. An image cell's `src` may be
   * a `data:image/…;base64,` URI for exactly that case.
   */
  note?: string;
  /**
   * The cell's size on the canvas, in canvas pixels, when the host has one
   * stored (`config.preview.<which>.width` / `.height`). Absent: 220 wide,
   * height by kind. Bounds 160×60 to 1200×900. The cell is always centred
   * under its node whatever the width.
   */
  width?: number;
  height?: number;
}

/** One `n.input.*` node's Run-form field, already resolved by the host. */
export interface InputWidgetSpec {
  /** The word after `input.`: text, number, boolean, json, file, files, image, audio, video. */
  kind: string;
  /** The envelope field the node declares. */
  name: string;
  /** Form label; the field name when empty. */
  label?: string;
  optional?: boolean;
  /** FileRef kinds, mimes or extensions a file must match. */
  accept?: string[];
  default?: unknown;
  min?: number | string;
  max?: number | string;
  /** Seeds the field when it is (re)built; never part of the rebuild signature. */
  value?: unknown;
  /** Highlight the field (a required value is empty). Toggles a class only. */
  invalid?: boolean;
  /** What went in on the latest run; shown instead of the field until "change". */
  result?: { text?: string; meta?: string; src?: string } | null;
  /**
   * The widget's size on the canvas, in canvas pixels, when the host has one
   * stored (`config.ui.widget`). Absent: 220 wide, as tall as its content.
   * Bounds 160×48 to 900×600. `text` and `json` widgets have a corner grip;
   * the widget is always centred under its node whatever the width.
   */
  width?: number;
  height?: number;
}

/** How one node's last (or current) run went, drawn as a badge at the box's top-right. */
export interface NodeRunStatus {
  /**
   * pending: grey dot · running: orange pulsing dot · ok: green tick · skip:
   * grey dash · fail: red cross · retry: orange ring with the attempt count
   * ("3/40") · error_routed: orange ring, tooltip "error → <to_node>". Red is
   * for `fail` only: a failure an `:error` edge consumed is a wait or a
   * handled error, never a failed node.
   */
  state: "pending" | "running" | "ok" | "skip" | "fail" | "retry" | "error_routed";
  /** Shown under the mark when known. */
  duration_ms?: number;
  /** Shown as the badge's tooltip. */
  error?: string;
  /** `retry`: the attempt this is, and the budget when known — drawn as "3/40" beside the ring. */
  attempt?: number;
  max_attempts?: number;
  /** `error_routed`: the node the `:error` edge reached, named in the tooltip. */
  to_node?: string;
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
  /** Canvas selection mode shown in the bottom-right selection control */
  selectionMode?: "normal" | "box";
  /** Called when the bottom-right selection mode control changes mode */
  onSelectionModeChange?: (mode: "normal" | "box") => void;
  /** Called when the bottom-right select-all button is clicked */
  onSelectAll?: () => void;
  /**
   * Panels drawn under node boxes, keyed by pipeline node id (the node's
   * slug). Presentation only — the canvas never reads the pipeline for it.
   * A node with no entry gets no panel.
   */
  previewData?: Record<string, { in?: PreviewCell; out?: PreviewCell }>;
  /** Called when a preview panel is clicked; the canvas itself opens nothing */
  onPreviewOpen?: (pipelineNodeId: string, which: "in" | "out") => void;
  /**
   * Called when a preview cell's corner grip is released, with the new size
   * (already clamped and grid-snapped). The canvas has resized the cell
   * live and stores nothing: the host writes it to `config.preview.<which>`
   * and passes it back in with the cell. Not called when read-only.
   */
  onPreviewResize?: (
    pipelineNodeId: string,
    which: "in" | "out",
    size: { width: number; height: number }
  ) => void;
  /**
   * Run-form fields drawn under `n.input.*` node boxes, keyed by pipeline
   * node id. Widget first, preview beneath. Values live in the host's state:
   * the canvas stores nothing and never writes the pipeline.
   */
  inputWidgets?: Record<string, InputWidgetSpec>;
  /** Called on every change of a widget: a string/boolean for value kinds, a File or File[] for file kinds */
  onInputChange?: (pipelineNodeId: string, value: unknown) => void;
  /**
   * Called when a `text` / `json` widget's corner grip is released, with the
   * new size (already clamped and grid-snapped). The canvas has resized the
   * widget live and stores nothing: the host writes it to `config.ui.widget`
   * and passes it back in with the spec. Not called when read-only.
   */
  onInputResize?: (pipelineNodeId: string, size: { width: number; height: number }) => void;
  /**
   * Run badges, keyed by pipeline node id. Presentation only: the host
   * seeds them from the latest record and updates them from the execute
   * stream. A node with no entry gets no badge.
   */
  nodeStatus?: Record<string, NodeRunStatus>;
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
