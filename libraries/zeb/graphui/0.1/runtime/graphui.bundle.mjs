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
.zgu-edge-style-control {
  position: absolute;
  right: 14px;
  bottom: 14px;
  z-index: 80;
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
    this.lastId = 1;
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
  }

  clear() {
    this.nodes.forEach((node) => node.el && node.el.remove());
    this.nodes = [];
    this.links = [];
    this.lastId = 1;
  }

  connect(fromNodeId, fromSlot, toNodeId, toSlot, options = {}) {
    this.links = this.links.filter((l) => !(l.toNode === toNodeId && l.toSlot === toSlot));
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

    this.transform = { x: 0, y: 0, k: 1 };

    this.draggingNode = null;
    this.panning = false;
    this.connecting = false;
    this.connectOrigin = null;
    this.pendingDangling = null;
    this.selectedNode = null;
    this.selectedLink = null;
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

    this.transformEl.appendChild(this.svgEl);
    this.transformEl.appendChild(this.nodesEl);
    this.workspaceEl.appendChild(this.transformEl);
    this.mountEdgeStyleControl();

    this.root.appendChild(this.headerEl);
    this.root.appendChild(this.toolboxEl);
    this.root.appendChild(this.workspaceEl);
  }

  mountEdgeStyleControl() {
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
    this.edgeStyleControlEl.addEventListener("pointerdown", (event) => {
      event.stopPropagation();
    });
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
    this.workspaceEl.appendChild(this.edgeStyleControlEl);
    this.updateEdgeStyleButtons();
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
  }

  destroy() {
    this.workspaceEl.removeEventListener("pointerdown", this.pointerDownHandler);
    window.removeEventListener("pointermove", this.pointerMoveHandler);
    window.removeEventListener("pointerup", this.pointerUpHandler);
    this.workspaceEl.removeEventListener("wheel", this.wheelHandler);
    window.removeEventListener("keydown", this.keyHandler);
    this.workspaceEl.removeEventListener("contextmenu", this.contextHandler);
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
    if (node.el) {
      node.el.style.transform = `translate(${node.x}px, ${node.y}px)`;
    }
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
      event.preventDefault();
      return;
    }

    const target = event.target;

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

    if (this.draggingNode) {
      const dx = (event.clientX - this.startPos.x) / this.transform.k;
      const dy = (event.clientY - this.startPos.y) / this.transform.k;
      this.draggingNode.x += dx;
      this.draggingNode.y += dy;
      this.draggingNode.el.style.transform = `translate(${this.draggingNode.x}px, ${this.draggingNode.y}px)`;
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
    }
  }

  clearSelection() {
    if (this.selectedNode?.el) {
      this.selectedNode.el.classList.remove("selected");
    }
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

      graphNodes.forEach((node, index) => {
        const row = Math.floor(index / 3);
        const col = index % 3;
        const cfg = node?.config || {};
        const uiCfg = cfg.ui && typeof cfg.ui === "object" ? cfg.ui : {};
        const x = Number.isFinite(uiCfg.x) ? uiCfg.x : baseX + col * nodeSpacingX;
        const y = Number.isFinite(uiCfg.y) ? uiCfg.y : baseY + row * nodeSpacingY;

        const inputs = sanitizePins(node?.input_pins, ["in"]);
        const kind = String(node?.kind || "node");
        const outputs = deriveOutputPins(kind, cfg, node?.output_pins, ["out"]);
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
          ui: { x: Math.round(node.x), y: Math.round(node.y) },
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
    return {
      kind: "zebflow.pipeline",
      version: "0.1",
      id: app._pgId || "pipeline",
      entry_nodes:
        entry.length ? entry : nodes[0] ? [nodes[0].id] : [],
      nodes,
      edges,
    };
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
      });
      app._pgId = props.id || "pipeline";
      app._pgOnNodeEdit = props.onNodeEdit || null;
      app._pgOnOutputAdd = props.onOutputAdd || null;
      appRef.current = app;
      hostRef.current.__zebGraphApp = app;
      _pgLoadScene(app, props.pipeline, props.kindColors, props.kindIcons, props.kindTitles);
      setTimeout(function () {
        _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
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
      app._pgOnNodeEdit = props.onNodeEdit || null;
      app._pgOnOutputAdd = props.onOutputAdd || null;
      app.ui.readOnly = props.readOnly || false;
      _pgLoadScene(app, props.pipeline, props.kindColors, props.kindIcons, props.kindTitles);
      setTimeout(function () {
        _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
      }, 0);
      _pgEnsureObserver(stateRef, app, props.onNodeEdit, props.onOutputAdd);
    }, [props.pipeline, props.readOnly, props.kindIcons, props.kindTitles]);

    _useEffect(function () {
      const app = appRef.current;
      if (!app) return;
      app._pgOnNodeEdit = props.onNodeEdit || null;
      app._pgOnOutputAdd = props.onOutputAdd || null;
      _pgAttachChrome(app, props.onNodeEdit, props.onOutputAdd);
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
