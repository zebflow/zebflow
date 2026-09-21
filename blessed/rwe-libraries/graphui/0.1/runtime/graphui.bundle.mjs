export const DEFAULT_THEME = {
  bgColor: "#121212",
  gridColor: "rgba(255, 255, 255, 0.05)",
  wireColor: "#6b7280",
  wireHover: "#ffffff",
  wireActive: "#34d399",
};

export const DEFAULT_LINK_OPTIONS = {
  animated: false,
  color: null,
  thickness: null,
  dashArray: "10 10",
  speed: "1s",
  opacity: null,
};

export const DEFAULT_NODE_KIND_COLORS = {
  "n.trigger.webhook": "#065f46",
  "n.trigger.schedule": "#14532d",
  "n.script": "#1e3a8a",
  "n.http.request": "#7c2d12",
  "n.sekejap.query": "#0f766e",
  "n.sekejap.mutate": "#0f766e",
  "n.pg.query": "#7c3aed",
  "n.web.render": "#be185d",
};

const BASE_STYLE = `
.zgu-root, .zgu-root * { box-sizing: border-box; }
.zgu-root {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--zgu-bg-color);
  color: #fff;
  font-family: system-ui, -apple-system, sans-serif;
  -webkit-user-select: none;
  user-select: none;
}
.zgu-header {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 56px;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 16px;
  border-bottom: 1px solid #333;
  background: rgba(20,20,20,0.85);
  backdrop-filter: blur(12px);
  z-index: 30;
}
.zgu-header-title {
  font-size: 14px;
  font-weight: 700;
  color: #e2e8f0;
  margin-right: 6px;
}
.zgu-scene-btn {
  border: 1px solid transparent;
  background: transparent;
  color: #a3a3a3;
  padding: 6px 12px;
  font-size: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all .2s;
}
.zgu-scene-btn:hover { background: rgba(255,255,255,.06); color: #fff; }
.zgu-scene-btn.active {
  border-color: #555;
  background: rgba(255,255,255,.1);
  color: #fff;
}
.zgu-toolbox {
  position: absolute;
  top: 76px;
  left: 16px;
  width: 260px;
  border: 1px solid #333;
  border-radius: 12px;
  background: rgba(30,30,30,0.82);
  backdrop-filter: blur(8px);
  padding: 14px;
  z-index: 20;
}
.zgu-toolbox h2 {
  margin: 0 0 8px;
  color: #e2e8f0;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: .06em;
  text-transform: uppercase;
}
.zgu-toolbox-btn {
  width: 100%;
  margin-bottom: 8px;
  text-align: left;
  border: 1px solid #444;
  border-radius: 6px;
  background: #2d2d2d;
  color: #f1f5f9;
  cursor: pointer;
  padding: 8px 10px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.zgu-toolbox-btn:hover { background: #3d3d3d; border-color: #666; }
.zgu-workspace {
  position: absolute;
  inset: 0;
  cursor: grab;
  touch-action: none;
}
.zgu-workspace:active { cursor: grabbing; }
.zgu-canvas-controls {
  position: absolute;
  right: 14px;
  bottom: 14px;
  z-index: 80;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  pointer-events: all;
}
.zgu-edge-style-control {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  border: 1px solid rgba(148,163,184,.28);
  border-radius: 8px;
  background: rgba(18,18,18,.88);
  box-shadow: 0 8px 24px rgba(0,0,0,.34);
  backdrop-filter: blur(8px);
}
.zgu-edge-style-button {
  height: 24px;
  min-width: 42px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: #94a3b8;
  font-size: 10px;
  font-weight: 700;
  line-height: 1;
  cursor: pointer;
}
.zgu-edge-style-button:hover {
  color: #e2e8f0;
  background: rgba(148,163,184,.12);
}
.zgu-edge-style-button.is-active {
  color: #06281e;
  background: var(--zgu-wire-active);
}
.zgu-selection-control {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  border: 1px solid rgba(148,163,184,.28);
  border-radius: 8px;
  background: rgba(18,18,18,.88);
  box-shadow: 0 8px 24px rgba(0,0,0,.34);
  backdrop-filter: blur(8px);
}
.zgu-selection-button {
  height: 24px;
  min-width: 28px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: #94a3b8;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  cursor: pointer;
}
.zgu-selection-button:hover {
  color: #e2e8f0;
  background: rgba(148,163,184,.12);
}
.zgu-selection-button.is-active {
  color: #06281e;
  background: var(--zgu-wire-active);
}
.zgu-selection-button svg {
  width: 15px;
  height: 15px;
}
.zgu-selection-control[hidden] {
  display: none;
}
.zgu-auto-tidy-button {
  width: 32px;
  height: 32px;
  border: 1px solid rgba(148,163,184,.28);
  border-radius: 8px;
  background: rgba(18,18,18,.88);
  color: #94a3b8;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  cursor: pointer;
  box-shadow: 0 8px 24px rgba(0,0,0,.34);
  backdrop-filter: blur(8px);
}
.zgu-auto-tidy-button:hover {
  color: #e2e8f0;
  background: rgba(148,163,184,.12);
}
.zgu-auto-tidy-button svg {
  width: 16px;
  height: 16px;
}
.zgu-auto-tidy-button[hidden] {
  display: none;
}
.zgu-grid {
  position: absolute;
  inset: 0;
  background-image: radial-gradient(var(--zgu-grid-color) 1px, transparent 1px);
  background-size: 30px 30px;
  pointer-events: none;
}
.zgu-transform {
  position: absolute;
  top: 0;
  left: 0;
  width: 0;
  height: 0;
  transform-origin: 0 0;
}
.zgu-svg {
  position: absolute;
  inset: 0;
  overflow: visible;
  pointer-events: none;
}
.zgu-wire {
  fill: none;
  stroke: var(--zgu-wire-color);
  stroke-width: 3px;
  stroke-linecap: round;
  transition: stroke-width .2s, stroke .2s;
  pointer-events: visibleStroke;
  cursor: pointer;
}
.zgu-wire:hover {
  stroke: var(--zgu-wire-hover) !important;
  stroke-width: 6px !important;
  opacity: 1 !important;
}
.zgu-wire.selected {
  stroke: var(--zgu-wire-active) !important;
  stroke-width: 6px !important;
  opacity: 1 !important;
  filter: drop-shadow(0 0 6px rgba(52, 211, 153, 0.8));
}
.zgu-wire.temp {
  stroke: var(--zgu-wire-active);
  stroke-dasharray: 6 6;
  pointer-events: none;
}
.zgu-dangling-plus {
  cursor: pointer;
  pointer-events: all;
}
.zgu-dangling-wire {
  fill: none;
  stroke: var(--zgu-wire-color);
  stroke-width: 2.5px;
  stroke-linecap: round;
  stroke-dasharray: 4 4;
  opacity: .62;
}
.zgu-dangling-circle {
  fill: #171717;
  stroke: #64748b;
  stroke-width: 1.6px;
  transition: fill .14s, stroke .14s;
}
.zgu-dangling-plus:hover .zgu-dangling-wire {
  stroke: var(--zgu-wire-active);
  opacity: .95;
}
.zgu-dangling-plus:hover .zgu-dangling-circle {
  fill: #1f2937;
  stroke: var(--zgu-wire-active);
}
.zgu-dangling-mark {
  stroke: #cbd5e1;
  stroke-width: 2px;
  stroke-linecap: round;
  pointer-events: none;
}
.zgu-dangling-plus:hover .zgu-dangling-mark { stroke: var(--zgu-wire-active); }
.zgu-wire.animated {
  animation: zgu-flow var(--zgu-anim-speed, 1s) linear infinite;
}
@keyframes zgu-flow {
  from { stroke-dashoffset: 100; }
  to { stroke-dashoffset: 0; }
}
.zgu-nodes { position: absolute; inset: 0; }
.zgu-notes { position: absolute; inset: 0; }
.zgu-node-preview {
  position: absolute;
  top: 0;
  left: 0;
  z-index: 9;
  display: flex;
  flex-direction: column;
  /* Each cell keeps its own width and sits centred on the node's centre. */
  align-items: center;
  gap: 4px;
  cursor: pointer;
}
.zgu-node-preview-cell {
  position: relative;
  flex: none;
  border: 1px solid rgba(148,163,184,.45);
  border-radius: 8px;
  background: rgba(9,9,11,.92);
  box-shadow: 0 6px 16px rgba(0,0,0,.38);
  overflow: hidden;
}
.zgu-node-preview-cell:hover { border-color: var(--zgu-wire-active); }
.zgu-node-preview-cell.resizing { border-color: var(--zgu-wire-active); }
.zgu-node-preview-resize {
  position: absolute;
  right: 3px;
  bottom: 3px;
  z-index: 3;
  width: 12px;
  height: 12px;
  cursor: nwse-resize;
  color: #94a3b8;
  border-right: 2px solid currentColor;
  border-bottom: 2px solid currentColor;
  border-radius: 0 0 3px 0;
  opacity: .45;
}
.zgu-node-preview-cell:hover .zgu-node-preview-resize { opacity: .9; }
.zgu-root.zgu-readonly .zgu-node-preview-resize { display: none; }
.zgu-node-preview-tag {
  position: absolute;
  top: 3px;
  right: 5px;
  z-index: 2;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: .06em;
  text-transform: uppercase;
  color: #94a3b8;
  background: rgba(9,9,11,.78);
  border-radius: 4px;
  padding: 0 3px;
  pointer-events: none;
}
.zgu-node-preview-note {
  position: absolute;
  left: 5px;
  bottom: 4px;
  z-index: 2;
  font-size: 9px;
  font-weight: 600;
  letter-spacing: .04em;
  color: #fcd34d;
  background: rgba(9,9,11,.78);
  border-radius: 4px;
  padding: 0 4px;
  pointer-events: none;
}
.zgu-node-preview-body {
  display: block;
  width: 100%;
  overflow: hidden;
  background: #0b0b0e;
}
.zgu-node-preview-body.empty,
.zgu-node-preview-body.error {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px 8px;
  height: auto !important;
  min-height: 34px;
  font-size: 10.5px;
  line-height: 1.25;
  text-align: center;
  word-break: break-word;
}
.zgu-node-preview-body.empty { color: #64748b; }
.zgu-node-preview-body.error { color: #fda4af; }
.zgu-node-preview-body img,
.zgu-node-preview-body video {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
  background: #0b0b0e;
}
.zgu-node-preview-body audio { width: 100%; margin-top: 28px; }
.zgu-node-preview-body iframe { width: 100%; height: 100%; border: 0; background: #fff; }
.zgu-node-preview-body pre {
  margin: 0;
  padding: 6px 8px;
  height: 100%;
  overflow: auto;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 10px;
  line-height: 1.35;
  color: #cbd5e1;
  white-space: pre-wrap;
  word-break: break-word;
}
.zgu-node-preview-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 9.5px;
  color: #cbd5e1;
  table-layout: fixed;
}
.zgu-node-preview-table th,
.zgu-node-preview-table td {
  border-bottom: 1px solid rgba(148,163,184,.18);
  padding: 2px 5px;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.zgu-node-preview-table th { color: #94a3b8; font-weight: 600; }
.zgu-node-preview-scroll { height: 100%; overflow: auto; }
.zgu-node-status {
  position: absolute;
  top: 6px;
  right: 6px;
  z-index: 4;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}
.zgu-node-status-mark {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  font-weight: 800;
  line-height: 1;
  color: #0b0b0d;
  box-shadow: 0 1px 2px rgba(0,0,0,.6);
}
.zgu-node-status[data-state="pending"] .zgu-node-status-mark { background: #64748b; }
.zgu-node-status[data-state="running"] .zgu-node-status-mark {
  background: #f59e0b;
  animation: zgu-status-pulse 1s ease-in-out infinite;
}
.zgu-node-status[data-state="ok"] .zgu-node-status-mark { background: #34d399; }
.zgu-node-status[data-state="skip"] .zgu-node-status-mark { background: #94a3b8; }
.zgu-node-status[data-state="fail"] .zgu-node-status-mark { background: #f43f5e; color: #fff; }
/* A failure an :error edge consumed: a wait (retry) or a handled error
   (error_routed). An orange ring, never red — red is for fail only. */
.zgu-node-status[data-state="retry"] .zgu-node-status-mark,
.zgu-node-status[data-state="error_routed"] .zgu-node-status-mark {
  background: transparent;
  border: 2px solid #f59e0b;
  box-sizing: border-box;
}
.zgu-node-status-count {
  display: none;
  font-size: 8px;
  line-height: 1;
  font-weight: 700;
  color: #fbbf24;
  text-shadow: 0 1px 2px rgba(0,0,0,.85);
  white-space: nowrap;
}
.zgu-node-status[data-state="retry"] .zgu-node-status-count,
.zgu-node-status[data-state="running"] .zgu-node-status-count { display: block; }
.zgu-node-status-ms {
  font-size: 8px;
  line-height: 1;
  color: #cbd5e1;
  text-shadow: 0 1px 2px rgba(0,0,0,.85);
  white-space: nowrap;
}
@keyframes zgu-status-pulse {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.35); opacity: .55; }
}
.zgu-node-input {
  position: absolute;
  top: 0;
  left: 0;
  z-index: 9;
  border: 1px solid rgba(148,163,184,.45);
  border-radius: 8px;
  background: rgba(9,9,11,.94);
  box-shadow: 0 6px 16px rgba(0,0,0,.38);
  padding: 6px 8px 7px;
  font-size: 11px;
  color: #cbd5e1;
  cursor: default;
  /* A column so a stored height (data-height) stretches the field, not
     the label; the size comes from the grip below. */
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
}
.zgu-node-input.invalid { border-color: #fb7185; box-shadow: 0 0 0 2px rgba(251,113,133,.35), 0 6px 16px rgba(0,0,0,.38); }
.zgu-node-input.resizing { border-color: var(--zgu-wire-active); }
.zgu-node-input-resize {
  position: absolute;
  right: 3px;
  bottom: 3px;
  z-index: 3;
  width: 12px;
  height: 12px;
  cursor: nwse-resize;
  color: #94a3b8;
  border-right: 2px solid currentColor;
  border-bottom: 2px solid currentColor;
  border-radius: 0 0 3px 0;
  opacity: .45;
}
.zgu-node-input:hover .zgu-node-input-resize { opacity: .9; }
.zgu-root.zgu-readonly .zgu-node-input-resize { display: none; }
.zgu-node-input-label {
  display: flex;
  align-items: baseline;
  gap: 5px;
  margin-bottom: 4px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: .04em;
  text-transform: uppercase;
  color: #94a3b8;
}
.zgu-node-input-label .optional { font-weight: 400; text-transform: none; letter-spacing: 0; color: #64748b; }
.zgu-node-input.invalid .zgu-node-input-label { color: #fda4af; }
.zgu-node-input-field { display: flex; flex: 1; min-height: 0; }
.zgu-node-input-field > * { flex: 1; min-width: 0; }
.zgu-node-input[data-mode="result"] .zgu-node-input-field { display: none; }
.zgu-node-input-result { display: none; }
.zgu-node-input[data-mode="result"] .zgu-node-input-result { display: flex; align-items: center; gap: 8px; }
.zgu-node-input input[type="text"],
.zgu-node-input input[type="number"],
.zgu-node-input textarea {
  width: 100%;
  box-sizing: border-box;
  border: 1px solid rgba(148,163,184,.35);
  border-radius: 6px;
  background: #0b0b0e;
  color: #e2e8f0;
  font: inherit;
  font-size: 11px;
  padding: 4px 6px;
  outline: none;
}
/* No native resize handle: the widget's own grip sizes the whole cell. */
.zgu-node-input textarea { resize: none; min-height: 28px; height: 100%; display: block; }
.zgu-node-input textarea[data-kind="json"] { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 10px; }
.zgu-node-input input:focus, .zgu-node-input textarea:focus { border-color: var(--zgu-wire-active); }
.zgu-node-input-check { display: flex; align-items: center; gap: 6px; color: #e2e8f0; cursor: pointer; }
.zgu-node-input-drop {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  min-height: 48px;
  padding: 6px;
  border: 1px dashed rgba(148,163,184,.5);
  border-radius: 6px;
  background: #0b0b0e;
  color: #94a3b8;
  text-align: center;
  cursor: pointer;
}
.zgu-node-input-drop.over { border-color: var(--zgu-wire-active); color: #e2e8f0; }
.zgu-node-input-drop.has-file { border-style: solid; color: #e2e8f0; }
.zgu-node-input-drop input[type="file"] { position: absolute; inset: 0; width: 100%; height: 100%; opacity: 0; cursor: pointer; }
.zgu-node-input-drop .hint { font-size: 10px; }
.zgu-node-input-drop .chosen { font-size: 10.5px; word-break: break-all; }
.zgu-node-input-result img { width: 44px; height: 44px; object-fit: cover; border-radius: 4px; background: #0b0b0e; flex: none; }
.zgu-node-input-result .summary { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #e2e8f0; }
.zgu-node-input-result .meta { display: block; font-size: 10px; color: #64748b; }
.zgu-node-input-change {
  flex: none;
  border: 1px solid rgba(148,163,184,.4);
  border-radius: 5px;
  background: transparent;
  color: #94a3b8;
  font: inherit;
  font-size: 10px;
  padding: 2px 6px;
  cursor: pointer;
}
.zgu-node-input-change:hover { color: #e2e8f0; border-color: var(--zgu-wire-active); }
.zgu-note {
  position: absolute;
  z-index: 1;
  --zgu-note-bg: rgba(148,163,184,.14);
  --zgu-note-border: rgba(148,163,184,.55);
  --zgu-note-fg: #e2e8f0;
  border-radius: 10px;
  border: 1px solid var(--zgu-note-border);
  background: var(--zgu-note-bg);
  color: var(--zgu-note-fg);
  padding: 10px 12px;
  font-size: 12.5px;
  line-height: 1.45;
  user-select: none;
  touch-action: none;
  cursor: grab;
  overflow: hidden;
  will-change: transform;
  box-shadow: 0 6px 16px rgba(0,0,0,.22);
}
.zgu-note:active { cursor: grabbing; }
.zgu-note[data-color="amber"]  { --zgu-note-bg: rgba(245,158,11,.16); --zgu-note-border: rgba(245,158,11,.6); --zgu-note-fg: #fde68a; }
.zgu-note[data-color="blue"]   { --zgu-note-bg: rgba(96,165,250,.16); --zgu-note-border: rgba(96,165,250,.6); --zgu-note-fg: #bfdbfe; }
.zgu-note[data-color="green"]  { --zgu-note-bg: rgba(52,211,153,.16); --zgu-note-border: rgba(52,211,153,.6); --zgu-note-fg: #a7f3d0; }
.zgu-note[data-color="rose"]   { --zgu-note-bg: rgba(251,113,133,.16); --zgu-note-border: rgba(251,113,133,.6); --zgu-note-fg: #fecdd3; }
.zgu-note[data-color="violet"] { --zgu-note-bg: rgba(167,139,250,.16); --zgu-note-border: rgba(167,139,250,.6); --zgu-note-fg: #ddd6fe; }
.zgu-note[data-color="teal"]   { --zgu-note-bg: rgba(45,212,191,.16); --zgu-note-border: rgba(45,212,191,.6); --zgu-note-fg: #99f6e4; }
.zgu-note[data-color="orange"] { --zgu-note-bg: rgba(251,146,60,.16); --zgu-note-border: rgba(251,146,60,.6); --zgu-note-fg: #fed7aa; }
.zgu-note.selected { outline: 2px solid var(--zgu-wire-active); outline-offset: 2px; z-index: 2; }
/* The colour dot at a note's top-right and the palette it opens. The body
   under it has pointer-events: none, so the dot is what a press reaches. */
.zgu-note-color {
  position: absolute;
  top: 5px;
  right: 6px;
  z-index: 3;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--zgu-note-border);
  border: 1px solid rgba(255,255,255,.35);
  box-shadow: 0 1px 3px rgba(0,0,0,.4);
  cursor: pointer;
  opacity: .8;
}
.zgu-note-color:hover, .zgu-note.palette-open .zgu-note-color { opacity: 1; }
.zgu-note-palette {
  display: none;
  position: absolute;
  top: 21px;
  right: 4px;
  z-index: 4;
  grid-template-columns: repeat(4, 14px);
  gap: 5px;
  padding: 6px;
  border-radius: 8px;
  border: 1px solid rgba(148,163,184,.45);
  background: rgba(9,9,11,.96);
  box-shadow: 0 6px 16px rgba(0,0,0,.45);
  cursor: default;
}
.zgu-note.palette-open .zgu-note-palette { display: grid; }
.zgu-note-swatch {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 1px solid rgba(255,255,255,.3);
  cursor: pointer;
}
.zgu-note-swatch:hover { transform: scale(1.15); }
.zgu-note-swatch.selected { outline: 2px solid #e2e8f0; outline-offset: 1px; }
.zgu-note-swatch[data-color="slate"]  { background: #94a3b8; }
.zgu-note-swatch[data-color="amber"]  { background: #f59e0b; }
.zgu-note-swatch[data-color="blue"]   { background: #60a5fa; }
.zgu-note-swatch[data-color="green"]  { background: #34d399; }
.zgu-note-swatch[data-color="rose"]   { background: #fb7185; }
.zgu-note-swatch[data-color="violet"] { background: #a78bfa; }
.zgu-note-swatch[data-color="teal"]   { background: #2dd4bf; }
.zgu-note-swatch[data-color="orange"] { background: #fb923c; }
.zgu-root.zgu-readonly .zgu-note-color, .zgu-root.zgu-readonly .zgu-note-palette { display: none; }
.zgu-note-body { height: 100%; overflow: auto; word-break: break-word; white-space: normal; pointer-events: none; }
.zgu-note-body p { margin: 0 0 6px; }
.zgu-note-body p:last-child { margin-bottom: 0; }
.zgu-note-body h4 { margin: 0 0 6px; font-size: 13px; font-weight: 600; }
.zgu-note-body ul { margin: 0 0 6px; padding-left: 16px; }
.zgu-note-body code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 11.5px; background: rgba(0,0,0,.28); padding: 0 4px; border-radius: 4px; }
.zgu-note-edit {
  display: none;
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  padding: 10px 12px;
  margin: 0;
  border: 0;
  outline: none;
  resize: none;
  background: transparent;
  color: inherit;
  font: inherit;
  line-height: inherit;
}
.zgu-note.editing { cursor: text; }
.zgu-note.editing .zgu-note-body { visibility: hidden; }
.zgu-note.editing .zgu-note-edit { display: block; }
.zgu-note-resize {
  position: absolute;
  right: 3px;
  bottom: 3px;
  width: 12px;
  height: 12px;
  cursor: nwse-resize;
  border-right: 2px solid currentColor;
  border-bottom: 2px solid currentColor;
  border-radius: 0 0 3px 0;
  opacity: .45;
}
.zgu-root.zgu-readonly .zgu-note-resize { display: none; }
.zgu-node {
  position: absolute;
  --zgu-node-width: 84px;
  --zgu-label-max: 126px;
  width: 84px;
  min-height: 84px;
  border-radius: 10px;
  border: 1.5px solid #475569;
  background: #18181b;
  box-shadow: 0 8px 20px rgba(0,0,0,.42);
  user-select: none;
  z-index: 10;
  cursor: grab;
  -webkit-user-drag: none;
  touch-action: none;
  will-change: transform;
}
.zgu-node:active { cursor: grabbing; }
.zgu-node.trigger {
  --zgu-node-width: 102px;
  --zgu-label-max: 153px;
  width: 102px;
  border-color: transparent;
  background: transparent;
  box-shadow: none;
}
.zgu-node.selected {
  border-color: var(--zgu-wire-active);
  box-shadow: 0 0 0 2px rgba(52,211,153,.18), 0 8px 20px rgba(0,0,0,.42);
  z-index: 20;
}
.zgu-node.trigger.selected {
  border-color: transparent;
  box-shadow: none;
}
.zgu-node-core {
  position: relative;
  min-height: 76px;
  padding: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}
.zgu-node.trigger .zgu-node-core {
  padding-right: 18px;
  background: transparent;
  border: 0;
  clip-path: none;
}
.zgu-trigger-shape {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: visible;
  pointer-events: none;
  filter: drop-shadow(0 8px 18px rgba(0,0,0,.42));
}
.zgu-trigger-path {
  fill: #18181b;
  stroke: #475569;
  stroke-width: 1.5;
  vector-effect: non-scaling-stroke;
}
.zgu-node.trigger.selected .zgu-trigger-path {
  stroke: var(--zgu-wire-active);
}
.zgu-node.expanded .zgu-node-core {
  justify-content: flex-start;
  padding-top: 18px;
}
.zgu-node.expanded.trigger .zgu-node-core {
  padding-top: 18px;
  padding-right: 18px;
}
.zgu-node-icon {
  position: relative;
  z-index: 1;
  width: 46px;
  height: 46px;
  flex: 0 0 auto;
  object-fit: contain;
  display: block;
  pointer-events: none;
  -webkit-user-drag: none;
  user-drag: none;
}
.zgu-node-icon-fallback {
  position: relative;
  z-index: 1;
  width: 46px;
  height: 46px;
  border-radius: 8px;
  background: linear-gradient(135deg, rgba(52,211,153,.22), rgba(59,130,246,.16));
  border: 1px solid rgba(255,255,255,.1);
}
.zgu-node-label {
  position: absolute;
  top: calc(100% + 9px);
  left: 50%;
  width: max-content;
  max-width: var(--zgu-label-max);
  transform: translateX(-50%);
  color: #cbd5e1;
  font-size: 12px;
  font-weight: 650;
  line-height: 1.18;
  text-align: center;
  white-space: normal;
  overflow-wrap: normal;
  word-break: normal;
  pointer-events: none;
  text-shadow: 0 1px 2px rgba(0,0,0,.8);
}
.zgu-port-list {
  position: absolute;
  top: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 10px;
  pointer-events: none;
}
.zgu-port-list.in {
  left: 0;
}
.zgu-port-list.out {
  right: 0;
}
.zgu-port-wrap {
  position: relative;
  align-items: center;
  display: flex;
  width: 1px;
  min-height: 24px;
}
.zgu-port-wrap.in { justify-content: flex-start; }
.zgu-port-wrap.out { justify-content: flex-end; }
.zgu-port-pin-label {
  position: absolute;
  color: #cbd5e1;
  font-size: 8px;
  font-weight: 650;
  line-height: 1.25;
  white-space: nowrap;
  pointer-events: none;
  text-shadow: 0 1px 2px rgba(0,0,0,.85);
}
.zgu-port-pin-label.out {
  left: 18px;
  top: -2px;
  transform: translateY(-100%);
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: left;
}
.zgu-port {
  top: 50%;
  border: 2px solid #18181b;
  background: #64748b;
  position: absolute;
  cursor: crosshair;
  pointer-events: all;
  transition: background .12s, transform .12s;
}
.zgu-port:hover { background: var(--zgu-wire-active); }
.zgu-port.in {
  left: -5px;
  width: 10px;
  height: 22px;
  border-radius: 2px;
  transform: translateY(-50%);
}
.zgu-port.out {
  right: -7.5px;
  width: 13px;
  height: 13px;
  border-radius: 999px;
  transform: translateY(-50%);
}
.zgu-port.in:hover { transform: translateY(-50%) scale(1.12); }
.zgu-port.out:hover { transform: translateY(-50%) scale(1.3); }
.zgu-custom-input { width: 100%; accent-color: #10b981; }
.zgu-data-display {
  background: #000;
  border: 1px solid #333;
  border-radius: 4px;
  text-align: center;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 16px;
  color: #a78bfa;
  padding: 8px;
}
`;

function ensureStyles() {
  if (document.querySelector("style[data-zeb-graphui]")) {
    return;
  }
  const style = document.createElement("style");
  style.dataset.zebGraphui = "true";
  style.textContent = BASE_STYLE;
  document.head.appendChild(style);
}

function setThemeVars(root, theme) {
  root.style.setProperty("--zgu-bg-color", theme.bgColor);
  root.style.setProperty("--zgu-grid-color", theme.gridColor);
  root.style.setProperty("--zgu-wire-color", theme.wireColor);
  root.style.setProperty("--zgu-wire-hover", theme.wireHover);
  root.style.setProperty("--zgu-wire-active", theme.wireActive);
}

function clamp(v, min, max) {
  return Math.max(min, Math.min(max, v));
}

const AUTO_LAYOUT_BASE_X = 120;
const AUTO_LAYOUT_BASE_Y = 120;
const AUTO_LAYOUT_RANK_GAP_X = 360;
const AUTO_LAYOUT_NODE_GAP_Y = 180;

function autoLayoutAverageOrder(neighbors, order) {
  const values = (neighbors || [])
    .map((id) => order.get(id))
    .filter((value) => Number.isFinite(value));
  if (!values.length) return null;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function autoLayoutNodeOriginalOrder(nodes) {
  return new Map(nodes.map((node, index) => [node.id, index]));
}

function autoLayoutBuildTopology(nodes, links) {
  const idSet = new Set(nodes.map((node) => node.id));
  const incoming = new Map(nodes.map((node) => [node.id, []]));
  const outgoing = new Map(nodes.map((node) => [node.id, []]));
  const outSlotOrder = new Map();
  for (const link of links || []) {
    if (!idSet.has(link.fromNode) || !idSet.has(link.toNode)) continue;
    incoming.get(link.toNode).push(link.fromNode);
    outgoing.get(link.fromNode).push(link.toNode);
    outSlotOrder.set(`${link.fromNode}:${link.toNode}`, Number(link.fromSlot) || 0);
  }
  for (const ids of incoming.values()) ids.sort((a, b) => a - b);
  for (const ids of outgoing.values()) ids.sort((a, b) => a - b);
  return { incoming, outgoing, outSlotOrder };
}

function autoLayoutRankNodes(nodes, incoming, outgoing) {
  const ranks = new Map();
  const indegree = new Map(nodes.map((node) => [node.id, incoming.get(node.id)?.length || 0]));
  const original = autoLayoutNodeOriginalOrder(nodes);
  const roots = nodes
    .filter((node) => String(node.zfKind || "").startsWith("n.trigger.") || (incoming.get(node.id)?.length || 0) === 0)
    .sort((a, b) => (original.get(a.id) || 0) - (original.get(b.id) || 0));
  const queue = [];
  for (const node of roots) {
    if (!ranks.has(node.id)) {
      ranks.set(node.id, 0);
      queue.push(node.id);
    }
  }
  while (queue.length) {
    const id = queue.shift();
    const rank = ranks.get(id) || 0;
    for (const child of outgoing.get(id) || []) {
      ranks.set(child, Math.max(ranks.get(child) || 0, rank + 1));
      const nextIn = Math.max(0, (indegree.get(child) || 0) - 1);
      indegree.set(child, nextIn);
      if (nextIn === 0) queue.push(child);
    }
  }
  for (const node of nodes) {
    if (ranks.has(node.id)) continue;
    const parentRanks = (incoming.get(node.id) || [])
      .map((parent) => ranks.get(parent))
      .filter((rank) => Number.isFinite(rank));
    ranks.set(node.id, parentRanks.length ? Math.max(...parentRanks) + 1 : 0);
  }
  return ranks;
}

function autoLayoutOrderLevels(levels, incoming, outgoing, outSlotOrder, original) {
  const order = new Map();
  for (const ids of levels.values()) {
    ids.forEach((id, index) => order.set(id, index));
  }
  const rankKeys = Array.from(levels.keys()).sort((a, b) => a - b);
  for (let pass = 0; pass < 4; pass++) {
    for (const rank of rankKeys) {
      const ids = levels.get(rank) || [];
      ids.sort((a, b) => {
        const aw = autoLayoutAverageOrder(incoming.get(a), order);
        const bw = autoLayoutAverageOrder(incoming.get(b), order);
        const av = aw == null ? original.get(a) || 0 : aw;
        const bv = bw == null ? original.get(b) || 0 : bw;
        return av - bv || (original.get(a) || 0) - (original.get(b) || 0);
      });
      ids.forEach((id, index) => order.set(id, index));
    }
    for (const rank of [...rankKeys].reverse()) {
      const ids = levels.get(rank) || [];
      ids.sort((a, b) => {
        const aw = autoLayoutAverageOrder(outgoing.get(a), order);
        const bw = autoLayoutAverageOrder(outgoing.get(b), order);
        const av = aw == null ? original.get(a) || 0 : aw;
        const bv = bw == null ? original.get(b) || 0 : bw;
        const slotsA = (outgoing.get(a) || []).map((to) => outSlotOrder.get(`${a}:${to}`) ?? 0);
        const slotsB = (outgoing.get(b) || []).map((to) => outSlotOrder.get(`${b}:${to}`) ?? 0);
        const slotA = slotsA.length ? Math.min(...slotsA) : 0;
        const slotB = slotsB.length ? Math.min(...slotsB) : 0;
        return av - bv || slotA - slotB || (original.get(a) || 0) - (original.get(b) || 0);
      });
      ids.forEach((id, index) => order.set(id, index));
    }
  }
}

export function autoTidyGraphLayout(app, options = {}) {
  const nodes = Array.isArray(app?.graph?.nodes) ? app.graph.nodes : [];
  const links = Array.isArray(app?.graph?.links) ? app.graph.links : [];
  if (!nodes.length) return [];
  const baseX = Number(options.baseX ?? AUTO_LAYOUT_BASE_X);
  const baseY = Number(options.baseY ?? AUTO_LAYOUT_BASE_Y);
  const rankGapX = Number(options.rankGapX ?? AUTO_LAYOUT_RANK_GAP_X);
  const nodeGapY = Number(options.nodeGapY ?? AUTO_LAYOUT_NODE_GAP_Y);
  const { incoming, outgoing, outSlotOrder } = autoLayoutBuildTopology(nodes, links);
  const ranks = autoLayoutRankNodes(nodes, incoming, outgoing);
  const original = autoLayoutNodeOriginalOrder(nodes);
  const levels = new Map();
  for (const node of nodes) {
    const rank = ranks.get(node.id) || 0;
    if (!levels.has(rank)) levels.set(rank, []);
    levels.get(rank).push(node.id);
  }
  autoLayoutOrderLevels(levels, incoming, outgoing, outSlotOrder, original);

  const byId = new Map(nodes.map((node) => [node.id, node]));
  const placements = [];
  for (const rank of Array.from(levels.keys()).sort((a, b) => a - b)) {
    const ids = levels.get(rank) || [];
    ids.forEach((id, row) => {
      const node = byId.get(id);
      if (!node) return;
      node.x = baseX + rank * rankGapX;
      node.y = baseY + row * nodeGapY;
      if (app.ui?.snapToGrid) {
        app.ui.snapNodePosition(node);
      } else {
        node.applyTransform();
      }
      placements.push({ id, x: node.x, y: node.y, rank, row });
    });
  }
  app.ui?.clearSelection?.();
  app.ui?.updateWires?.();
  return placements;
}

function triggerShapePath(width, height) {
  const w = Math.max(48, Number(width) || 102);
  const h = Math.max(48, Number(height) || 84);
  const point = Math.min(18, w * 0.25);
  const bodyRight = w - point;
  const r = Math.min(9, h * 0.12, bodyRight * 0.15);
  const join = Math.min(5, point * 0.32);
  const mid = h / 2;
  return [
    `M ${r} 0.75`,
    `L ${bodyRight - join} 0.75`,
    `Q ${bodyRight + join * 0.25} 0.75 ${bodyRight + join * 0.75} ${join}`,
    `L ${w - 1.25} ${mid - join}`,
    `Q ${w + 0.2} ${mid} ${w - 1.25} ${mid + join}`,
    `L ${bodyRight + join * 0.75} ${h - join}`,
    `Q ${bodyRight + join * 0.25} ${h - 0.75} ${bodyRight - join} ${h - 0.75}`,
    `L ${r} ${h - 0.75}`,
    `Q 0.75 ${h - 0.75} 0.75 ${h - r}`,
    `L 0.75 ${r}`,
    `Q 0.75 0.75 ${r} 0.75`,
    "Z",
  ].join(" ");
}

export class GraphStore {
  constructor() {
    this.nodes = [];
    this.links = [];
    // Canvas notes: presentation only. Never in `links`, never executed.
    this.notes = [];
    this.lastId = 1;
  }

  addNote(note) {
    note.uid = this.lastId++;
    this.notes.push(note);
    return note;
  }

  removeNote(note) {
    this.notes = this.notes.filter((n) => n.uid !== note.uid);
    if (note.el) {
      note.el.remove();
    }
  }

  add(node) {
    node.id = this.lastId++;
    this.nodes.push(node);
    return node;
  }

  remove(node) {
    this.nodes = this.nodes.filter((n) => n.id !== node.id);
    this.links = this.links.filter((l) => l.fromNode !== node.id && l.toNode !== node.id);
    if (node.el) {
      node.el.remove();
    }
    node.detachSlot?.();
  }

  clear() {
    this.nodes.forEach((node) => {
      if (node.el) node.el.remove();
      node.detachSlot?.();
    });
    this.notes.forEach((note) => note.el && note.el.remove());
    this.nodes = [];
    this.links = [];
    this.notes = [];
    this.lastId = 1;
  }

  connect(fromNodeId, fromSlot, toNodeId, toSlot, options = {}) {
    // Remove only exact duplicate edges (same source AND same target), allow fan-in.
    this.links = this.links.filter(
      (l) => !(l.fromNode === fromNodeId && l.fromSlot === fromSlot && l.toNode === toNodeId && l.toSlot === toSlot)
    );
    const cfg = { ...DEFAULT_LINK_OPTIONS, ...options };
    const id = `zgu_link_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    this.links.push({
      id,
      fromNode: fromNodeId,
      fromSlot,
      toNode: toNodeId,
      toSlot,
      options: cfg,
    });
    return id;
  }

  execute() {
    this.nodes.forEach((node) => {
      node.inputs.forEach((input) => {
        input.value = 0;
      });
    });

    this.links.forEach((link) => {
      const source = this.nodes.find((n) => n.id === link.fromNode);
      const target = this.nodes.find((n) => n.id === link.toNode);
      if (!source || !target) {
        return;
      }
      if (!target.inputs[link.toSlot] || !source.outputs[link.fromSlot]) {
        return;
      }
      target.inputs[link.toSlot].value = source.outputs[link.fromSlot].value;
    });

    this.nodes.forEach((node) => {
      node.compute();
      if (typeof node.updateDOM === "function") {
        node.updateDOM();
      }
    });
  }
}

export class GraphNode {
  constructor({ title, x = 0, y = 0, color = "#334155", icon = "" }) {
    this.id = 0;
    this.title = title;
    this.x = x;
    this.y = y;
    this.color = color;
    this.icon = icon;
    this.inputs = [];
    this.outputs = [];
    this.el = null;
    // Preview panel drawn under the box — presentation only, see
    // `positionNodePreviewEl`. Owned by the canvas, not by the graph.
    this.previewEl = null;
    // Input widget for an `n.input.*` node — the Run form's field, drawn in
    // the same slot above the preview. Owned by the canvas as well.
    this.inputEl = null;
    // Run badge at the box's top-right — see `setNodeStatus`. It lives
    // inside `el`, so it moves with the box and dies with it.
    this.statusEl = null;
  }

  /** Drop the run badge only — `setNodeStatus(node, null)` uses this. */
  detachStatus() {
    if (this.statusEl) {
      this.statusEl.remove();
      this.statusEl = null;
    }
  }

  /**
   * Move the node's DOM to its current x/y. Every place that moves a node goes
   * through here, so its preview panel can never be left behind.
   */
  applyTransform() {
    if (this.el) {
      this.el.style.transform = `translate(${this.x}px, ${this.y}px)`;
    }
    positionNodePreviewEl(this);
  }

  /** Drop the preview panel only — `setNodePreview(node, null)` uses this. */
  detachPreview() {
    if (this.previewEl) {
      this.previewEl.remove();
      this.previewEl = null;
    }
    positionNodePreviewEl(this);
  }

  /** Drop the input widget only — `setNodeInput(node, null)` uses this. */
  detachInput() {
    if (this.inputEl) {
      this.inputEl.remove();
      this.inputEl = null;
    }
    positionNodePreviewEl(this);
  }

  /**
   * Drop the whole under-box slot — called wherever the node's element is
   * removed. Kept apart from the two halves: a preview sync that finds no
   * preview for a node must not take that node's Run-form field with it.
   */
  detachSlot() {
    this.detachPreview();
    this.detachInput();
    this.detachStatus();
  }

  addInput(name) {
    this.inputs.push({ name, value: 0 });
  }

  addOutput(name) {
    this.outputs.push({ name, value: 0 });
  }

  compute() {}

  buildCustomHTML(_container) {}

  updateDOM() {}

  buildDOM(container) {
    const node = document.createElement("div");
    node.className = "zgu-node";
    node.dataset.id = String(this.id);
    node.style.transform = `translate(${this.x}px, ${this.y}px)`;
    node.draggable = false;
    node.addEventListener("dragstart", (event) => event.preventDefault());
    const kind = String(this.zfKind || "");
    if (kind.startsWith("n.trigger.")) {
      node.classList.add("trigger");
    }

    const pinCount = Math.max(this.inputs.length, this.outputs.length);
    const nodeHeight = Math.max(84, 52 + Math.max(1, pinCount) * 26);
    this.zguNodeHeight = nodeHeight;
    if (pinCount > 2) {
      node.classList.add("expanded");
    }
    node.style.minHeight = `${nodeHeight}px`;

    const core = document.createElement("div");
    core.className = "zgu-node-core";
    core.style.minHeight = `${nodeHeight - 2}px`;
    if (kind.startsWith("n.trigger.")) {
      const svgNS = "http://www.w3.org/2000/svg";
      const shape = document.createElementNS(svgNS, "svg");
      shape.classList.add("zgu-trigger-shape");
      shape.setAttribute("viewBox", `0 0 102 ${nodeHeight}`);
      shape.setAttribute("preserveAspectRatio", "none");
      shape.setAttribute("aria-hidden", "true");
      const path = document.createElementNS(svgNS, "path");
      path.classList.add("zgu-trigger-path");
      path.setAttribute("d", triggerShapePath(102, nodeHeight));
      shape.appendChild(path);
      core.appendChild(shape);
    }
    if (this.icon) {
      const icon = document.createElement("img");
      icon.className = "zgu-node-icon";
      icon.src = this.icon;
      icon.alt = "";
      icon.decoding = "async";
      icon.draggable = false;
      icon.addEventListener("dragstart", (event) => event.preventDefault());
      core.appendChild(icon);
    } else {
      const fallback = document.createElement("div");
      fallback.className = "zgu-node-icon-fallback";
      core.appendChild(fallback);
    }

    node.appendChild(core);

    if (this.inputs.length > 0) {
      const list = document.createElement("div");
      list.className = "zgu-port-list in";
      this.inputs.forEach((input, index) => {
        const wrap = document.createElement("div");
        wrap.className = "zgu-port-wrap in";
        const port = document.createElement("div");
        port.className = "zgu-port in";
        port.dataset.type = "in";
        port.dataset.index = String(index);
        wrap.appendChild(port);
        list.appendChild(wrap);
      });
      node.appendChild(list);
    }

    if (this.outputs.length > 0) {
      const list = document.createElement("div");
      list.className = "zgu-port-list out";
      this.outputs.forEach((output, index) => {
        const wrap = document.createElement("div");
        wrap.className = "zgu-port-wrap out";
        const displayLabel = String(this.zfOutputLabels?.[output.name] || output.label || output.name || "").trim();
        if (displayLabel && (this.outputs.length > 1 || String(this.zfKind || "") === "n.logic.match")) {
          const pinLabel = document.createElement("div");
          pinLabel.className = "zgu-port-pin-label out";
          pinLabel.textContent = displayLabel;
          wrap.appendChild(pinLabel);
        }
        const port = document.createElement("div");
        port.className = "zgu-port out";
        port.dataset.type = "out";
        port.dataset.index = String(index);
        if (displayLabel) {
          port.title = displayLabel;
        }
        wrap.appendChild(port);
        list.appendChild(wrap);
      });
      node.appendChild(list);
    }

    const label = document.createElement("div");
    label.className = "zgu-node-label";
    label.textContent = this.title;
    node.appendChild(label);

    container.appendChild(node);
    this.el = node;
    // A rebuilt box may be a different height (pins changed); the preview that
    // outlived it moves to match.
    positionNodePreviewEl(this);
    return node;
  }
}

export class CustomNode extends GraphNode {
  constructor({ title, x = 0, y = 0, color = "#334155", icon = "", inputs = [], outputs = [] }) {
    super({ title, x, y, color, icon });
    inputs.forEach((name) => this.addInput(name));
    outputs.forEach((name) => this.addOutput(name));
  }

  compute() {
    if (this.inputs.length === 0 || this.outputs.length === 0) {
      return;
    }
    const value = this.inputs[0].value;
    this.outputs.forEach((output) => {
      output.value = value;
    });
  }
}

export class NumberNode extends GraphNode {
  constructor({ x = 0, y = 0, title = "Number Generator", color = "#065f46", min = 0, max = 100, value = null } = {}) {
    super({ title, x, y, color });
    this.addOutput("Value");
    this.min = min;
    this.max = max;
    this.value = value == null ? Math.floor(Math.random() * (max - min + 1)) + min : value;
    this.displayEl = null;
  }

  buildCustomHTML(container) {
    container.innerHTML = `
      <div style=\"display:flex;justify-content:space-between;font-size:11px;color:#a3a3a3;margin-bottom:6px;\">
        <span>Value</span>
        <span class=\"zgu-number-value\">${this.value}</span>
      </div>
      <input class=\"zgu-custom-input\" type=\"range\" min=\"${this.min}\" max=\"${this.max}\" value=\"${this.value}\" data-zgu-nodrag=\"true\" />
    `;
    this.displayEl = container.querySelector(".zgu-number-value");
    const slider = container.querySelector("input");
    slider.addEventListener("input", (event) => {
      this.value = Number(event.target.value);
      if (this.displayEl) {
        this.displayEl.textContent = String(this.value);
      }
    });
  }

  compute() {
    this.outputs[0].value = this.value;
  }
}

export class AddNode extends GraphNode {
  constructor({ x = 0, y = 0, title = "Math: Add", color = "#1e3a8a" } = {}) {
    super({ title, x, y, color });
    this.addInput("A");
    this.addInput("B");
    this.addOutput("Result");
  }

  compute() {
    const a = Number(this.inputs[0].value || 0);
    const b = Number(this.inputs[1].value || 0);
    this.outputs[0].value = a + b;
  }
}

export class DisplayNode extends GraphNode {
  constructor({ x = 0, y = 0, title = "Display", color = "#4c1d95" } = {}) {
    super({ title, x, y, color });
    this.addInput("In");
    this.displayValue = 0;
    this.displayEl = null;
  }

  buildCustomHTML(container) {
    container.innerHTML = `<div class=\"zgu-data-display\">0</div>`;
    this.displayEl = container.querySelector(".zgu-data-display");
  }

  compute() {
    this.displayValue = Number(this.inputs[0].value || 0);
  }

  updateDOM() {
    if (!this.displayEl) {
      return;
    }
    this.displayEl.textContent = this.displayValue.toFixed(0);
  }
}

function escapeNoteHtml(text) {
  return String(text)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function inlineNoteMarkdown(text) {
  return escapeNoteHtml(text)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*])\*([^*\n]+)\*(?!\*)/g, "$1<em>$2</em>");
}

/**
 * The subset of markdown a canvas note needs: headings, bullets, paragraphs,
 * bold, italic, inline code. Everything is escaped first, so a note is never
 * a script.
 */
export function renderNoteMarkdown(text) {
  const out = [];
  let paragraph = [];
  let list = [];
  const flushParagraph = () => {
    if (paragraph.length) {
      out.push(`<p>${paragraph.map(inlineNoteMarkdown).join("<br>")}</p>`);
      paragraph = [];
    }
  };
  const flushList = () => {
    if (list.length) {
      out.push(`<ul>${list.map((item) => `<li>${inlineNoteMarkdown(item)}</li>`).join("")}</ul>`);
      list = [];
    }
  };
  for (const raw of String(text || "").split(/\r?\n/)) {
    const line = raw.trimEnd();
    if (!line.trim()) {
      flushParagraph();
      flushList();
      continue;
    }
    const heading = line.match(/^#{1,6}\s+(.*)$/);
    if (heading) {
      flushParagraph();
      flushList();
      out.push(`<h4>${inlineNoteMarkdown(heading[1])}</h4>`);
      continue;
    }
    const bullet = line.match(/^\s*[-*]\s+(.*)$/);
    if (bullet) {
      flushParagraph();
      list.push(bullet[1]);
      continue;
    }
    flushList();
    paragraph.push(line);
  }
  flushParagraph();
  flushList();
  return out.join("");
}

// ── Node previews ───────────────────────────────────────────────────────────
//
// A preview is a panel drawn under a node box showing one value from that
// node's latest run. Like a note it is presentation only: never a link, never
// executed, never read by the engine. The bundle owns the drawing; *what* to
// draw arrives from the host as a `previewData` prop keyed by pipeline node id:
//
//   { [pipelineNodeId]: { in?: PreviewCell, out?: PreviewCell } }
//   PreviewCell = { as, status: "ok" | "none" | "error",
//                   src?, text?, rows?, error?, width?, height? }
//
// `width` / `height` are the cell's size in canvas pixels when the host has
// one stored; otherwise the defaults below. A cell has a resize grip at its
// bottom-right, like a note: dragging it resizes that cell live, and on
// release the canvas reports `onPreviewResize(pipelineNodeId, which,
// { width, height })` — the host stores it, the canvas never does.
//
// Every value below is written through `textContent` or a checked `src`, so a
// payload can never become markup here.

const NODE_PREVIEW_WIDTH = 220;
const NODE_PREVIEW_GAP = 6;
const NODE_PREVIEW_MIN_WIDTH = 160;
const NODE_PREVIEW_MIN_HEIGHT = 60;
const NODE_PREVIEW_MAX_WIDTH = 1200;
const NODE_PREVIEW_MAX_HEIGHT = 900;
const NODE_PREVIEW_HEIGHTS = {
  image: 160,
  video: 160,
  pdf: 160,
  html: 160,
  table: 120,
  json: 90,
  text: 90,
  audio: 90,
};
const NODE_PREVIEW_TABLE_ROWS = 8;
const NODE_PREVIEW_TABLE_COLUMNS = 4;

function nodePreviewHeight(as) {
  return NODE_PREVIEW_HEIGHTS[String(as || "").toLowerCase()] || 90;
}

// An input widget (`text` / `json`) has a grip like a preview cell's. Its
// size is `spec.width` / `spec.height` when the host stores one
// (`config.ui.widget`), else 220 wide and as tall as its content. On release
// the canvas reports `onInputResize(pipelineNodeId, { width, height })` and
// stores nothing; between the drag and the next scene load the DOM owns it.
const NODE_INPUT_MIN_WIDTH = 160;
const NODE_INPUT_MIN_HEIGHT = 48;
const NODE_INPUT_MAX_WIDTH = 900;
const NODE_INPUT_MAX_HEIGHT = 600;
const NODE_INPUT_RESIZABLE_KINDS = ["text", "json"];

/** A stored widget size within bounds, or null when the spec has none. */
function nodeInputSize(spec) {
  const w = Number(spec && spec.width);
  const h = Number(spec && spec.height);
  if (!(w > 0) || !(h > 0)) return null;
  return {
    width: clamp(Math.round(w), NODE_INPUT_MIN_WIDTH, NODE_INPUT_MAX_WIDTH),
    height: clamp(Math.round(h), NODE_INPUT_MIN_HEIGHT, NODE_INPUT_MAX_HEIGHT),
  };
}

/** Apply (or, for null, clear) a size on a widget element. */
function applyNodeInputSize(el, size) {
  if (!size) {
    el.style.height = "";
    delete el.dataset.width;
    delete el.dataset.height;
    return;
  }
  el.style.width = `${size.width}px`;
  el.style.height = `${size.height}px`;
  el.dataset.width = String(size.width);
  el.dataset.height = String(size.height);
}

/** A cell's size: the stored one within bounds, else the defaults by kind. */
function nodePreviewSize(cell) {
  const stored = (value, fallback, min, max) => {
    const n = Number(value);
    return Number.isFinite(n) && n > 0 ? clamp(Math.round(n), min, max) : fallback;
  };
  return {
    width: stored(cell && cell.width, NODE_PREVIEW_WIDTH, NODE_PREVIEW_MIN_WIDTH, NODE_PREVIEW_MAX_WIDTH),
    height: stored(cell && cell.height, nodePreviewHeight(cell && cell.as), NODE_PREVIEW_MIN_HEIGHT, NODE_PREVIEW_MAX_HEIGHT),
  };
}

/** Apply a size to a built cell: the wrap's width, the body's height. */
function applyNodePreviewSize(cellEl, width, height) {
  cellEl.style.width = `${width}px`;
  cellEl.dataset.width = String(width);
  cellEl.dataset.height = String(height);
  const body = cellEl.querySelector(".zgu-node-preview-body");
  if (body) body.style.height = `${height}px`;
}

/** A URL the browser may load as media — an absolute path or http(s). */
function safeNodePreviewSrc(raw, as) {
  const src = String(raw || "").trim();
  if (!src) return "";
  if (src.startsWith("/") || /^https?:\/\//i.test(src)) return src;
  // An image cell may carry its picture inline: the host's snapshot of a
  // temporary file, taken from the run's record. Only an image, only a
  // data URI — a script or a page never rides in here.
  if (as === "image" && /^data:image\/(png|jpeg|webp|gif);base64,/i.test(src)) return src;
  return "";
}

function nodePreviewCellText(value) {
  if (value === null || value === undefined) return "";
  return typeof value === "string" ? value : String(value);
}

function buildNodePreviewTable(rows) {
  const items = (Array.isArray(rows) ? rows : []).slice(0, NODE_PREVIEW_TABLE_ROWS);
  const scroll = document.createElement("div");
  scroll.className = "zgu-node-preview-scroll";
  if (!items.length) {
    scroll.textContent = "no rows";
    return scroll;
  }
  const columns = [];
  for (const row of items) {
    if (!row || typeof row !== "object" || Array.isArray(row)) continue;
    for (const key of Object.keys(row)) {
      if (!columns.includes(key) && columns.length < NODE_PREVIEW_TABLE_COLUMNS) {
        columns.push(key);
      }
    }
  }
  const table = document.createElement("table");
  table.className = "zgu-node-preview-table";
  const head = document.createElement("tr");
  for (const column of columns) {
    const th = document.createElement("th");
    th.textContent = column;
    head.appendChild(th);
  }
  table.appendChild(head);
  for (const row of items) {
    const tr = document.createElement("tr");
    for (const column of columns) {
      const td = document.createElement("td");
      const value = row && typeof row === "object" ? row[column] : undefined;
      td.textContent =
        value !== null && typeof value === "object"
          ? JSON.stringify(value)
          : nodePreviewCellText(value);
      td.title = td.textContent;
      tr.appendChild(td);
    }
    table.appendChild(tr);
  }
  scroll.appendChild(table);
  return scroll;
}

/** One half of a node's preview — `which` is "in" or "out". */
function buildNodePreviewCell(cell, which) {
  const wrap = document.createElement("div");
  wrap.className = "zgu-node-preview-cell";
  wrap.dataset.which = which;
  wrap.dataset.as = String((cell && cell.as) || "");

  const tag = document.createElement("span");
  tag.className = "zgu-node-preview-tag";
  tag.textContent = which;
  wrap.appendChild(tag);

  const body = document.createElement("div");
  body.className = "zgu-node-preview-body";
  wrap.appendChild(body);
  const size = nodePreviewSize(cell);
  applyNodePreviewSize(wrap, size.width, size.height);

  // The grip — same look as a note's, hidden by CSS when read-only. The
  // canvas recognises it by class in the preview panel's pointerdown.
  const grip = document.createElement("div");
  grip.className = "zgu-node-preview-resize";
  grip.title = "Drag to resize";
  wrap.appendChild(grip);

  const status = String((cell && cell.status) || "none");
  if (status === "error") {
    body.classList.add("error");
    body.textContent = nodePreviewCellText((cell && cell.error) || "error");
    return wrap;
  }
  if (status !== "ok") {
    body.classList.add("empty");
    body.textContent = "no run yet";
    return wrap;
  }

  const as = String((cell && cell.as) || "json").toLowerCase();
  const src = safeNodePreviewSrc(cell && cell.src, as);
  if (!src && (as === "image" || as === "video" || as === "audio" || as === "pdf" || as === "html")) {
    body.classList.add("empty");
    body.textContent = "nothing to show";
    return wrap;
  }
  // A small caption the host sets — "temporary — not saved" on a snapshot.
  const noteText = nodePreviewCellText(cell && cell.note).trim();
  if (noteText) {
    const note = document.createElement("span");
    note.className = "zgu-node-preview-note";
    note.textContent = noteText;
    wrap.appendChild(note);
  }

  if (as === "image") {
    const img = document.createElement("img");
    img.src = src;
    img.alt = "";
    img.loading = "lazy";
    img.decoding = "async";
    img.draggable = false;
    img.addEventListener("dragstart", (event) => event.preventDefault());
    body.appendChild(img);
  } else if (as === "video") {
    const video = document.createElement("video");
    video.src = src;
    video.controls = true;
    video.preload = "metadata";
    body.appendChild(video);
  } else if (as === "audio") {
    const audio = document.createElement("audio");
    audio.src = src;
    audio.controls = true;
    audio.preload = "metadata";
    body.appendChild(audio);
  } else if (as === "pdf" || as === "html") {
    const frame = document.createElement("iframe");
    frame.src = src;
    frame.setAttribute("sandbox", "");
    frame.setAttribute("loading", "lazy");
    frame.setAttribute("referrerpolicy", "no-referrer");
    frame.title = `${which} preview`;
    body.appendChild(frame);
  } else if (as === "table") {
    body.appendChild(buildNodePreviewTable(cell && cell.rows));
  } else {
    const pre = document.createElement("pre");
    pre.textContent = nodePreviewCellText(cell && cell.text);
    body.appendChild(pre);
  }
  return wrap;
}

// ── Node run status ─────────────────────────────────────────────────────────
//
// A badge at a node box's top-right saying how the node's last (or current)
// run went — n8n-style. Presentation only, like a preview: the host drives
// it through a `nodeStatus` prop keyed by pipeline node id:
//
//   { [pipelineNodeId]: { state: "pending" | "running" | "ok" | "skip" | "fail"
//                                | "retry" | "error_routed",
//                         duration_ms?, error?, attempt?, max_attempts?, to_node? } }
//
// pending: grey dot · running: orange pulsing dot (with the count when the
// node is going round again) · ok: green tick · skip: grey dash · fail: red
// cross · retry: orange ring with the attempt count ("3/40") · error_routed:
// orange ring, tooltip "error → <to_node>"; the duration under it when known. Red is for `fail` only: a failure an
// `:error` edge consumed is a wait or a handled error, and a poll loop that
// waited eight times and then succeeded must not look like eight failures.

const NODE_STATUS_STATES = ["pending", "running", "ok", "skip", "fail", "retry", "error_routed"];
const NODE_STATUS_MARKS = { pending: "", running: "", ok: "✓", skip: "–", fail: "✕", retry: "", error_routed: "" };

/** `3/40`, or `3` when the budget is unknown (a record seeds the count only); empty otherwise. */
function formatNodeStatusCount(status) {
  const attempt = Number(status && status.attempt);
  if (!Number.isFinite(attempt) || attempt <= 0) return "";
  const max = Number(status && status.max_attempts);
  return Number.isFinite(max) && max > 0 ? `${attempt}/${max}` : `${attempt}`;
}

/** The badge's tooltip: the state, the count or the destination, and the error text when there is one. */
function nodeStatusTitle(state, status) {
  const error = status && status.error ? String(status.error) : "";
  if (state === "retry") {
    const count = formatNodeStatusCount(status);
    const head = count ? `retry ${count}` : "retry";
    return error ? `${head}: ${error}` : head;
  }
  if (state === "error_routed") {
    const head = status && status.to_node ? `error → ${String(status.to_node)}` : "error routed";
    return error ? `${head}: ${error}` : head;
  }
  return error ? `${state}: ${error}` : state;
}

/** `412 ms`, `3.9 s`, `1m 05s`; empty when unknown. */
function formatNodeStatusDuration(ms) {
  const n = Number(ms);
  if (!Number.isFinite(n) || n < 0) return "";
  if (n < 1000) return `${Math.round(n)} ms`;
  if (n < 60000) return `${(n / 1000).toFixed(n < 10000 ? 1 : 0)} s`;
  const seconds = Math.round((n % 60000) / 1000);
  return `${Math.floor(n / 60000)}m ${String(seconds).padStart(2, "0")}s`;
}

// ── Node input widgets ──────────────────────────────────────────────────────
//
// An `n.input.*` node draws a form field in the same under-node slot the
// previews use: the Run form, built from the graph. What to draw arrives from
// the host as an `inputWidgets` prop keyed by pipeline node id:
//
//   { [pipelineNodeId]: { kind, name, label, optional, accept, default,
//                         value?, invalid?, result? } }
//
// `kind` is the word after `input.`; `value` seeds the field when it is
// (re)built and is deliberately outside the rebuild signature, so typing never
// rebuilds the field under the cursor; `invalid` highlights without rebuilding;
// `result` — what went in on the latest run — is shown instead of the field
// until "change" is pressed. Values live in the host's state only: the widget
// emits `onInputChange(pipelineNodeId, value | File | File[])` and stores
// nothing. Widget first, preview beneath, when both exist.

const NODE_INPUT_FILE_KINDS = ["file", "files", "image", "audio", "video"];
const NODE_INPUT_ACCEPT_BY_KIND = {
  image: "image/*",
  audio: "audio/*",
  video: "video/*",
  pdf: "application/pdf,.pdf",
  csv: "text/csv,.csv",
  json: "application/json,.json",
  geojson: "application/geo+json,.geojson",
  spreadsheet: ".xlsx,.xls,.ods",
  archive: "application/zip,.zip",
  parquet: ".parquet",
};

/** The `accept` attribute for a declaration's FileRef kinds, mimes and extensions. */
function nodeInputAcceptAttr(spec) {
  const kind = String((spec && spec.kind) || "");
  let list = Array.isArray(spec && spec.accept) ? spec.accept : [];
  if (!list.length && NODE_INPUT_ACCEPT_BY_KIND[kind]) list = [kind];
  const out = [];
  for (const raw of list) {
    const item = String(raw || "").trim().toLowerCase();
    if (!item) continue;
    if (NODE_INPUT_ACCEPT_BY_KIND[item]) out.push(NODE_INPUT_ACCEPT_BY_KIND[item]);
    else if (item.endsWith("/")) out.push(`${item}*`);
    else if (item.includes("/")) out.push(item);
    else out.push(item.startsWith(".") ? item : `.${item}`);
  }
  return out.join(",");
}

function nodeInputFormatBytes(size) {
  const n = Number(size);
  if (!Number.isFinite(n) || n < 0) return "";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function nodeInputFileSummary(files) {
  const list = Array.from(files || []);
  if (!list.length) return "";
  if (list.length === 1) return `${list[0].name} · ${nodeInputFormatBytes(list[0].size)}`;
  return `${list.length} files · ${nodeInputFormatBytes(list.reduce((a, f) => a + (f.size || 0), 0))}`;
}

/** The result view: what went in on the latest run, as the host resolved it. */
function buildNodeInputResult(result, onChange) {
  const wrap = document.createElement("div");
  wrap.className = "zgu-node-input-result";
  const src = safeNodePreviewSrc(result && result.src);
  if (src) {
    const img = document.createElement("img");
    img.src = src;
    img.alt = "";
    img.loading = "lazy";
    img.draggable = false;
    img.addEventListener("dragstart", (event) => event.preventDefault());
    img.addEventListener("error", () => img.remove());
    wrap.appendChild(img);
  }
  const summary = document.createElement("span");
  summary.className = "summary";
  summary.textContent = nodePreviewCellText(result && result.text);
  summary.title = summary.textContent;
  const meta = nodePreviewCellText(result && result.meta);
  if (meta) {
    const metaEl = document.createElement("span");
    metaEl.className = "meta";
    metaEl.textContent = meta;
    summary.appendChild(metaEl);
  }
  wrap.appendChild(summary);
  const change = document.createElement("button");
  change.type = "button";
  change.className = "zgu-node-input-change";
  change.textContent = "change";
  change.addEventListener("click", (event) => {
    event.stopPropagation();
    onChange();
  });
  wrap.appendChild(change);
  return wrap;
}

/** The field view: one control per value type, emitting through `emit`. */
function buildNodeInputField(spec, emit) {
  const kind = String(spec.kind || "text");
  const seed = spec.value !== undefined && spec.value !== null ? spec.value : spec.default;
  const field = document.createElement("div");
  field.className = "zgu-node-input-field";
  const stop = (el) => {
    // The canvas listens for keys (delete, select-all) and pointer moves; a
    // field is typed into, not a handle for the node.
    for (const type of ["keydown", "keyup", "keypress", "pointerdown", "mousedown", "wheel"]) {
      el.addEventListener(type, (event) => event.stopPropagation());
    }
  };

  if (NODE_INPUT_FILE_KINDS.includes(kind)) {
    const drop = document.createElement("div");
    drop.className = "zgu-node-input-drop";
    const hint = document.createElement("span");
    hint.className = "hint";
    hint.textContent = kind === "files" ? "Drop files or click" : "Drop a file or click";
    const chosen = document.createElement("span");
    chosen.className = "chosen";
    const input = document.createElement("input");
    input.type = "file";
    input.name = String(spec.name || "");
    if (kind === "files") input.multiple = true;
    const accept = nodeInputAcceptAttr(spec);
    if (accept) input.accept = accept;
    const show = (files) => {
      const summary = nodeInputFileSummary(files);
      chosen.textContent = summary;
      drop.classList.toggle("has-file", !!summary);
      hint.textContent = summary ? "Drop to replace" : kind === "files" ? "Drop files or click" : "Drop a file or click";
    };
    const take = (files) => {
      const list = Array.from(files || []);
      show(list);
      emit(kind === "files" ? list : list[0] || null);
    };
    input.addEventListener("change", () => take(input.files));
    for (const type of ["dragenter", "dragover"]) {
      drop.addEventListener(type, (event) => {
        event.preventDefault();
        event.stopPropagation();
        drop.classList.add("over");
      });
    }
    drop.addEventListener("dragleave", () => drop.classList.remove("over"));
    drop.addEventListener("drop", (event) => {
      event.preventDefault();
      event.stopPropagation();
      drop.classList.remove("over");
      if (event.dataTransfer && event.dataTransfer.files && event.dataTransfer.files.length) {
        try { input.files = event.dataTransfer.files; } catch (_) {}
        take(event.dataTransfer.files);
      }
    });
    stop(drop);
    if (seed && typeof seed === "object") show(Array.isArray(seed) ? seed : [seed]);
    drop.appendChild(hint);
    drop.appendChild(chosen);
    drop.appendChild(input);
    field.appendChild(drop);
    return field;
  }

  if (kind === "boolean") {
    const label = document.createElement("label");
    label.className = "zgu-node-input-check";
    const input = document.createElement("input");
    input.type = "checkbox";
    input.name = String(spec.name || "");
    input.checked = seed === true || seed === "true" || seed === 1 || seed === "1" || seed === "on";
    input.addEventListener("change", () => emit(input.checked));
    const text = document.createElement("span");
    text.textContent = "yes";
    label.appendChild(input);
    label.appendChild(text);
    stop(label);
    field.appendChild(label);
    return field;
  }

  if (kind === "json") {
    const area = document.createElement("textarea");
    area.name = String(spec.name || "");
    area.dataset.kind = "json";
    area.rows = 3;
    area.spellcheck = false;
    area.placeholder = "{ }";
    area.value =
      seed === undefined || seed === null ? "" : typeof seed === "string" ? seed : JSON.stringify(seed, null, 2);
    area.addEventListener("input", () => emit(area.value));
    stop(area);
    field.appendChild(area);
    return field;
  }

  // Text is a textarea too — a prompt or a story beat runs to several lines,
  // and the widget's grip sizes it. `--max` is still the length limit.
  if (kind === "text") {
    const area = document.createElement("textarea");
    area.name = String(spec.name || "");
    area.dataset.kind = "text";
    area.rows = 2;
    if (spec.max !== undefined && spec.max !== null && spec.max !== "") {
      area.maxLength = Number(spec.max) || 524288;
    }
    area.placeholder = String(spec.label || spec.name || "");
    area.value = seed === undefined || seed === null ? "" : String(seed);
    area.addEventListener("input", () => emit(area.value));
    stop(area);
    field.appendChild(area);
    return field;
  }

  const input = document.createElement("input");
  input.type = kind === "number" ? "number" : "text";
  input.name = String(spec.name || "");
  if (kind === "number") {
    if (spec.min !== undefined && spec.min !== null && spec.min !== "") input.min = String(spec.min);
    if (spec.max !== undefined && spec.max !== null && spec.max !== "") input.max = String(spec.max);
    input.step = "any";
  } else if (spec.max !== undefined && spec.max !== null && spec.max !== "") {
    input.maxLength = Number(spec.max) || 524288;
  }
  input.placeholder = String(spec.label || spec.name || "");
  input.value = seed === undefined || seed === null ? "" : String(seed);
  input.addEventListener("input", () => emit(input.value));
  stop(input);
  field.appendChild(input);
  return field;
}

/** The whole widget for one input node: label, field, and the result view. */
function buildNodeInputWidget(el, spec, emit) {
  while (el.firstChild) el.removeChild(el.firstChild);
  const label = document.createElement("div");
  label.className = "zgu-node-input-label";
  const name = document.createElement("span");
  name.textContent = String(spec.label || spec.name || "input");
  label.appendChild(name);
  if (spec.optional) {
    const optional = document.createElement("span");
    optional.className = "optional";
    optional.textContent = "(optional)";
    label.appendChild(optional);
  }
  el.appendChild(label);
  el.appendChild(buildNodeInputField(spec, emit));
  if (spec.result) {
    el.appendChild(buildNodeInputResult(spec.result, () => { el.dataset.mode = "field"; }));
    el.dataset.mode = "result";
  } else {
    el.dataset.mode = "field";
  }
  // The grip — same look as a preview cell's, hidden by CSS when read-only.
  // The widget's own pointerdown listener recognises it by class.
  if (NODE_INPUT_RESIZABLE_KINDS.includes(String(spec.kind || ""))) {
    const grip = document.createElement("div");
    grip.className = "zgu-node-input-resize";
    grip.title = "Drag to resize";
    el.appendChild(grip);
  }
}

/** The widest cell in a preview panel — the panel's own width. */
function nodePreviewPanelWidth(el) {
  let width = 0;
  for (const cellEl of el.querySelectorAll(".zgu-node-preview-cell")) {
    width = Math.max(width, Number(cellEl.dataset.width) || cellEl.offsetWidth || 0);
  }
  return width || NODE_PREVIEW_WIDTH;
}

/**
 * Place a node's under-box slot — its input widget, then its preview panel —
 * below the node's title, so nothing covers the name, and centred on the
 * node's centre. Every place that moves a node ends up here.
 *
 * Centred means: each element's x is `node.x + boxWidth/2 − width/2`. The
 * preview panel is as wide as its widest cell, and its cells are centred
 * within it (`align-items: center`), so an `in` and an `out` cell of
 * different widths are each centred on the node on their own. The slot's
 * height is read off the DOM, so a resized cell moves whatever sits below.
 */
function positionNodePreviewEl(node) {
  if (!node || (!node.previewEl && !node.inputEl)) return;
  const box = node.el;
  const boxWidth = (box && box.offsetWidth) || 84;
  const boxHeight = (box && box.offsetHeight) || node.zguNodeHeight || 84;
  const labelEl = box ? box.querySelector(".zgu-node-label") : null;
  const labelHeight = labelEl ? labelEl.offsetHeight : 0;
  let top = node.y + boxHeight + (labelHeight ? labelHeight + 9 : 0) + NODE_PREVIEW_GAP;
  const centre = node.x + boxWidth / 2;
  for (const el of [node.inputEl, node.previewEl]) {
    if (!el) continue;
    const width =
      el === node.previewEl ? nodePreviewPanelWidth(el) : Number(el.dataset.width) || NODE_PREVIEW_WIDTH;
    el.style.width = `${width}px`;
    el.style.transform = `translate(${Math.round(centre - width / 2)}px, ${Math.round(top)}px)`;
    top += (el.offsetHeight || 0) + NODE_PREVIEW_GAP;
  }
}

const NOTE_MIN_WIDTH = 120;
const NOTE_MIN_HEIGHT = 60;
const NOTE_DEFAULT_WIDTH = 280;
const NOTE_DEFAULT_HEIGHT = 120;
// The palette's eight; each has a `.zgu-note[data-color]` rule (slate is
// the default grey). The DSL's `--color` takes any word — one outside this
// list draws grey and keeps its name.
const NOTE_COLORS = ["slate", "amber", "blue", "green", "rose", "violet", "teal", "orange"];

export class GraphCanvasUI {
  constructor(root, graph, options = {}) {
    this.root = root;
    this.graph = graph;
    this.options = options;
    this.readOnly = options.readOnly === true;
    this.snapToGrid = options.snapToGrid !== false;
    this.gridSize = Number.isFinite(Number(options.gridSize)) && Number(options.gridSize) > 0
      ? Number(options.gridSize)
      : 30;
    this.edgeStyle = this.resolveInitialEdgeStyle(options.edgeStyle);
    this.selectionMode = this.resolveInitialSelectionMode(options.selectionMode);

    this.transform = { x: 0, y: 0, k: 1 };

    this.draggingNode = null;
    this.panning = false;
    this.connecting = false;
    this.connectOrigin = null;
    this.pendingDangling = null;
    this.selectedNode = null;
    this.selectedLink = null;
    this.selectedNote = null;
    this.draggingNote = null;
    this.resizingNote = null;
    // `{ node, which, el }` while a preview cell's grip is being dragged —
    // the preview twin of `resizingNote`.
    this.resizingPreview = null;
    // `{ node, el }` while an input widget's grip is being dragged.
    this.resizingInput = null;
    // The note whose colour palette is open, if any.
    this.paletteNote = null;
    this.startPos = { x: 0, y: 0 };
    this.initialTransform = { x: 0, y: 0 };
    this.activePointerId = null;
    this.wireUpdateFrame = null;

    this.pointerDownHandler = this.onPointerDown.bind(this);
    this.pointerMoveHandler = this.onPointerMove.bind(this);
    this.pointerUpHandler = this.onPointerUp.bind(this);
    this.wheelHandler = this.onWheel.bind(this);
    this.keyHandler = this.onKeyDown.bind(this);
    this.contextHandler = (event) => event.preventDefault();
    // Pressing a note captures the pointer on the workspace, so the browser
    // retargets the resulting click/dblclick to the workspace, never to the
    // note. Resolve the note under the cursor instead of trusting the target.
    this.dblClickHandler = (event) => {
      if (this.readOnly) return;
      const hit = document.elementFromPoint(event.clientX, event.clientY);
      if (hit && (hit.tagName === "TEXTAREA" || hit.tagName === "INPUT")) return;
      if (hit && (hit.classList.contains("zgu-note-color") || hit.closest(".zgu-note-palette"))) return;
      const note = this.findNoteByElement(hit);
      if (!note || note.el.classList.contains("editing")) return;
      event.preventDefault();
      event.stopPropagation();
      this.beginNoteEdit(note);
    };

    this.mountDom();
    this.initEvents();
    this.updateTransform();
  }

  resolveInitialEdgeStyle(rawStyle) {
    const valid = new Set(["bezier", "straight", "elbow", "rounded-elbow"]);
    const fromOption = String(rawStyle || "").trim();
    if (valid.has(fromOption)) return fromOption;
    try {
      const stored = window.localStorage?.getItem("zebflow.graph.edgeStyle") || "";
      if (valid.has(stored)) return stored;
    } catch (_err) {}
    return "bezier";
  }

  resolveInitialSelectionMode(rawMode) {
    const mode = String(rawMode || "").trim();
    return mode === "box" ? "box" : "normal";
  }

  mountDom() {
    this.root.classList.add("zgu-root");
    this.root.innerHTML = "";

    const headerOffset = this.options.showHeader === false ? 0 : 56;

    this.headerEl = document.createElement("div");
    this.headerEl.className = "zgu-header";
    if (this.options.showHeader === false) {
      this.headerEl.style.display = "none";
    }

    const title = document.createElement("div");
    title.className = "zgu-header-title";
    title.textContent = this.options.headerTitle || "Graph UI";
    this.headerEl.appendChild(title);

    this.sceneButtonsEl = document.createElement("div");
    this.sceneButtonsEl.style.display = "flex";
    this.sceneButtonsEl.style.gap = "8px";
    this.headerEl.appendChild(this.sceneButtonsEl);

    this.toolboxEl = document.createElement("div");
    this.toolboxEl.className = "zgu-toolbox";
    if (this.options.showToolbox === false) {
      this.toolboxEl.style.display = "none";
    }
    this.toolboxEl.innerHTML = "<h2>Toolbox</h2><div data-zgu-toolbox-buttons></div>";
    this.toolboxButtonsEl = this.toolboxEl.querySelector("[data-zgu-toolbox-buttons]");

    this.workspaceEl = document.createElement("div");
    this.workspaceEl.className = "zgu-workspace";
    this.workspaceEl.style.top = `${headerOffset}px`;
    this.workspaceEl.style.height = `calc(100% - ${headerOffset}px)`;

    this.gridEl = document.createElement("div");
    this.gridEl.className = "zgu-grid";
    this.workspaceEl.appendChild(this.gridEl);

    this.transformEl = document.createElement("div");
    this.transformEl.className = "zgu-transform";

    this.svgEl = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    this.svgEl.setAttribute("class", "zgu-svg");
    this.tempWire = document.createElementNS("http://www.w3.org/2000/svg", "path");
    this.tempWire.setAttribute("class", "zgu-wire temp");
    this.tempWire.style.display = "none";
    this.svgEl.appendChild(this.tempWire);

    this.nodesEl = document.createElement("div");
    this.nodesEl.className = "zgu-nodes";

    this.notesEl = document.createElement("div");
    this.notesEl.className = "zgu-notes";

    if (this.readOnly) {
      this.root.classList.add("zgu-readonly");
    }

    this.transformEl.appendChild(this.notesEl);
    this.transformEl.appendChild(this.svgEl);
    this.transformEl.appendChild(this.nodesEl);
    this.workspaceEl.appendChild(this.transformEl);
    this.mountCanvasControls();

    this.root.appendChild(this.headerEl);
    this.root.appendChild(this.toolboxEl);
    this.root.appendChild(this.workspaceEl);
  }

  mountCanvasControls() {
    this.canvasControlsEl = document.createElement("div");
    this.canvasControlsEl.className = "zgu-canvas-controls";
    this.canvasControlsEl.setAttribute("data-zgu-nodrag", "true");
    this.canvasControlsEl.addEventListener("pointerdown", (event) => {
      event.stopPropagation();
    });

    this.selectionControlEl = document.createElement("div");
    this.selectionControlEl.className = "zgu-selection-control";
    this.selectionControlEl.setAttribute("data-zgu-nodrag", "true");
    this.selectionControlEl.setAttribute("aria-label", "Selection mode");

    const selectionButtons = [
      [
        "normal",
        "Normal selection",
        `<svg viewBox="0 0 24 24" fill="none" aria-hidden="true" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 3l10 10-4 1-2 5-4-16Z" />
        </svg>`,
      ],
      [
        "box",
        "Box selection",
        `<svg viewBox="0 0 24 24" fill="none" aria-hidden="true" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <rect x="5" y="5" width="14" height="14" rx="1.5" stroke-dasharray="3 3" />
          <path d="M9 9h6v6H9z" />
        </svg>`,
      ],
      [
        "all",
        "Select all",
        `<svg viewBox="0 0 24 24" fill="none" aria-hidden="true" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <rect x="4" y="4" width="6" height="6" rx="1" />
          <rect x="14" y="4" width="6" height="6" rx="1" />
          <rect x="4" y="14" width="6" height="6" rx="1" />
          <rect x="14" y="14" width="6" height="6" rx="1" />
        </svg>`,
      ],
    ];
    selectionButtons.forEach(([mode, title, icon]) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "zgu-selection-button";
      button.dataset.selectionMode = mode;
      button.title = title;
      button.setAttribute("aria-label", title);
      button.innerHTML = icon;
      button.addEventListener("click", (event) => {
        event.preventDefault();
        event.stopPropagation();
        if (this.readOnly) return;
        if (mode === "all") {
          if (typeof this.options.onSelectAll === "function") {
            this.options.onSelectAll();
          }
          return;
        }
        this.setSelectionMode(mode);
      });
      this.selectionControlEl.appendChild(button);
    });
    this.canvasControlsEl.appendChild(this.selectionControlEl);

    this.autoTidyButtonEl = document.createElement("button");
    this.autoTidyButtonEl.type = "button";
    this.autoTidyButtonEl.className = "zgu-auto-tidy-button";
    this.autoTidyButtonEl.title = "Auto tidy graph";
    this.autoTidyButtonEl.setAttribute("aria-label", "Auto tidy graph");
    this.autoTidyButtonEl.innerHTML = `
      <svg viewBox="0 0 24 24" fill="none" aria-hidden="true" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M4 6h4" />
        <path d="M4 12h4" />
        <path d="M4 18h4" />
        <path d="M16 6h4" />
        <path d="M16 12h4" />
        <path d="M16 18h4" />
        <path d="M8 6h8" />
        <path d="M8 12h8" />
        <path d="M8 18h8" />
      </svg>
    `;
    this.autoTidyButtonEl.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (this.readOnly || typeof this.options.onAutoTidy !== "function") return;
      this.options.onAutoTidy();
    });
    this.canvasControlsEl.appendChild(this.autoTidyButtonEl);

    const styles = [
      ["bezier", "Curve"],
      ["straight", "Line"],
      ["elbow", "Elbow"],
      ["rounded-elbow", "Round"],
    ];
    this.edgeStyleControlEl = document.createElement("div");
    this.edgeStyleControlEl.className = "zgu-edge-style-control";
    this.edgeStyleControlEl.setAttribute("data-zgu-nodrag", "true");
    this.edgeStyleControlEl.setAttribute("aria-label", "Edge style");
    styles.forEach(([style, label]) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "zgu-edge-style-button";
      button.dataset.edgeStyle = style;
      button.textContent = label;
      button.title = `${label} connector`;
      button.addEventListener("click", (event) => {
        event.preventDefault();
        event.stopPropagation();
        this.setEdgeStyle(style);
      });
      this.edgeStyleControlEl.appendChild(button);
    });
    this.canvasControlsEl.appendChild(this.edgeStyleControlEl);
    this.workspaceEl.appendChild(this.canvasControlsEl);
    this.updateCanvasControls();
    this.updateEdgeStyleButtons();
    this.updateSelectionButtons();
  }

  updateCanvasControls() {
    if (this.autoTidyButtonEl) {
      this.autoTidyButtonEl.hidden = this.readOnly || typeof this.options.onAutoTidy !== "function";
    }
    if (this.selectionControlEl) {
      const hasSelectionActions =
        typeof this.options.onSelectionModeChange === "function" ||
        typeof this.options.onSelectAll === "function";
      this.selectionControlEl.hidden = this.readOnly || !hasSelectionActions;
    }
  }

  setSelectionMode(mode, options = {}) {
    const nextMode = mode === "box" ? "box" : "normal";
    if (this.selectionMode === nextMode) {
      this.updateSelectionButtons();
      return;
    }
    this.selectionMode = nextMode;
    this.updateSelectionButtons();
    if (options.emit !== false && typeof this.options.onSelectionModeChange === "function") {
      this.options.onSelectionModeChange(nextMode);
    }
  }

  updateSelectionButtons() {
    this.selectionControlEl?.querySelectorAll(".zgu-selection-button").forEach((button) => {
      const mode = button.dataset.selectionMode;
      button.classList.toggle("is-active", mode === this.selectionMode && mode !== "all");
    });
  }

  setEdgeStyle(style) {
    const valid = new Set(["bezier", "straight", "elbow", "rounded-elbow"]);
    if (!valid.has(style) || this.edgeStyle === style) return;
    this.edgeStyle = style;
    try {
      window.localStorage?.setItem("zebflow.graph.edgeStyle", style);
    } catch (_err) {}
    this.updateEdgeStyleButtons();
    this.updateWires();
  }

  updateEdgeStyleButtons() {
    this.edgeStyleControlEl?.querySelectorAll(".zgu-edge-style-button").forEach((button) => {
      button.classList.toggle("is-active", button.dataset.edgeStyle === this.edgeStyle);
    });
  }

  initEvents() {
    this.workspaceEl.addEventListener("pointerdown", this.pointerDownHandler);
    window.addEventListener("pointermove", this.pointerMoveHandler);
    window.addEventListener("pointerup", this.pointerUpHandler);
    this.workspaceEl.addEventListener("wheel", this.wheelHandler, { passive: false });
    window.addEventListener("keydown", this.keyHandler);
    this.workspaceEl.addEventListener("contextmenu", this.contextHandler);
    this.workspaceEl.addEventListener("dblclick", this.dblClickHandler);
    // A press outside the workspace (the toolbar, a dialog) closes an open
    // palette too; presses inside are handled in `onPointerDown`.
    this.paletteCloseHandler = (event) => {
      if (!this.paletteNote) return;
      const target = event.target;
      if (target && target.closest && (target.closest(".zgu-note-palette") || target.classList.contains("zgu-note-color"))) return;
      this.toggleNotePalette(this.paletteNote, false);
    };
    document.addEventListener("pointerdown", this.paletteCloseHandler, true);
  }

  destroy() {
    document.removeEventListener("pointerdown", this.paletteCloseHandler, true);
    this.workspaceEl.removeEventListener("pointerdown", this.pointerDownHandler);
    window.removeEventListener("pointermove", this.pointerMoveHandler);
    window.removeEventListener("pointerup", this.pointerUpHandler);
    this.workspaceEl.removeEventListener("wheel", this.wheelHandler);
    window.removeEventListener("keydown", this.keyHandler);
    this.workspaceEl.removeEventListener("contextmenu", this.contextHandler);
    this.workspaceEl.removeEventListener("dblclick", this.dblClickHandler);
    if (this.wireUpdateFrame != null) {
      window.cancelAnimationFrame(this.wireUpdateFrame);
      this.wireUpdateFrame = null;
    }
  }

  resetCamera() {
    const w = this.workspaceEl.clientWidth;
    const h = this.workspaceEl.clientHeight;
    this.transform = { x: w / 2 - 380, y: h / 2 - 200, k: 1 };
    this.updateTransform();
  }

  addNodeToDOM(node) {
    if (this.snapToGrid) {
      this.snapNodePosition(node);
    }
    node.buildDOM(this.nodesEl);
    this.updateWires();
  }

  /**
   * Draw (or, for an empty `data`, remove) the preview panel under one node.
   *
   * `data` is `{ in?: PreviewCell, out?: PreviewCell }`; `in` is drawn above
   * `out`. Rebuilding only on a changed signature keeps a playing video from
   * restarting on every re-render.
   */
  setNodePreview(node, data) {
    if (!node) return;
    const cells = [];
    if (data && data.in) cells.push(["in", data.in]);
    if (data && data.out) cells.push(["out", data.out]);
    if (!cells.length) {
      // The preview half only — the Run-form field in the same slot stays.
      node.detachPreview?.();
      return;
    }
    if (!node.previewEl) {
      const el = document.createElement("div");
      el.className = "zgu-node-preview";
      el.dataset.nodeId = String(node.id);
      // The canvas drags whatever is pressed; a preview is a thing you look
      // at and click, not a handle for the node. Its one handle is the
      // resize grip, which starts a resize here — the event never reaches
      // the workspace, so `onPointerDown` cannot see it the way it sees a
      // note's grip.
      el.setAttribute("data-zgu-nodrag", "true");
      el.addEventListener("pointerdown", (event) => {
        event.stopPropagation();
        if (event.button === 0 && event.target.classList.contains("zgu-node-preview-resize")) {
          this.beginPreviewResize(node, event.target.closest(".zgu-node-preview-cell"), event);
        }
      });
      this.nodesEl.appendChild(el);
      node.previewEl = el;
    }
    const el = node.previewEl;
    el.dataset.nodeSlug = String(node.zfPipelineNodeId || "");
    const signature = JSON.stringify(cells);
    if (el.dataset.signature !== signature) {
      el.dataset.signature = signature;
      while (el.firstChild) el.removeChild(el.firstChild);
      for (const [which, cell] of cells) {
        const cellEl = buildNodePreviewCell(cell, which);
        cellEl.addEventListener("click", (event) => {
          event.stopPropagation();
          // Releasing the grip is not a click on the preview.
          if (event.target.classList.contains("zgu-node-preview-resize")) return;
          const open = this.options.onPreviewOpen;
          if (typeof open === "function") {
            open(String(node.zfPipelineNodeId || node.id), which);
          }
        });
        el.appendChild(cellEl);
      }
    }
    positionNodePreviewEl(node);
  }

  /**
   * Start dragging one preview cell's grip — the same pointer-capture flow
   * as a note's grip: capture on the workspace, remember where the pointer
   * and the size started, then `onPointerMove` resizes and `onPointerUp`
   * snaps and reports.
   */
  beginPreviewResize(node, cellEl, event) {
    if (this.readOnly || !node || !cellEl) return;
    event.preventDefault();
    this.resizingPreview = { node, el: cellEl, which: String(cellEl.dataset.which || "out") };
    this.capturePointer(event);
    this.startPos = { x: event.clientX, y: event.clientY };
    this.startSize = {
      width: Number(cellEl.dataset.width) || cellEl.offsetWidth || NODE_PREVIEW_WIDTH,
      height: Number(cellEl.dataset.height) || nodePreviewHeight(cellEl.dataset.as),
    };
    this.clearSelection();
    cellEl.classList.add("resizing");
  }

  /** Clamp, apply and re-centre a cell mid-drag; `snap` on release. */
  resizePreviewCell(resizing, width, height, snap) {
    let w = clamp(Math.round(width), NODE_PREVIEW_MIN_WIDTH, NODE_PREVIEW_MAX_WIDTH);
    let h = clamp(Math.round(height), NODE_PREVIEW_MIN_HEIGHT, NODE_PREVIEW_MAX_HEIGHT);
    if (snap && this.snapToGrid) {
      w = clamp(this.snapCoordinate(w), NODE_PREVIEW_MIN_WIDTH, NODE_PREVIEW_MAX_WIDTH);
      h = clamp(this.snapCoordinate(h), NODE_PREVIEW_MIN_HEIGHT, NODE_PREVIEW_MAX_HEIGHT);
    }
    applyNodePreviewSize(resizing.el, w, h);
    positionNodePreviewEl(resizing.node);
    return { width: w, height: h };
  }

  /**
   * Draw (or, for an empty `spec`, remove) the input widget under one node.
   *
   * Rebuilt only when the declaration or the run result changes; `value`
   * seeds a rebuild and `invalid` only toggles a class, so typing and
   * validation never rebuild the field under the cursor.
   */
  setNodeInput(node, spec) {
    if (!node) return;
    if (!spec || typeof spec !== "object") {
      node.detachInput?.();
      return;
    }
    if (!node.inputEl) {
      const el = document.createElement("div");
      el.className = "zgu-node-input";
      el.dataset.nodeId = String(node.id);
      el.setAttribute("data-zgu-nodrag", "true");
      // The widget swallows the press (typing never drags the node); its one
      // handle is the grip, which starts a resize here, as a preview's does.
      el.addEventListener("pointerdown", (event) => {
        event.stopPropagation();
        if (event.button === 0 && event.target.classList.contains("zgu-node-input-resize")) {
          this.beginInputResize(node, el, event);
        }
      });
      el.addEventListener("click", (event) => event.stopPropagation());
      this.nodesEl.appendChild(el);
      node.inputEl = el;
    }
    const el = node.inputEl;
    const pipelineNodeId = String(node.zfPipelineNodeId || node.id);
    el.dataset.nodeSlug = pipelineNodeId;
    el.dataset.kind = String(spec.kind || "");
    el.dataset.name = String(spec.name || "");
    const { value: _value, invalid: _invalid, width: _width, height: _height, ...stable } = spec;
    const signature = JSON.stringify(stable);
    if (el.dataset.signature !== signature) {
      el.dataset.signature = signature;
      buildNodeInputWidget(el, spec, (value) => {
        const change = this.options.onInputChange;
        if (typeof change === "function") change(pipelineNodeId, value);
      });
    }
    // The stored size is applied when the host's copy of it changes, never
    // on every sync: after a drag the DOM holds the new size until the host
    // stores it and hands it back (a Save Draft reloads the scene with it).
    const size = nodeInputSize(spec);
    const sizeKey = size ? `${size.width}x${size.height}` : "";
    if (el.dataset.sizeKey !== sizeKey && !(this.resizingInput && this.resizingInput.el === el)) {
      el.dataset.sizeKey = sizeKey;
      applyNodeInputSize(el, size);
    }
    el.classList.toggle("invalid", !!spec.invalid);
    if (spec.invalid) el.dataset.mode = "field";
    positionNodePreviewEl(node);
  }

  /** Start dragging one input widget's grip — the preview's flow, one cell. */
  beginInputResize(node, el, event) {
    if (this.readOnly || !node || !el) return;
    event.preventDefault();
    this.resizingInput = { node, el };
    this.capturePointer(event);
    this.startPos = { x: event.clientX, y: event.clientY };
    this.startSize = {
      width: Number(el.dataset.width) || el.offsetWidth || NODE_PREVIEW_WIDTH,
      height: Number(el.dataset.height) || el.offsetHeight || NODE_INPUT_MIN_HEIGHT,
    };
    this.clearSelection();
    el.classList.add("resizing");
  }

  /** Clamp, apply and re-centre a widget mid-drag; `snap` on release. */
  resizeInputWidget(resizing, width, height, snap) {
    let w = clamp(Math.round(width), NODE_INPUT_MIN_WIDTH, NODE_INPUT_MAX_WIDTH);
    let h = clamp(Math.round(height), NODE_INPUT_MIN_HEIGHT, NODE_INPUT_MAX_HEIGHT);
    if (snap && this.snapToGrid) {
      w = clamp(this.snapCoordinate(w), NODE_INPUT_MIN_WIDTH, NODE_INPUT_MAX_WIDTH);
      h = clamp(this.snapCoordinate(h), NODE_INPUT_MIN_HEIGHT, NODE_INPUT_MAX_HEIGHT);
    }
    applyNodeInputSize(resizing.el, { width: w, height: h });
    positionNodePreviewEl(resizing.node);
    return { width: w, height: h };
  }

  /**
   * Draw (or, for an empty `status`, remove) the run badge at one node's
   * top-right. It lives inside the node's box, so it moves with the box for
   * free; a box rebuilt by a scene load gets it back on the next sync.
   */
  setNodeStatus(node, status) {
    if (!node) return;
    const state =
      status && typeof status === "object" ? String(status.state || "").toLowerCase() : "";
    if (!NODE_STATUS_STATES.includes(state) || !node.el) {
      node.detachStatus?.();
      return;
    }
    if (!node.statusEl || node.statusEl.parentNode !== node.el || node.statusEl.childNodes.length !== 3) {
      node.statusEl?.remove();
      const el = document.createElement("div");
      el.className = "zgu-node-status";
      const mark = document.createElement("span");
      mark.className = "zgu-node-status-mark";
      const count = document.createElement("span");
      count.className = "zgu-node-status-count";
      const ms = document.createElement("span");
      ms.className = "zgu-node-status-ms";
      el.appendChild(mark);
      el.appendChild(count);
      el.appendChild(ms);
      node.el.appendChild(el);
      node.statusEl = el;
    }
    const el = node.statusEl;
    el.dataset.state = state;
    el.childNodes[0].textContent = NODE_STATUS_MARKS[state] || "";
    // The count rides along while the node runs again: the wait between
    // attempts happens inside that run, and "2/40" is what it is waiting for.
    el.childNodes[1].textContent = state === "retry" || state === "running" ? formatNodeStatusCount(status) : "";
    el.childNodes[2].textContent = formatNodeStatusDuration(status.duration_ms);
    el.title = nodeStatusTitle(state, status);
  }

  addNoteToDOM(note) {
    note.width = Math.max(NOTE_MIN_WIDTH, Number(note.width) || NOTE_DEFAULT_WIDTH);
    note.height = Math.max(NOTE_MIN_HEIGHT, Number(note.height) || NOTE_DEFAULT_HEIGHT);
    note.x = Number(note.x) || 0;
    note.y = Number(note.y) || 0;
    if (this.snapToGrid) {
      note.x = this.snapCoordinate(note.x);
      note.y = this.snapCoordinate(note.y);
    }
    const el = document.createElement("div");
    el.className = "zgu-note";
    el.dataset.uid = String(note.uid);
    el.title = this.readOnly ? "" : "Drag to move · double-click to edit · right-click or Delete to remove";
    const body = document.createElement("div");
    body.className = "zgu-note-body";
    const edit = document.createElement("textarea");
    edit.className = "zgu-note-edit";
    edit.spellcheck = false;
    const grip = document.createElement("div");
    grip.className = "zgu-note-resize";
    // The colour dot and its palette; `onPointerDown` recognises both by
    // class before the press becomes a drag.
    const dot = document.createElement("div");
    dot.className = "zgu-note-color";
    dot.title = "Colour";
    const palette = document.createElement("div");
    palette.className = "zgu-note-palette";
    for (const color of NOTE_COLORS) {
      const swatch = document.createElement("div");
      swatch.className = "zgu-note-swatch";
      swatch.dataset.color = color;
      swatch.title = color;
      palette.appendChild(swatch);
    }
    el.appendChild(body);
    el.appendChild(edit);
    el.appendChild(grip);
    el.appendChild(dot);
    el.appendChild(palette);
    note.el = el;
    note.bodyEl = body;
    note.editEl = edit;
    note.paletteEl = palette;
    this.renderNote(note);
    this.notesEl.appendChild(el);

    if (!this.readOnly) {
      edit.addEventListener("blur", () => this.commitNoteEdit(note));
      edit.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          edit.value = note.text;
          edit.blur();
        } else if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
          event.preventDefault();
          edit.blur();
        }
        event.stopPropagation();
      });
    }
    return el;
  }

  renderNote(note) {
    if (!note.el) return;
    note.el.style.transform = `translate(${note.x}px, ${note.y}px)`;
    note.el.style.width = `${note.width}px`;
    note.el.style.height = `${note.height}px`;
    const color = String(note.color || "").trim().toLowerCase() || "slate";
    note.el.dataset.color = color;
    if (note.paletteEl) {
      for (const swatch of note.paletteEl.children) {
        swatch.classList.toggle("selected", swatch.dataset.color === color);
      }
    }
    note.bodyEl.innerHTML = renderNoteMarkdown(note.text) || '<p style="opacity:.5">(empty note — double-click to write)</p>';
  }

  /** Open one note's palette (closing any other's), or close it when `open` is false. */
  toggleNotePalette(note, open) {
    if (this.paletteNote && this.paletteNote !== note) {
      this.paletteNote.el?.classList.remove("palette-open");
      this.paletteNote = null;
    }
    if (!note || !note.el) return;
    const next = open === undefined ? !note.el.classList.contains("palette-open") : !!open;
    note.el.classList.toggle("palette-open", next);
    this.paletteNote = next ? note : null;
  }

  /** Pick a colour: the note re-renders and the host hears `onNoteChange`, as for a move. */
  setNoteColor(note, color) {
    if (!note) return;
    note.color = String(color || "").trim().toLowerCase();
    this.renderNote(note);
    this.toggleNotePalette(note, false);
    if (typeof this.options.onNoteChange === "function") {
      this.options.onNoteChange(note);
    }
  }

  beginNoteEdit(note) {
    if (this.readOnly || !note.el) return;
    note.el.classList.add("editing");
    note.editEl.value = note.text || "";
    note.editEl.focus();
    note.editEl.setSelectionRange(note.editEl.value.length, note.editEl.value.length);
  }

  commitNoteEdit(note) {
    if (!note.el || !note.el.classList.contains("editing")) return;
    note.text = note.editEl.value;
    note.el.classList.remove("editing");
    this.renderNote(note);
    if (typeof this.options.onNoteChange === "function") {
      this.options.onNoteChange(note);
    }
  }

  removeNote(note) {
    if (this.selectedNote === note) {
      this.selectedNote = null;
    }
    if (this.paletteNote === note) {
      this.paletteNote = null;
    }
    this.graph.removeNote(note);
  }

  findNoteByElement(target) {
    const noteEl = target && target.closest ? target.closest(".zgu-note") : null;
    if (!noteEl) return null;
    return this.graph.notes.find((note) => note.uid === Number(noteEl.dataset.uid)) || null;
  }

  clearSVG() {
    this.svgEl.querySelectorAll(".zgu-wire:not(.temp)").forEach((path) => path.remove());
    this.svgEl.querySelectorAll(".zgu-dangling-plus").forEach((group) => group.remove());
  }

  updateTransform() {
    const dpr = window.devicePixelRatio || 1;
    const snap = (value) => Math.round(value * dpr) / dpr;
    const x = snap(this.transform.x);
    const y = snap(this.transform.y);
    const gridSize = snap(30 * this.transform.k);
    this.transformEl.style.transform = `translate3d(${x}px, ${y}px, 0) scale(${this.transform.k})`;
    this.gridEl.style.backgroundPosition = `${x}px ${y}px`;
    this.gridEl.style.backgroundSize = `${gridSize}px ${gridSize}px`;
  }

  drawBezier(x1, y1, x2, y2) {
    const dist = Math.max(Math.abs(x2 - x1) * 0.5, 50);
    return `M ${x1} ${y1} C ${x1 + dist} ${y1}, ${x2 - dist} ${y2}, ${x2} ${y2}`;
  }

  drawStraight(x1, y1, x2, y2) {
    return `M ${x1} ${y1} L ${x2} ${y2}`;
  }

  elementWorldRect(el) {
    if (!el) return null;
    const rect = el.getBoundingClientRect();
    const transformRect = this.transformEl.getBoundingClientRect();
    return {
      left: (rect.left - transformRect.left) / this.transform.k,
      right: (rect.right - transformRect.left) / this.transform.k,
      top: (rect.top - transformRect.top) / this.transform.k,
      bottom: (rect.bottom - transformRect.top) / this.transform.k,
      width: rect.width / this.transform.k,
      height: rect.height / this.transform.k,
    };
  }

  nodeObstacleRect(nodeEl) {
    const node = this.elementWorldRect(nodeEl);
    if (!node) return null;
    const label = this.elementWorldRect(nodeEl.querySelector(".zgu-node-label"));
    if (!label) return node;
    return {
      left: Math.min(node.left, label.left),
      right: Math.max(node.right, label.right),
      top: Math.min(node.top, label.top),
      bottom: Math.max(node.bottom, label.bottom),
      width: Math.max(node.right, label.right) - Math.min(node.left, label.left),
      height: Math.max(node.bottom, label.bottom) - Math.min(node.top, label.top),
    };
  }

  elbowPoints(x1, y1, x2, y2, sourceEl = null, targetEl = null) {
    const exit = 56;
    const entry = 36;
    const laneGap = 48;
    const dx = x2 - x1;
    const dy = y2 - y1;
    const sourceRect = this.nodeObstacleRect(sourceEl);
    const targetRect = this.nodeObstacleRect(targetEl);
    const sourceNodeRect = this.elementWorldRect(sourceEl);
    const exitX = sourceRect ? sourceRect.right + laneGap : x1 + exit;
    const entryX = targetRect ? targetRect.left - laneGap : x2 - entry;
    const sourceHeight = sourceNodeRect?.height || sourceRect?.height || 84;
    const sourceWidth = sourceNodeRect?.width || sourceRect?.width || 84;
    const significantGap = sourceHeight * 0.75;
    const targetFarAbove = Boolean(sourceRect && targetRect && targetRect.bottom < sourceRect.top - significantGap);
    const targetFarBelow = Boolean(sourceRect && targetRect && targetRect.top > sourceRect.bottom + significantGap);
    const rightClearance = sourceRect && targetRect ? targetRect.left - sourceRect.right : 0;
    const hasNormalRightRunway = rightClearance > sourceWidth * 0.75;
    const verticalOverlap = sourceRect && targetRect
      ? Math.min(sourceRect.bottom, targetRect.bottom) - Math.max(sourceRect.top, targetRect.top)
      : 0;
    const nearVerticalBand = verticalOverlap > -sourceHeight * 0.75;
    const needsLane = dx < exit + entry || Boolean(sourceRect && targetRect && targetRect.left < sourceRect.right && nearVerticalBand);
    const needsGapLane = Boolean(sourceRect && targetRect && (targetFarAbove || targetFarBelow) && !hasNormalRightRunway);

    if (needsGapLane) {
      const laneY = targetFarAbove
        ? (targetRect.bottom + sourceRect.top) / 2
        : (sourceRect.bottom + targetRect.top) / 2;
      return [
        { x: x1, y: y1 },
        { x: exitX, y: y1 },
        { x: exitX, y: laneY },
        { x: entryX, y: laneY },
        { x: entryX, y: y2 },
        { x: x2, y: y2 },
      ];
    }

    if (needsLane) {
      const laneY = dy < 0
        ? Math.min(sourceRect?.top ?? y1, targetRect?.top ?? y2) - laneGap
        : Math.max(sourceRect?.bottom ?? y1, targetRect?.bottom ?? y2) + laneGap;
      return [
        { x: x1, y: y1 },
        { x: exitX, y: y1 },
        { x: exitX, y: laneY },
        { x: entryX, y: laneY },
        { x: entryX, y: y2 },
        { x: x2, y: y2 },
      ];
    }

    const midX = sourceRect && targetRect
      ? (sourceRect.right + targetRect.left) / 2
      : x1 + dx / 2;
    return [
      { x: x1, y: y1 },
      { x: midX, y: y1 },
      { x: midX, y: y2 },
      { x: x2, y: y2 },
    ];
  }

  drawElbow(x1, y1, x2, y2, sourceEl = null, targetEl = null) {
    const points = this.elbowPoints(x1, y1, x2, y2, sourceEl, targetEl);
    return points
      .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
      .join(" ");
  }

  drawRoundedPolyline(points, radius = 12) {
    const cleaned = points.filter((point, index) => {
      if (index === 0) return true;
      const prev = points[index - 1];
      return prev.x !== point.x || prev.y !== point.y;
    });
    if (cleaned.length < 3) {
      return cleaned
        .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
        .join(" ");
    }
    const parts = [`M ${cleaned[0].x} ${cleaned[0].y}`];
    for (let i = 1; i < cleaned.length - 1; i++) {
      const prev = cleaned[i - 1];
      const current = cleaned[i];
      const next = cleaned[i + 1];
      const prevDist = Math.hypot(current.x - prev.x, current.y - prev.y);
      const nextDist = Math.hypot(next.x - current.x, next.y - current.y);
      const r = Math.min(radius, prevDist / 2, nextDist / 2);
      if (r <= 0) {
        parts.push(`L ${current.x} ${current.y}`);
        continue;
      }
      const before = {
        x: current.x + ((prev.x - current.x) / prevDist) * r,
        y: current.y + ((prev.y - current.y) / prevDist) * r,
      };
      const after = {
        x: current.x + ((next.x - current.x) / nextDist) * r,
        y: current.y + ((next.y - current.y) / nextDist) * r,
      };
      parts.push(`L ${before.x} ${before.y}`);
      parts.push(`Q ${current.x} ${current.y} ${after.x} ${after.y}`);
    }
    const last = cleaned[cleaned.length - 1];
    parts.push(`L ${last.x} ${last.y}`);
    return parts.join(" ");
  }

  drawRoundedElbow(x1, y1, x2, y2, sourceEl = null, targetEl = null) {
    return this.drawRoundedPolyline(this.elbowPoints(x1, y1, x2, y2, sourceEl, targetEl), 12);
  }

  drawEdgePath(x1, y1, x2, y2, sourceEl = null, targetEl = null) {
    if (this.edgeStyle === "straight") {
      return this.drawStraight(x1, y1, x2, y2);
    }
    if (this.edgeStyle === "elbow") {
      return this.drawElbow(x1, y1, x2, y2, sourceEl, targetEl);
    }
    if (this.edgeStyle === "rounded-elbow") {
      return this.drawRoundedElbow(x1, y1, x2, y2, sourceEl, targetEl);
    }
    return this.drawBezier(x1, y1, x2, y2);
  }

  getPortCenter(nodeEl, type, index) {
    const port = nodeEl.querySelector(`.zgu-port.${type}[data-index="${index}"]`);
    if (!port) {
      return { x: 0, y: 0 };
    }
    const rect = port.getBoundingClientRect();
    const transformRect = this.transformEl.getBoundingClientRect();
    return {
      x: (rect.left - transformRect.left + rect.width / 2) / this.transform.k,
      y: (rect.top - transformRect.top + rect.height / 2) / this.transform.k,
    };
  }

  snapCoordinate(value) {
    return Math.round(value / this.gridSize) * this.gridSize;
  }

  snapNodePosition(node) {
    if (!node) {
      return;
    }
    node.x = this.snapCoordinate(node.x);
    node.y = this.snapCoordinate(node.y);
    node.applyTransform();
  }

  updateWires() {
    this.svgEl.querySelectorAll(".zgu-wire:not(.temp)").forEach((path) => {
      if (!this.graph.links.find((link) => link.id === path.id)) {
        path.remove();
      }
    });

    this.graph.links.forEach((link) => {
      let path = this.svgEl.querySelector(`#${CSS.escape(link.id)}`);
      if (!path) {
        path = document.createElementNS("http://www.w3.org/2000/svg", "path");
        path.id = link.id;
        path.classList.add("zgu-wire");
        if (link.options.animated) {
          path.classList.add("animated");
          path.style.strokeDasharray = link.options.dashArray;
          path.style.setProperty("--zgu-anim-speed", link.options.speed);
        }
        if (link.options.color) {
          path.style.stroke = link.options.color;
        }
        if (link.options.thickness) {
          path.style.strokeWidth = String(link.options.thickness);
        }
        if (link.options.opacity) {
          path.style.opacity = String(link.options.opacity);
        }
        if (!this.readOnly) {
          path.addEventListener("contextmenu", (event) => {
            event.preventDefault();
            this.graph.links = this.graph.links.filter((next) => next.id !== link.id);
            path.remove();
          });
        }
        this.svgEl.appendChild(path);
      }

      const source = this.graph.nodes.find((node) => node.id === link.fromNode);
      const target = this.graph.nodes.find((node) => node.id === link.toNode);
      if (!source?.el || !target?.el) {
        return;
      }
      const start = this.getPortCenter(source.el, "out", link.fromSlot);
      const end = this.getPortCenter(target.el, "in", link.toSlot);
      path.setAttribute("d", this.drawEdgePath(start.x, start.y, end.x, end.y, source.el, target.el));
    });
    this.updateDanglingPlus();
  }

  scheduleWiresUpdate() {
    if (this.wireUpdateFrame != null) {
      return;
    }
    this.wireUpdateFrame = window.requestAnimationFrame(() => {
      this.wireUpdateFrame = null;
      this.updateWires();
    });
  }

  getConnectedOutputKeys() {
    const keys = new Set();
    this.graph.links.forEach((link) => {
      keys.add(`${link.fromNode}:${link.fromSlot}`);
    });
    return keys;
  }

  clientToWorld(clientX, clientY) {
    const rect = this.transformEl.getBoundingClientRect();
    return {
      x: (clientX - rect.left) / this.transform.k,
      y: (clientY - rect.top) / this.transform.k,
    };
  }

  makeOutputAddPayload(node, slot, position = null) {
    const output = Array.isArray(node.outputs) ? node.outputs[slot] : null;
    return {
      graphNodeId: node.id,
      zfKind: node.zfKind || "",
      zfPipelineNodeId: node.zfPipelineNodeId || "",
      title: node.title,
      x: Number.isFinite(Number(position?.x)) ? Number(position.x) : node.x,
      y: Number.isFinite(Number(position?.y)) ? Number(position.y) : node.y,
      outputSlot: slot,
      outputPin: output?.name || "out",
      _raw: node,
    };
  }

  startConnectionFromOutput(node, slot, danglingEl = null) {
    if (!node?.el) return;
    const start = this.getPortCenter(node.el, "out", slot);
    this.connecting = true;
    this.connectOrigin = {
      nodeId: node.id,
      slot,
      el: node.el,
      fromDangling: danglingEl,
    };
    if (danglingEl) {
      danglingEl.style.display = "none";
    }
    this.tempWire.style.display = "block";
    this.tempWire.setAttribute("d", this.drawEdgePath(start.x, start.y, start.x, start.y));
  }

  updateDanglingPlus() {
    const onOutputAdd = this.options.onOutputAdd;
    const readOnly = this.readOnly || typeof onOutputAdd !== "function";
    const svgNS = "http://www.w3.org/2000/svg";
    const connected = this.getConnectedOutputKeys();
    const valid = new Set();
    const stubLen = 50;
    const plusRadius = 10;

    if (readOnly) {
      this.svgEl.querySelectorAll(".zgu-dangling-plus").forEach((group) => group.remove());
      return;
    }

    this.graph.nodes.forEach((node) => {
      if (!node?.el || !Array.isArray(node.outputs)) return;
      node.outputs.forEach((_output, slot) => {
        const key = `${node.id}:${slot}`;
        if (connected.has(key)) return;
        const center = this.getPortCenter(node.el, "out", slot);
        if (!Number.isFinite(center.x) || !Number.isFinite(center.y)) return;
        valid.add(key);

        const endX = center.x + stubLen;
        const endY = center.y;
        let group = this.svgEl.querySelector(`.zgu-dangling-plus[data-key="${CSS.escape(key)}"]`);
        if (!group) {
          group = document.createElementNS(svgNS, "g");
          group.classList.add("zgu-dangling-plus");
          group.dataset.key = key;

          const wire = document.createElementNS(svgNS, "path");
          wire.classList.add("zgu-dangling-wire");
          group.appendChild(wire);

          const circle = document.createElementNS(svgNS, "circle");
          circle.classList.add("zgu-dangling-circle");
          circle.setAttribute("r", String(plusRadius));
          group.appendChild(circle);

          const hLine = document.createElementNS(svgNS, "line");
          hLine.classList.add("zgu-dangling-mark");
          hLine.dataset.axis = "h";
          group.appendChild(hLine);

          const vLine = document.createElementNS(svgNS, "line");
          vLine.classList.add("zgu-dangling-mark");
          vLine.dataset.axis = "v";
          group.appendChild(vLine);

          group.addEventListener("pointerdown", (event) => {
            event.preventDefault();
            event.stopPropagation();
            const [nodeIdRaw, slotRaw] = String(group.dataset.key || "").split(":");
            const source = this.graph.nodes.find((next) => next.id === Number(nodeIdRaw));
            if (!source) return;
            this.pendingDangling = {
              node: source,
              slot: Number(slotRaw),
              group,
              x: event.clientX,
              y: event.clientY,
              world: this.clientToWorld(event.clientX, event.clientY),
            };
          });

          this.svgEl.appendChild(group);
        }

        group.querySelector(".zgu-dangling-wire").setAttribute("d", `M ${center.x} ${center.y} L ${endX} ${endY}`);
        group.querySelector(".zgu-dangling-circle").setAttribute("cx", String(endX));
        group.querySelector(".zgu-dangling-circle").setAttribute("cy", String(endY));
        const hLine = group.querySelector('.zgu-dangling-mark[data-axis="h"]');
        const vLine = group.querySelector('.zgu-dangling-mark[data-axis="v"]');
        hLine.setAttribute("x1", String(endX - 4));
        hLine.setAttribute("y1", String(endY));
        hLine.setAttribute("x2", String(endX + 4));
        hLine.setAttribute("y2", String(endY));
        vLine.setAttribute("x1", String(endX));
        vLine.setAttribute("y1", String(endY - 4));
        vLine.setAttribute("x2", String(endX));
        vLine.setAttribute("y2", String(endY + 4));
      });
    });

    this.svgEl.querySelectorAll(".zgu-dangling-plus").forEach((group) => {
      if (!valid.has(group.dataset.key || "")) {
        group.remove();
      }
    });
  }

  capturePointer(event) {
    if (event.pointerId != null && typeof this.workspaceEl.setPointerCapture === "function") {
      try {
        this.workspaceEl.setPointerCapture(event.pointerId);
        this.activePointerId = event.pointerId;
      } catch (_err) {}
    }
  }

  releasePointer() {
    if (this.activePointerId != null && typeof this.workspaceEl.releasePointerCapture === "function") {
      try {
        this.workspaceEl.releasePointerCapture(this.activePointerId);
      } catch (_err) {}
    }
    this.activePointerId = null;
  }

  onPointerDown(event) {
    if (this.readOnly) {
      const target = event.target;
      if (event.button === 2) {
        event.preventDefault();
        return;
      }
      if (
        target.closest(".zgu-node") ||
        target.closest(".zgu-note") ||
        target.classList.contains("zgu-port") ||
        target.classList.contains("zgu-wire")
      ) {
        return;
      }
      this.panning = true;
      this.capturePointer(event);
      this.startPos = { x: event.clientX, y: event.clientY };
      this.initialTransform = { ...this.transform };
      this.clearSelection();
      return;
    }

    if (event.button === 2) {
      const nodeEl = event.target.closest(".zgu-node");
      if (nodeEl) {
        const node = this.graph.nodes.find((next) => next.id === Number(nodeEl.dataset.id));
        if (node) {
          this.graph.remove(node);
          this.updateWires();
        }
      }
      const note = this.findNoteByElement(event.target);
      if (note && !note.el.classList.contains("editing")) {
        this.removeNote(note);
      }
      event.preventDefault();
      return;
    }

    const target = event.target;

    // The colour dot opens the palette; a swatch picks; a press anywhere
    // else closes it (the document listener covers presses off the canvas).
    if (target.classList.contains("zgu-note-color")) {
      const note = this.findNoteByElement(target);
      event.preventDefault();
      event.stopPropagation();
      if (note) {
        this.clearSelection();
        this.selectedNote = note;
        note.el.classList.add("selected");
        this.toggleNotePalette(note);
      }
      return;
    }
    if (target.closest(".zgu-note-palette")) {
      const note = this.findNoteByElement(target);
      event.preventDefault();
      event.stopPropagation();
      if (note && target.classList.contains("zgu-note-swatch")) {
        this.setNoteColor(note, target.dataset.color);
      }
      return;
    }
    if (this.paletteNote) this.toggleNotePalette(this.paletteNote, false);

    if (target.classList.contains("zgu-note-resize")) {
      const note = this.findNoteByElement(target);
      if (note) {
        event.preventDefault();
        event.stopPropagation();
        this.resizingNote = note;
        this.capturePointer(event);
        this.startPos = { x: event.clientX, y: event.clientY };
        this.startSize = { width: note.width, height: note.height };
        this.clearSelection();
        this.selectedNote = note;
        note.el.classList.add("selected");
        return;
      }
    }

    if (target.classList.contains("zgu-wire") && !target.classList.contains("temp")) {
      event.preventDefault();
      event.stopPropagation();
      this.clearSelection();
      this.selectedLink = target.id;
      target.classList.add("selected");
      this.svgEl.appendChild(target);
      return;
    }

    if (target.classList.contains("zgu-port") && target.classList.contains("out")) {
      event.preventDefault();
      event.stopPropagation();
      const nodeEl = target.closest(".zgu-node");
      const node = this.graph.nodes.find((next) => next.id === Number(nodeEl.dataset.id));
      this.capturePointer(event);
      this.startConnectionFromOutput(node, Number(target.dataset.index));
      return;
    }

    if (target.closest("[data-zgu-nodrag='true']") || target.tagName === "INPUT" || target.tagName === "TEXTAREA") {
      return;
    }

    const pressedNote = this.findNoteByElement(target);
    if (pressedNote) {
      event.preventDefault();
      event.stopPropagation();
      this.draggingNote = pressedNote;
      this.capturePointer(event);
      this.startPos = { x: event.clientX, y: event.clientY };
      this.clearSelection();
      this.selectedNote = pressedNote;
      pressedNote.el.classList.add("selected");
      return;
    }

    const nodeEl = target.closest(".zgu-node");
    if (nodeEl) {
      event.preventDefault();
      event.stopPropagation();
      this.draggingNode = this.graph.nodes.find((node) => node.id === Number(nodeEl.dataset.id));
      this.capturePointer(event);
      this.startPos = { x: event.clientX, y: event.clientY };
      this.clearSelection();
      this.selectedNode = this.draggingNode;
      this.selectedNode.el.classList.add("selected");
      this.nodesEl.appendChild(nodeEl);
      return;
    }

    this.panning = true;
    this.capturePointer(event);
    this.startPos = { x: event.clientX, y: event.clientY };
    this.initialTransform = { ...this.transform };
    this.clearSelection();
  }

  onPointerMove(event) {
    if (this.activePointerId != null && event.pointerId != null && event.pointerId !== this.activePointerId) {
      return;
    }

    if (this.pendingDangling) {
      const dx = event.clientX - this.pendingDangling.x;
      const dy = event.clientY - this.pendingDangling.y;
      if (Math.abs(dx) >= 4 || Math.abs(dy) >= 4) {
        const pending = this.pendingDangling;
        this.pendingDangling = null;
        this.startConnectionFromOutput(pending.node, pending.slot, pending.group);
      } else {
        return;
      }
    }

    if (this.connecting) {
      const rect = this.transformEl.getBoundingClientRect();
      const mouseX = (event.clientX - rect.left) / this.transform.k;
      const mouseY = (event.clientY - rect.top) / this.transform.k;
      const start = this.getPortCenter(this.connectOrigin.el, "out", this.connectOrigin.slot);
      this.tempWire.setAttribute("d", this.drawEdgePath(start.x, start.y, mouseX, mouseY));
      return;
    }

    if (this.draggingNote) {
      const dx = (event.clientX - this.startPos.x) / this.transform.k;
      const dy = (event.clientY - this.startPos.y) / this.transform.k;
      this.draggingNote.x += dx;
      this.draggingNote.y += dy;
      this.draggingNote.el.style.transform = `translate(${this.draggingNote.x}px, ${this.draggingNote.y}px)`;
      this.startPos = { x: event.clientX, y: event.clientY };
      return;
    }

    if (this.resizingNote) {
      const dx = (event.clientX - this.startPos.x) / this.transform.k;
      const dy = (event.clientY - this.startPos.y) / this.transform.k;
      this.resizingNote.width = Math.max(NOTE_MIN_WIDTH, this.startSize.width + dx);
      this.resizingNote.height = Math.max(NOTE_MIN_HEIGHT, this.startSize.height + dy);
      this.resizingNote.el.style.width = `${this.resizingNote.width}px`;
      this.resizingNote.el.style.height = `${this.resizingNote.height}px`;
      return;
    }

    if (this.resizingPreview) {
      const dx = (event.clientX - this.startPos.x) / this.transform.k;
      const dy = (event.clientY - this.startPos.y) / this.transform.k;
      this.resizePreviewCell(this.resizingPreview, this.startSize.width + dx, this.startSize.height + dy, false);
      return;
    }

    if (this.resizingInput) {
      const dx = (event.clientX - this.startPos.x) / this.transform.k;
      const dy = (event.clientY - this.startPos.y) / this.transform.k;
      this.resizeInputWidget(this.resizingInput, this.startSize.width + dx, this.startSize.height + dy, false);
      return;
    }

    if (this.draggingNode) {
      const dx = (event.clientX - this.startPos.x) / this.transform.k;
      const dy = (event.clientY - this.startPos.y) / this.transform.k;
      this.draggingNode.x += dx;
      this.draggingNode.y += dy;
      this.draggingNode.applyTransform();
      this.scheduleWiresUpdate();
      this.startPos = { x: event.clientX, y: event.clientY };
      return;
    }

    if (this.panning) {
      const dx = event.clientX - this.startPos.x;
      const dy = event.clientY - this.startPos.y;
      this.transform.x = this.initialTransform.x + dx;
      this.transform.y = this.initialTransform.y + dy;
      this.updateTransform();
    }
  }

  onPointerUp(event) {
    if (this.activePointerId != null && event.pointerId != null && event.pointerId !== this.activePointerId) {
      return;
    }

    if (this.pendingDangling) {
      const pending = this.pendingDangling;
      this.pendingDangling = null;
      if (!this.readOnly && typeof this.options.onOutputAdd === "function") {
        this.options.onOutputAdd(this.makeOutputAddPayload(pending.node, pending.slot, pending.world));
      }
      this.releasePointer();
      return;
    }

    if (this.connecting) {
      this.connecting = false;
      this.tempWire.style.display = "none";
      let connected = false;
      const droppedOn = document.elementFromPoint(event.clientX, event.clientY);
      if (droppedOn?.classList.contains("zgu-port") && droppedOn.classList.contains("in")) {
        const targetNodeEl = droppedOn.closest(".zgu-node");
        const targetId = Number(targetNodeEl.dataset.id);
        const targetSlot = Number(droppedOn.dataset.index);
        if (targetId !== this.connectOrigin.nodeId) {
          this.graph.connect(this.connectOrigin.nodeId, this.connectOrigin.slot, targetId, targetSlot, this.options.defaultManualLinkOptions || {});
          this.updateWires();
          connected = true;
        }
      }
      if (!connected && !this.readOnly && typeof this.options.onOutputAdd === "function") {
        if (this.connectOrigin?.fromDangling) {
          this.connectOrigin.fromDangling.style.display = "";
        }
        this.options.onOutputAdd(
          this.makeOutputAddPayload(
            this.graph.nodes.find((node) => node.id === this.connectOrigin.nodeId),
            this.connectOrigin.slot,
            this.clientToWorld(event.clientX, event.clientY)
          )
        );
      }
      this.connectOrigin = null;
    }

    const draggedNode = this.draggingNode;
    this.draggingNode = null;
    if (draggedNode && this.snapToGrid) {
      this.snapNodePosition(draggedNode);
      this.updateWires();
    }
    const movedNote = this.draggingNote || this.resizingNote;
    this.draggingNote = null;
    this.resizingNote = null;
    if (movedNote) {
      if (this.snapToGrid) {
        movedNote.x = this.snapCoordinate(movedNote.x);
        movedNote.y = this.snapCoordinate(movedNote.y);
        movedNote.width = Math.max(NOTE_MIN_WIDTH, this.snapCoordinate(movedNote.width));
        movedNote.height = Math.max(NOTE_MIN_HEIGHT, this.snapCoordinate(movedNote.height));
      }
      this.renderNote(movedNote);
      if (typeof this.options.onNoteChange === "function") {
        this.options.onNoteChange(movedNote);
      }
    }
    const resizedPreview = this.resizingPreview;
    this.resizingPreview = null;
    if (resizedPreview) {
      resizedPreview.el.classList.remove("resizing");
      const size = this.resizePreviewCell(
        resizedPreview,
        Number(resizedPreview.el.dataset.width),
        Number(resizedPreview.el.dataset.height),
        true
      );
      if (typeof this.options.onPreviewResize === "function") {
        const node = resizedPreview.node;
        this.options.onPreviewResize(String(node.zfPipelineNodeId || node.id), resizedPreview.which, size);
      }
    }
    const resizedInput = this.resizingInput;
    this.resizingInput = null;
    if (resizedInput) {
      resizedInput.el.classList.remove("resizing");
      const size = this.resizeInputWidget(
        resizedInput,
        Number(resizedInput.el.dataset.width),
        Number(resizedInput.el.dataset.height),
        true
      );
      if (typeof this.options.onInputResize === "function") {
        const node = resizedInput.node;
        this.options.onInputResize(String(node.zfPipelineNodeId || node.id), size);
      }
    }
    this.panning = false;
    this.releasePointer();
  }

  onWheel(event) {
    event.preventDefault();
    const delta = -event.deltaY * 0.001;
    const nextScale = clamp(this.transform.k + delta, 0.1, 3);
    const rect = this.workspaceEl.getBoundingClientRect();
    const mouseX = event.clientX - rect.left;
    const mouseY = event.clientY - rect.top;
    const worldX = (mouseX - this.transform.x) / this.transform.k;
    const worldY = (mouseY - this.transform.y) / this.transform.k;
    this.transform.k = nextScale;
    this.transform.x = mouseX - worldX * nextScale;
    this.transform.y = mouseY - worldY * nextScale;
    this.updateTransform();
    this.updateWires();
  }

  onKeyDown(event) {
    if (this.readOnly) {
      return;
    }
    if (event.key === "Escape" && this.paletteNote) {
      this.toggleNotePalette(this.paletteNote, false);
      return;
    }
    if (event.target.tagName === "INPUT" || event.target.tagName === "TEXTAREA") {
      return;
    }
    if (event.key !== "Delete" && event.key !== "Backspace") {
      return;
    }

    if (this.selectedLink) {
      this.graph.links = this.graph.links.filter((link) => link.id !== this.selectedLink);
      const path = this.svgEl.querySelector(`#${CSS.escape(this.selectedLink)}`);
      if (path) {
        path.remove();
      }
      this.selectedLink = null;
      return;
    }

    if (this.selectedNode) {
      this.graph.remove(this.selectedNode);
      this.updateWires();
      this.selectedNode = null;
      return;
    }

    if (this.selectedNote) {
      this.removeNote(this.selectedNote);
    }
  }

  clearSelection() {
    if (this.selectedNode?.el) {
      this.selectedNode.el.classList.remove("selected");
    }
    if (this.selectedNote?.el) {
      this.selectedNote.el.classList.remove("selected");
    }
    this.selectedNote = null;
    if (this.selectedLink) {
      const old = this.svgEl.querySelector(`#${CSS.escape(this.selectedLink)}`);
      if (old) {
        old.classList.remove("selected");
      }
    }
    this.selectedNode = null;
    this.selectedLink = null;
  }
}

function defaultNodeFactory() {
  return {
    number: (x, y) => new NumberNode({ x, y }),
    add: (x, y) => new AddNode({ x, y }),
    display: (x, y) => new DisplayNode({ x, y }),
    custom: (x, y, config = {}) => new CustomNode({ x, y, ...config }),
  };
}

export function createSeedScenes() {
  return {
    logic: {
      label: "1. Logic & Math",
      toolbox: [
        { label: "Number Slider", action: (app) => app.spawn("number"), accent: "#34d399" },
        { label: "Add Math", action: (app) => app.spawn("add"), accent: "#60a5fa" },
        { label: "Display", action: (app) => app.spawn("display"), accent: "#c084fc" },
      ],
      setup(app) {
        const n1 = app.addNode(app.factory.number(100, 100));
        const n2 = app.addNode(app.factory.number(100, 250));
        const add = app.addNode(app.factory.add(400, 150));
        const out = app.addNode(app.factory.display(700, 170));

        app.graph.connect(n1.id, 0, add.id, 0);
        app.graph.connect(n2.id, 0, add.id, 1);
        app.graph.connect(add.id, 0, out.id, 0);
      },
    },
    branch: {
      label: "2. Branch & Merge",
      toolbox: [],
      setup(app) {
        app.ui.transform = { x: app.ui.workspaceEl.clientWidth / 2 - 450, y: app.ui.workspaceEl.clientHeight / 2 - 320, k: 1 };
        app.ui.updateTransform();

        const source = app.addNode(app.factory.custom(100, 300, {
          title: "[A] Data Source",
          color: "#b91c1c",
          inputs: [],
          outputs: ["Output Stream"],
        }));

        const branches = [];
        for (let i = 0; i < 5; i += 1) {
          branches.push(app.addNode(app.factory.custom(400, 110 + i * 120, {
            title: `[B${i + 1}] Worker`,
            color: "#047857",
            inputs: ["Input"],
            outputs: ["Result"],
          })));
        }

        const merge = app.addNode(app.factory.custom(760, 300, {
          title: "[C] Merged Output",
          color: "#4338ca",
          inputs: ["In 1", "In 2", "In 3", "In 4", "In 5"],
          outputs: [],
        }));

        branches.forEach((branch, i) => {
          app.graph.connect(source.id, 0, branch.id, 0, {
            animated: true,
            color: "#0ea5e9",
            dashArray: "15 15",
            speed: "1s",
          });
          app.graph.connect(branch.id, 0, merge.id, i, {
            animated: true,
            color: "#8b5cf6",
            dashArray: "8 8",
            speed: "1.5s",
            thickness: 2,
          });
        });
      },
    },
    custom: {
      label: "3. Edge Styles",
      toolbox: [],
      setup(app) {
        const source = app.addNode(app.factory.custom(100, 150, {
          title: "Start Points",
          color: "#111827",
          inputs: [],
          outputs: ["Port 1", "Port 2", "Port 3", "Port 4"],
        }));
        const target = app.addNode(app.factory.custom(620, 150, {
          title: "End Points",
          color: "#111827",
          inputs: ["Port 1", "Port 2", "Port 3", "Port 4"],
          outputs: [],
        }));

        app.graph.connect(source.id, 0, target.id, 0, {
          animated: true,
          color: "#f59e0b",
          dashArray: "30 10",
          speed: "0.5s",
          thickness: 6,
        });
        app.graph.connect(source.id, 1, target.id, 1, {
          animated: true,
          color: "#10b981",
          dashArray: "5 5",
          speed: "2s",
          thickness: 2,
        });
        app.graph.connect(source.id, 2, target.id, 2, {
          animated: false,
          color: "#ec4899",
          thickness: 5,
          opacity: 0.5,
        });
        app.graph.connect(source.id, 3, target.id, 3);
      },
    },
  };
}

function sanitizePins(pins, fallback) {
  if (!Array.isArray(pins)) {
    return fallback.slice();
  }
  const out = [];
  pins.forEach((pin) => {
    const label = String(pin || "").trim();
    if (label) {
      out.push(label);
    }
  });
  return out;
}

function cloneJsonLike(value) {
  if (value == null) {
    return value;
  }
  try {
    return JSON.parse(JSON.stringify(value));
  } catch (_err) {
    return value;
  }
}

function resolveNodeColor(kind, colorMap, fallbackColor) {
  if (colorMap && typeof colorMap === "object" && typeof colorMap[kind] === "string") {
    return colorMap[kind];
  }
  if (typeof DEFAULT_NODE_KIND_COLORS[kind] === "string") {
    return DEFAULT_NODE_KIND_COLORS[kind];
  }
  return fallbackColor;
}

function resolveNodeTitle(kind, config, catalogTitle) {
  const cfg = config && typeof config === "object" ? config : {};
  // 1. Instance title (user-set)
  if (cfg.title) {
    return String(cfg.title);
  }
  // 2. Special formatting for certain kinds
  if (kind === "n.trigger.webhook") {
    const method = String(cfg.method || "GET").toUpperCase();
    const path = String(cfg.path || "/").trim() || "/";
    return `${method} ${path}`;
  }
  // 3. Definition title (from catalog)
  if (catalogTitle) {
    return String(catalogTitle);
  }
  // 4. Definition slug (kind) — final fallback
  return kind;
}

function slugifyPinName(raw, fallback = "case") {
  const out = String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return out || fallback;
}

function normalizeMatchCase(item, index) {
  if (typeof item === "string") {
    const value = item.trim();
    return value ? { value, pin: slugifyPinName(value, `case-${index + 1}`), label: value } : null;
  }
  const source = item && typeof item === "object" ? item : {};
  const value = String(source.value || "").trim();
  if (!value) return null;
  return {
    value,
    pin: slugifyPinName(source.pin || value, `case-${index + 1}`),
    label: String(source.label || value).trim() || value,
  };
}

function normalizeMatchCases(rawCases) {
  if (typeof rawCases === "string") {
    return rawCases.split("\n").map(normalizeMatchCase).filter(Boolean);
  }
  if (!Array.isArray(rawCases)) return [];
  return rawCases.map(normalizeMatchCase).filter(Boolean);
}

function normalizeMatchDefault(rawDefault) {
  if (typeof rawDefault === "string") {
    const pin = slugifyPinName(rawDefault, "default");
    return { pin, label: pin === "default" ? "Default" : pin };
  }
  const source = rawDefault && typeof rawDefault === "object" ? rawDefault : {};
  return {
    pin: slugifyPinName(source.pin || "default", "default"),
    label: String(source.label || "Default").trim() || "Default",
  };
}

function deriveOutputPins(kind, config, rawPins, fallback) {
  if (String(kind || "") !== "n.logic.match") {
    return sanitizePins(rawPins, fallback);
  }
  const pins = [];
  normalizeMatchCases(config?.cases).forEach((item) => {
    if (!pins.includes(item.pin)) pins.push(item.pin);
  });
  const defaultRoute = normalizeMatchDefault(config?.default);
  if (!pins.includes(defaultRoute.pin)) pins.push(defaultRoute.pin);
  return pins.length > 0 ? pins : sanitizePins(rawPins, ["default"]);
}

function deriveOutputLabels(kind, config, outputPins = []) {
  const labels = {};
  if (String(kind || "") !== "n.logic.match") {
    outputPins.forEach((pin) => {
      labels[pin] = pin;
    });
    return labels;
  }
  normalizeMatchCases(config?.cases).forEach((item) => {
    labels[item.pin] = item.label || item.value || item.pin;
  });
  const defaultRoute = normalizeMatchDefault(config?.default);
  labels[defaultRoute.pin] = defaultRoute.label || defaultRoute.pin;
  outputPins.forEach((pin) => {
    if (!labels[pin]) labels[pin] = pin;
  });
  return labels;
}

function defaultConfigForKind(kind) {
  if (String(kind || "") === "n.logic.match") {
    return { cases: [], default: { pin: "default", label: "Default" } };
  }
  return {};
}

export function createPipelineScene(pipeline, options = {}) {
  const nodeSpacingX = Number(options.nodeSpacingX || 320);
  const nodeSpacingY = Number(options.nodeSpacingY || 170);
  const baseX = Number(options.baseX || 120);
  const baseY = Number(options.baseY || 120);
  const fallbackNodeColor = options.fallbackNodeColor || "#334155";
  const colorMap = options.kindColors || null;
  const iconMap = options.kindIcons || null;
  const titleMap = options.kindTitles || null;
  const defaultEdgeOptions = options.defaultEdgeOptions || {};

  const graphNodes = Array.isArray(pipeline?.nodes) ? pipeline.nodes : [];
  const graphEdges = Array.isArray(pipeline?.edges) ? pipeline.edges : [];

  return {
    label: options.label || String(pipeline?.id || "Pipeline"),
    toolbox: options.toolbox || [],
    setup(app) {
      const nodeMap = new Map();

      // Pins an edge leaves from that the node never declared — `:error` on
      // any node in graph mode, which the engine routes without a declared
      // pin. Each needs a slot of its own: without one the link landed on
      // slot 0 and a Save Draft wrote `[a]:error -> [r]` back as `[a] -> [r]`,
      // silently unwiring a retry loop.
      const undeclaredOutputs = new Map();
      graphEdges.forEach((edge) => {
        const fromId = String(edge?.from_node || "");
        const pin = String(edge?.from_pin || "");
        if (!fromId || !pin) return;
        if (!undeclaredOutputs.has(fromId)) undeclaredOutputs.set(fromId, []);
        const pins = undeclaredOutputs.get(fromId);
        if (!pins.includes(pin)) pins.push(pin);
      });

      graphNodes.forEach((node, index) => {
        const row = Math.floor(index / 3);
        const col = index % 3;
        const cfg = node?.config || {};
        const uiCfg = cfg.ui && typeof cfg.ui === "object" ? cfg.ui : {};
        const x = Number.isFinite(uiCfg.x) ? uiCfg.x : baseX + col * nodeSpacingX;
        const y = Number.isFinite(uiCfg.y) ? uiCfg.y : baseY + row * nodeSpacingY;

        const inputs = sanitizePins(node?.input_pins, ["in"]);
        const kind = String(node?.kind || "node");
        const outputs = [...deriveOutputPins(kind, cfg, node?.output_pins, ["out"])];
        for (const pin of undeclaredOutputs.get(String(node?.id || "")) || []) {
          if (!outputs.includes(pin)) outputs.push(pin);
        }
        const catalogTitle = titleMap && titleMap[kind] ? titleMap[kind] : "";
        const title = resolveNodeTitle(kind, cfg, catalogTitle);
        const color = resolveNodeColor(kind, colorMap, fallbackNodeColor);
        const icon = iconMap && iconMap[kind] ? iconMap[kind] : "";

        const domNode = app.factory.custom(x, y, {
          title,
          color,
          icon,
          inputs,
          outputs,
        });
        domNode.zfPipelineNodeId = String(node?.id || "");
        domNode.zfKind = kind;
        domNode.zfConfig = cloneJsonLike(cfg || {});
        domNode.zfOutputLabels = deriveOutputLabels(kind, cfg, outputs);
        const mounted = app.addNode(domNode);
        nodeMap.set(String(node?.id || ""), mounted);
      });

      graphEdges.forEach((edge) => {
        const fromId = String(edge?.from_node || "");
        const toId = String(edge?.to_node || "");
        const from = nodeMap.get(fromId);
        const to = nodeMap.get(toId);
        if (!from || !to) {
          return;
        }

        const fromPin = String(edge?.from_pin || "");
        const toPin = String(edge?.to_pin || "");
        const fromIndex = from.outputs.findIndex((pin) => pin.name === fromPin);
        const toIndex = to.inputs.findIndex((pin) => pin.name === toPin);
        const resolvedFrom = fromIndex >= 0 ? fromIndex : 0;
        const resolvedTo = toIndex >= 0 ? toIndex : 0;

        const edgeOptions = typeof options.edgeOptions === "function"
          ? options.edgeOptions(edge, from, to) || defaultEdgeOptions
          : defaultEdgeOptions;
        app.graph.connect(from.id, resolvedFrom, to.id, resolvedTo, edgeOptions);
      });

      const graphNotes = Array.isArray(pipeline?.notes) ? pipeline.notes : [];
      graphNotes.forEach((note) => {
        if (!note || typeof note !== "object") return;
        app.addNote({
          zfId: note.id,
          text: note.text,
          x: note.x,
          y: note.y,
          width: note.width,
          height: note.height,
          color: note.color,
        });
      });
    },
  };
}

export function createGraphUI(root, options = {}) {
  if (!root) {
    throw new Error("createGraphUI requires root element");
  }
  ensureStyles();

  const theme = { ...DEFAULT_THEME, ...(options.theme || {}) };
  setThemeVars(root, theme);

  const graph = new GraphStore();
  const ui = new GraphCanvasUI(root, graph, options);
  const factory = { ...defaultNodeFactory(), ...(options.nodeFactory || {}) };

  const scenes = new Map();
  const sceneOrder = [];

  let rafId = null;
  const tick = () => {
    graph.execute();
    rafId = window.requestAnimationFrame(tick);
  };
  rafId = window.requestAnimationFrame(tick);

  const app = {
    root,
    graph,
    ui,
    factory,
    currentScene: null,

    addNode(node) {
      graph.add(node);
      ui.addNodeToDOM(node);
      return node;
    },

    /** A canvas note: `{ zfId, text, x, y, width, height, color }`. */
    addNote(note) {
      const next = {
        zfId: String(note?.zfId || note?.id || ""),
        text: String(note?.text || ""),
        x: Number(note?.x) || 0,
        y: Number(note?.y) || 0,
        width: Number(note?.width) || 0,
        height: Number(note?.height) || 0,
        color: String(note?.color || ""),
      };
      graph.addNote(next);
      ui.addNoteToDOM(next);
      return next;
    },

    removeNote(note) {
      ui.removeNote(note);
    },

    updateNodePins(node, nextPins = {}) {
      if (!node || !graph.nodes.includes(node)) {
        return node;
      }
      const oldInputs = (node.inputs || []).map((pin) => pin.name);
      const oldOutputs = (node.outputs || []).map((pin) => pin.name);
      const inputNames = Array.isArray(nextPins.inputs)
        ? sanitizePins(nextPins.inputs, [])
        : oldInputs;
      const outputNames = Array.isArray(nextPins.outputs)
        ? sanitizePins(nextPins.outputs, [])
        : oldOutputs;

      node.inputs = inputNames.map((name) => {
        const old = (node.inputs || []).find((pin) => pin.name === name);
        return { name, value: old?.value || 0 };
      });
      node.outputs = outputNames.map((name) => {
        const old = (node.outputs || []).find((pin) => pin.name === name);
        return { name, value: old?.value || 0 };
      });

      graph.links = graph.links
        .map((link) => {
          if (link.fromNode === node.id) {
            const oldName = oldOutputs[link.fromSlot];
            const nextIndex = outputNames.indexOf(oldName);
            if (nextIndex < 0) return null;
            link.fromSlot = nextIndex;
          }
          if (link.toNode === node.id) {
            const oldName = oldInputs[link.toSlot];
            const nextIndex = inputNames.indexOf(oldName);
            if (nextIndex < 0) return null;
            link.toSlot = nextIndex;
          }
          return link;
        })
        .filter(Boolean);

      const wasSelected = ui.selectedNode?.id === node.id;
      if (node.el) {
        node.el.remove();
        node.el = null;
      }
      node.buildDOM(ui.nodesEl);
      if (wasSelected && node.el) {
        ui.selectedNode = node;
        node.el.classList.add("selected");
      }
      ui.updateWires();
      return node;
    },

    spawn(type) {
      const nodeFactory = factory[type];
      if (!nodeFactory) {
        return null;
      }
      const cx = (-ui.transform.x + ui.workspaceEl.clientWidth / 2) / ui.transform.k - 90;
      const cy = (-ui.transform.y + ui.workspaceEl.clientHeight / 2) / ui.transform.k - 50;
      const node = nodeFactory(cx, cy);
      if (!node) {
        return null;
      }
      return app.addNode(node);
    },

    registerScene(id, scene) {
      if (!scenes.has(id)) {
        sceneOrder.push(id);
      }
      scenes.set(id, scene);
      renderSceneButtons();
    },

    loadScene(id) {
      const scene = scenes.get(id);
      if (!scene) {
        return;
      }

      app.currentScene = id;
      graph.clear();
      ui.clearSVG();
      ui.clearSelection();
      ui.resetCamera();

      renderToolbox(scene.toolbox || []);
      renderSceneButtons();

      if (typeof scene.setup === "function") {
        scene.setup(app);
      }

      window.setTimeout(() => {
        ui.updateWires();
      }, 32);
    },

    clear() {
      graph.clear();
      ui.clearSVG();
    },

    autoTidy(options = {}) {
      return autoTidyGraphLayout(app, options);
    },

    snapshot() {
      return {
        nodes: graph.nodes.map((node) => ({
          graph_node_id: node.id,
          pipeline_node_id: node.zfPipelineNodeId || null,
          kind: node.zfKind || null,
          title: node.title,
          x: node.x,
          y: node.y,
          input_pins: node.inputs.map((pin) => pin.name),
          output_pins: node.outputs.map((pin) => pin.name),
          config: cloneJsonLike(node.zfConfig || {}),
        })),
        edges: graph.links.map((link) => ({
          id: link.id,
          from_graph_node_id: link.fromNode,
          from_slot: link.fromSlot,
          to_graph_node_id: link.toNode,
          to_slot: link.toSlot,
          options: cloneJsonLike(link.options || {}),
        })),
      };
    },

    destroy() {
      if (rafId) {
        window.cancelAnimationFrame(rafId);
      }
      ui.destroy();
      root.innerHTML = "";
    },
  };
  ui.options.onAutoTidy = () => app.autoTidy();
  ui.updateCanvasControls?.();

  function renderSceneButtons() {
    if (options.showHeader === false) {
      return;
    }
    ui.sceneButtonsEl.innerHTML = "";
    sceneOrder.forEach((id) => {
      const scene = scenes.get(id);
      const btn = document.createElement("button");
      btn.className = "zgu-scene-btn" + (app.currentScene === id ? " active" : "");
      btn.textContent = scene.label || id;
      btn.addEventListener("click", () => app.loadScene(id));
      ui.sceneButtonsEl.appendChild(btn);
    });
  }

  function renderToolbox(tools) {
    if (options.showToolbox === false) {
      return;
    }
    ui.toolboxButtonsEl.innerHTML = "";
    if (!Array.isArray(tools) || tools.length === 0) {
      ui.toolboxEl.style.opacity = "0";
      return;
    }
    ui.toolboxEl.style.opacity = "1";
    tools.forEach((tool) => {
      const btn = document.createElement("button");
      btn.className = "zgu-toolbox-btn";
      const accent = tool.accent ? `<span style=\"color:${tool.accent};font-size:10px;\">●</span>` : "";
      btn.innerHTML = `<span>${tool.label || "Action"}</span>${accent}`;
      btn.addEventListener("click", () => {
        if (typeof tool.action === "function") {
          tool.action(app);
        }
      });
      ui.toolboxButtonsEl.appendChild(btn);
    });
  }

  const seedScenes = options.scenes || createSeedScenes();
  Object.entries(seedScenes).forEach(([id, scene]) => {
    app.registerScene(id, scene);
  });

  const firstScene = options.initialScene || sceneOrder[0] || null;
  if (firstScene) {
    app.loadScene(firstScene);
  }

  return app;
}

// ── PipelineGraph — Preact wrapper ─────────────────────────────────────────
export const PipelineGraph = (() => {
  const _h = globalThis.h;
  const _useRef = globalThis.useRef;
  const _useEffect = globalThis.useEffect;
  const _forwardRef = globalThis.forwardRef;
  const _useImperativeHandle = globalThis.useImperativeHandle;

  if (!_h || !_useRef || !_forwardRef) {
    // Hooks not yet available (should not happen in browser context).
    // Return a sentinel stub so the module can still be imported.
    const stub = function PipelineGraph(props) {
      return _h
        ? _h("div", {
            "data-zeb-lib": "graphui",
            "data-zeb-wrapper": "PipelineGraph",
            id: props && props.id,
            className: (props && props.className) || "w-full h-full",
          })
        : null;
    };
    globalThis.PipelineGraph = stub;
    return stub;
  }

  // ── internal helpers ──────────────────────────────────────────────────────

  function _pgSanitizeSlug(raw) {
    return (
      String(raw || "")
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9._-]+/g, "-")
        .replace(/-+/g, "-")
        .replace(/^-|-$/g, "") || "node"
    );
  }

  function _pgGenerateSlug(kind, nodes) {
    const base = _pgSanitizeSlug(
      String(kind || "node")
        .split(".")
        .filter(Boolean)
        .slice(1)
        .join("-") || "node"
    );
    const count = (nodes || []).filter(
      (n) => String(n.zfKind || "") === String(kind)
    ).length;
    return count <= 0 ? base : `${base}-${count}`;
  }

  function _pgAttachEditButtons(app, onNodeEdit) {
    const root = app.root;
    if (!root) return;
    const readOnly = !!(app.ui && app.ui.readOnly);
    const nodeMap = new Map(app.graph.nodes.map((n) => [String(n.id), n]));
    const toNodeEditPayload = (nodeData) => ({
      graphNodeId: nodeData.id,
      zfKind: nodeData.zfKind || "",
      zfPipelineNodeId: nodeData.zfPipelineNodeId || "",
      zfConfig: nodeData.zfConfig || {},
      title: nodeData.title,
      x: nodeData.x,
      y: nodeData.y,
      inputs: nodeData.inputs || [],
      outputs: nodeData.outputs || [],
      _raw: nodeData,
    });
    const emitNodeEdit = (payload, fallback) => {
      let prevented = false;
      if (typeof window !== "undefined" && typeof window.CustomEvent === "function") {
        const event = new CustomEvent("zebflow:pipeline-node-edit", {
          detail: payload,
          bubbles: true,
          cancelable: true,
        });
        window.dispatchEvent(event);
        prevented = event.defaultPrevented;
      }
      if (!prevented && fallback) {
        fallback(payload);
      }
    };
    root.querySelectorAll(".zgu-node").forEach((el) => {
      const nodeData = nodeMap.get(el.getAttribute("data-id") || "");
      if (!nodeData) return;
      const openEdit = () => {
        const cb = app._pgOnNodeEdit || onNodeEdit;
        emitNodeEdit(toNodeEditPayload(nodeData), cb);
      };

      const existingButton = el.querySelector(".zf-node-edit");
      if (readOnly || !onNodeEdit) {
        if (existingButton)
          existingButton.remove();
        el.ondblclick = null;
        el.__pgOpenNodeEdit = null;
      } else if (!existingButton) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "zf-node-edit";
        btn.setAttribute("data-zgu-nodrag", "true");
        btn.textContent = "E";
        btn.title = "Edit Node";
        el.appendChild(btn);
      }
      const editButton = el.querySelector(".zf-node-edit");
      if (!readOnly && onNodeEdit && editButton) {
        el.__pgOpenNodeEdit = openEdit;
        if (!el.__pgNodePointerEditHandler) {
          el.__pgLastNodePointerDown = null;
          el.__pgNodePointerEditHandler = (e) => {
            if (e.button !== 0 || e.target?.closest?.("[data-zgu-nodrag='true']")) return;
            const previous = el.__pgLastNodePointerDown;
            const now = performance.now();
            el.__pgLastNodePointerDown = { t: now, x: e.clientX, y: e.clientY };
            if (
              previous &&
              now - previous.t <= 420 &&
              Math.abs(e.clientX - previous.x) <= 8 &&
              Math.abs(e.clientY - previous.y) <= 8
            ) {
              e.preventDefault();
              e.stopPropagation();
              el.__pgLastNodePointerDown = null;
              el.__pgOpenNodeEdit?.();
            }
          };
          el.addEventListener("pointerdown", el.__pgNodePointerEditHandler);
        }
        editButton.onclick = (e) => {
          e.preventDefault();
          e.stopPropagation();
          openEdit();
        };
        el.ondblclick = (e) => {
          if (e.target?.closest?.("[data-zgu-nodrag='true']")) return;
          e.preventDefault();
          e.stopPropagation();
          openEdit();
        };
      }

      let badge = el.querySelector(".zf-node-slug");
      if (!badge) {
        badge = document.createElement("div");
        badge.className = "zf-node-slug";
        el.appendChild(badge);
      }
      const nextText = String(nodeData.zfPipelineNodeId || "");
      if (badge.textContent !== nextText)
        badge.textContent = nextText;
      badge.classList.toggle("long", nextText.length > 2);

      const title = el.querySelector(".zgu-node-label");
      if (title && title.textContent !== String(nodeData.title || "")) {
        title.textContent = String(nodeData.title || "");
      }
    });
  }

  function _pgAttachContinuationButtons(app, onOutputAdd) {
    if (!app?.ui) return;
    app.ui.options.onOutputAdd = onOutputAdd
      ? (payload) => {
          let prevented = false;
          if (typeof window !== "undefined" && typeof window.CustomEvent === "function") {
            const event = new CustomEvent("zebflow:pipeline-output-add", {
              detail: payload,
              bubbles: true,
              cancelable: true,
            });
            window.dispatchEvent(event);
            prevented = event.defaultPrevented;
          }
          if (!prevented) {
            const cb = app._pgOnOutputAdd || onOutputAdd;
            if (cb) cb(payload);
          }
        }
      : null;
    app.ui.updateWires?.();
  }

  function _pgAttachChrome(app, onNodeEdit, onOutputAdd) {
    _pgAttachEditButtons(app, onNodeEdit);
    _pgAttachContinuationButtons(app, onOutputAdd);
  }

  /**
   * Push `previewData` onto the canvas: one panel per node that has an entry,
   * nothing for the rest. Keyed by pipeline node id — the slug the host knows
   * a node by — not by the canvas's own numeric id.
   */
  function _pgSyncPreviews(app, previewData, onPreviewOpen, onPreviewResize) {
    if (!app || !app.ui || typeof app.ui.setNodePreview !== "function") return;
    app.ui.options.onPreviewOpen = onPreviewOpen || null;
    app.ui.options.onPreviewResize = onPreviewResize || null;
    const data = previewData && typeof previewData === "object" ? previewData : {};
    for (const node of app.graph.nodes || []) {
      const key = String(node.zfPipelineNodeId || node.id);
      app.ui.setNodePreview(node, data[key] || null);
    }
  }

  /**
   * Push `inputWidgets` onto the canvas: one Run-form field per `n.input.*`
   * node that has an entry, keyed by pipeline node id like the previews.
   * Drawn before the previews so the widget takes the top of the slot.
   */
  function _pgSyncInputs(app, inputWidgets, onInputChange, onInputResize) {
    if (!app || !app.ui || typeof app.ui.setNodeInput !== "function") return;
    app.ui.options.onInputChange = onInputChange || null;
    app.ui.options.onInputResize = onInputResize || null;
    const data = inputWidgets && typeof inputWidgets === "object" ? inputWidgets : {};
    for (const node of app.graph.nodes || []) {
      const key = String(node.zfPipelineNodeId || node.id);
      app.ui.setNodeInput(node, data[key] || null);
    }
  }

  /**
   * Push `nodeStatus` onto the canvas: one run badge per node that has an
   * entry, none for the rest, keyed by pipeline node id like the previews.
   */
  function _pgSyncStatus(app, nodeStatus) {
    if (!app || !app.ui || typeof app.ui.setNodeStatus !== "function") return;
    const data = nodeStatus && typeof nodeStatus === "object" ? nodeStatus : {};
    for (const node of app.graph.nodes || []) {
      const key = String(node.zfPipelineNodeId || node.id);
      app.ui.setNodeStatus(node, data[key] || null);
    }
  }

  function _pgEnsureObserver(stateRef, app, onNodeEdit, onOutputAdd) {
    stateRef.current.observer?.disconnect();
    const obs = new MutationObserver(() =>
      _pgAttachChrome(app, onNodeEdit, onOutputAdd)
    );
    obs.observe(app.root, { childList: true, subtree: true });
    stateRef.current.observer = obs;
  }

  function _pgLoadScene(app, pipeline, kindColors, kindIcons, kindTitles) {
    if (!pipeline) return;
    const scene = createPipelineScene(pipeline, {
      kindColors: { ...DEFAULT_NODE_KIND_COLORS, ...(kindColors || {}) },
      kindIcons: kindIcons || {},
      kindTitles: kindTitles || {},
    });
    app.registerScene("__pg", scene);
    app.loadScene("__pg");
  }

  function _pgCollect(app) {
    const used = new Set();
    const nodes = app.graph.nodes.map((node) => {
      const kind = node.zfKind || "n.script";
      let id = _pgSanitizeSlug(
        node.zfPipelineNodeId || kind.split(".").pop() || "node"
      );
      let candidate = id,
        seq = 2;
      while (used.has(candidate)) {
        candidate = `${id}_${seq}`;
        seq++;
      }
      used.add(candidate);
      node.zfPipelineNodeId = candidate;
      return {
        id: candidate,
        kind,
        input_pins: (node.inputs || []).map((p) => p.name),
        output_pins: (node.outputs || []).map((p) => p.name),
        config: {
          ...(node.zfConfig || {}),
          // `config.ui` is the editor-only key the engine ignores: the box's
          // position, and beside it `widget` (an input widget's dragged size,
          // written by the host on `onInputResize`). Kept whole; x/y are the
          // canvas's own.
          ui: {
            ...((node.zfConfig && node.zfConfig.ui) || {}),
            x: Math.round(node.x),
            y: Math.round(node.y),
          },
        },
      };
    });
    const byId = new Map(app.graph.nodes.map((n) => [n.id, n]));
    const edges = app.graph.links
      .map((link) => {
        const from = byId.get(link.fromNode),
          to = byId.get(link.toNode);
        if (!from || !to) return null;
        return {
          from_node: from.zfPipelineNodeId,
          from_pin: (from.outputs[link.fromSlot] || {}).name || "out",
          to_node: to.zfPipelineNodeId,
          to_pin: (to.inputs[link.toSlot] || {}).name || "in",
        };
      })
      .filter(Boolean);
    const entry = nodes
      .filter((n) => String(n.kind).startsWith("n.trigger."))
      .map((n) => n.id);
    const usedNoteIds = new Set();
    const notes = (app.graph.notes || []).map((note, index) => {
      let id = _pgSanitizeSlug(note.zfId || "") || `note${index + 1}`;
      let candidate = id,
        seq = 2;
      while (usedNoteIds.has(candidate)) {
        candidate = `${id}_${seq}`;
        seq++;
      }
      usedNoteIds.add(candidate);
      note.zfId = candidate;
      const out = {
        id: candidate,
        text: String(note.text || ""),
        x: Math.round(note.x),
        y: Math.round(note.y),
        width: Math.round(note.width),
        height: Math.round(note.height),
      };
      if (note.color) out.color = String(note.color);
      return out;
    });
    const pipeline = {
      id: app._pgId || "pipeline",
      entry_nodes:
        entry.length ? entry : nodes[0] ? [nodes[0].id] : [],
      nodes,
      edges,
    };
    if (notes.length) pipeline.notes = notes;
    return pipeline;
  }

  // ── Component ─────────────────────────────────────────────────────────────

  const PipelineGraphComponent = _forwardRef(function PipelineGraph(props, ref) {
    const hostRef = _useRef(null);
    const appRef = _useRef(null);
    const stateRef = _useRef({ observer: null });

    _useImperativeHandle(ref, function () {
      return {
        addNode: function (kind, entry) {
          const app = appRef.current;
          if (!app) return;
          const { ui } = app;
          const x =
            (-ui.transform.x + ui.workspaceEl.clientWidth / 2) /
              ui.transform.k -
            90;
          const y =
            (-ui.transform.y + ui.workspaceEl.clientHeight / 2) /
              ui.transform.k -
            50;
          const config = defaultConfigForKind(kind);
          const node = app.factory.custom(x, y, {
            title: entry.title || kind,
            color:
              entry.color ||
              DEFAULT_NODE_KIND_COLORS[kind] ||
              "#334155",
            icon:
              entry.icon ||
              (props.kindIcons && props.kindIcons[kind]) ||
              "",
            inputs: entry.input_pins || ["in"],
            outputs: deriveOutputPins(kind, config, entry.output_pins, ["out"]),
          });
          node.zfKind = kind;
          node.zfConfig = config;
          node.zfOutputLabels = deriveOutputLabels(kind, config, node.outputs.map((pin) => pin.name));
          node.zfPipelineNodeId = _pgGenerateSlug(kind, app.graph.nodes);
          app.addNode(node);
          _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
        },
        collectPipeline: function () {
          return _pgCollect(appRef.current);
        },
        /** Place a new note at the canvas centre and open it for editing. */
        addNote: function (text, color) {
          const app = appRef.current;
          if (!app) return null;
          const { ui } = app;
          const x = (-ui.transform.x + ui.workspaceEl.clientWidth / 2) / ui.transform.k - 140;
          const y = (-ui.transform.y + ui.workspaceEl.clientHeight / 2) / ui.transform.k - 60;
          const taken = new Set((app.graph.notes || []).map((n) => n.zfId));
          let seq = (app.graph.notes || []).length + 1;
          while (taken.has(`note${seq}`)) seq++;
          const note = app.addNote({ zfId: `note${seq}`, text: text || "", x, y, color: color || "amber" });
          ui.beginNoteEdit(note);
          return note;
        },
        autoTidy: function (options) {
          const app = appRef.current;
          if (!app || typeof app.autoTidy !== "function") return [];
          const placements = app.autoTidy(options || {});
          _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
          return placements;
        },
        getApp: function () {
          return appRef.current;
        },
      };
    });

    _useEffect(function () {
      if (!hostRef.current) return;
      const app = createGraphUI(hostRef.current, {
        showHeader: false,
        showToolbox: false,
        readOnly: props.readOnly || false,
        snapToGrid: props.snapToGrid !== false,
        gridSize: props.gridSize || 30,
        onOutputAdd: props.onOutputAdd || null,
        selectionMode: props.selectionMode || "normal",
        onSelectionModeChange: props.onSelectionModeChange || null,
        onSelectAll: props.onSelectAll || null,
      });
      app._pgId = props.pipeline?.id || "pipeline";
      app._pgOnNodeEdit = props.onNodeEdit || null;
      app._pgOnOutputAdd = props.onOutputAdd || null;
      appRef.current = app;
      hostRef.current.__zebGraphApp = app;
      _pgLoadScene(app, props.pipeline, props.kindColors, props.kindIcons, props.kindTitles);
      _pgSyncInputs(app, props.inputWidgets, props.onInputChange, props.onInputResize);
      _pgSyncPreviews(app, props.previewData, props.onPreviewOpen, props.onPreviewResize);
      _pgSyncStatus(app, props.nodeStatus);
      setTimeout(function () {
        _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
        _pgSyncInputs(app, props.inputWidgets, props.onInputChange, props.onInputResize);
        _pgSyncPreviews(app, props.previewData, props.onPreviewOpen, props.onPreviewResize);
        _pgSyncStatus(app, props.nodeStatus);
      }, 0);
      setTimeout(function () {
        _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
      }, 120);
      _pgEnsureObserver(stateRef, app, props.onNodeEdit, props.onOutputAdd);
      if (props.onReady) props.onReady(app);
      return function () {
        stateRef.current.observer?.disconnect();
        if (hostRef.current) {
          delete hostRef.current.__zebGraphApp;
        }
        app.destroy();
        appRef.current = null;
      };
    }, []);

    _useEffect(function () {
      const app = appRef.current;
      if (!app) return;
      app._pgId = props.pipeline?.id || app._pgId || "pipeline";
      app._pgOnNodeEdit = props.onNodeEdit || null;
      app._pgOnOutputAdd = props.onOutputAdd || null;
      app.ui.options.onSelectionModeChange = props.onSelectionModeChange || null;
      app.ui.options.onSelectAll = props.onSelectAll || null;
      app.ui.readOnly = props.readOnly || false;
      app.ui.setSelectionMode?.(props.selectionMode || "normal", { emit: false });
      app.ui.updateCanvasControls?.();
      _pgLoadScene(app, props.pipeline, props.kindColors, props.kindIcons, props.kindTitles);
      _pgSyncInputs(app, props.inputWidgets, props.onInputChange, props.onInputResize);
      _pgSyncPreviews(app, props.previewData, props.onPreviewOpen, props.onPreviewResize);
      _pgSyncStatus(app, props.nodeStatus);
      setTimeout(function () {
        _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
        _pgSyncInputs(app, props.inputWidgets, props.onInputChange, props.onInputResize);
        _pgSyncPreviews(app, props.previewData, props.onPreviewOpen, props.onPreviewResize);
        _pgSyncStatus(app, props.nodeStatus);
      }, 0);
      _pgEnsureObserver(stateRef, app, props.onNodeEdit, props.onOutputAdd);
    }, [props.pipeline, props.readOnly, props.kindIcons, props.kindTitles]);

    _useEffect(function () {
      const app = appRef.current;
      if (!app) return;
      app._pgOnNodeEdit = props.onNodeEdit || null;
      app._pgOnOutputAdd = props.onOutputAdd || null;
      app.ui.options.onSelectionModeChange = props.onSelectionModeChange || null;
      app.ui.options.onSelectAll = props.onSelectAll || null;
      app.ui.setSelectionMode?.(props.selectionMode || "normal", { emit: false });
      app.ui.updateCanvasControls?.();
      _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
      _pgSyncInputs(app, props.inputWidgets, props.onInputChange, props.onInputResize);
      _pgSyncPreviews(app, props.previewData, props.onPreviewOpen, props.onPreviewResize);
      _pgSyncStatus(app, props.nodeStatus);
    });

    return _h("div", {
      ref: hostRef,
      id: props.id,
      className: props.className || "w-full h-full",
    });
  });

  Object.defineProperty(PipelineGraphComponent, "name", {
    value: "PipelineGraph",
  });
  globalThis.PipelineGraph = PipelineGraphComponent;
  return PipelineGraphComponent;
})();

export const graphui = {
  createGraphUI,
  createSeedScenes,
  createPipelineScene,
  GraphStore,
  GraphNode,
  CustomNode,
  NumberNode,
  AddNode,
  DisplayNode,
  GraphCanvasUI,
  PipelineGraph,
};

export default graphui;
