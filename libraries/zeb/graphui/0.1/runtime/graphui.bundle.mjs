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
  "n.sjtable.query": "#0f766e",
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
}
.zgu-workspace:active { cursor: grabbing; }
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
  min-width: 180px;
  max-width: 300px;
  border-radius: 8px;
  border: 1px solid #444;
  background: #222;
  box-shadow: 0 10px 15px -3px rgba(0,0,0,.5);
  user-select: none;
  z-index: 10;
}
.zgu-node.selected {
  border-color: #fff;
  box-shadow: 0 0 0 2px rgba(255,255,255,.18);
  z-index: 20;
}
.zgu-node-header {
  padding: 8px 12px;
  border-radius: 8px 8px 0 0;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}
.zgu-node-body { padding: 12px; position: relative; }
.zgu-port-list { display: flex; flex-direction: column; gap: 10px; }
.zgu-port-row {
  display: flex;
  align-items: center;
  position: relative;
  font-size: 11px;
  color: #a3a3a3;
}
.zgu-port-row.out { justify-content: flex-end; }
.zgu-port {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 2px solid #222;
  background: #555;
  position: absolute;
  cursor: crosshair;
}
.zgu-port:hover { transform: scale(1.5); background: var(--zgu-wire-active); }
.zgu-port.in { left: -18px; }
.zgu-port.out { right: -18px; }
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
  constructor({ title, x = 0, y = 0, color = "#334155" }) {
    this.id = 0;
    this.title = title;
    this.x = x;
    this.y = y;
    this.color = color;
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

    const header = document.createElement("div");
    header.className = "zgu-node-header";
    header.style.backgroundColor = this.color;
    header.textContent = this.title;
    node.appendChild(header);

    const body = document.createElement("div");
    body.className = "zgu-node-body";

    if (this.inputs.length > 0) {
      const inList = document.createElement("div");
      inList.className = "zgu-port-list";
      this.inputs.forEach((input, i) => {
        const row = document.createElement("div");
        row.className = "zgu-port-row";
        row.innerHTML = `<div class=\"zgu-port in\" data-type=\"in\" data-index=\"${i}\"></div><span>${input.name}</span>`;
        inList.appendChild(row);
      });
      body.appendChild(inList);
    }

    const custom = document.createElement("div");
    this.buildCustomHTML(custom);
    if (custom.innerHTML.trim() !== "") {
      custom.style.margin = "10px 0";
      body.appendChild(custom);
    }

    if (this.outputs.length > 0) {
      const outList = document.createElement("div");
      outList.className = "zgu-port-list";
      outList.style.marginTop = "10px";
      this.outputs.forEach((output, i) => {
        const row = document.createElement("div");
        row.className = "zgu-port-row out";
        row.innerHTML = `<span>${output.name}</span><div class=\"zgu-port out\" data-type=\"out\" data-index=\"${i}\"></div>`;
        outList.appendChild(row);
      });
      body.appendChild(outList);
    }

    node.appendChild(body);
    container.appendChild(node);
    this.el = node;
    return node;
  }
}

export class CustomNode extends GraphNode {
  constructor({ title, x = 0, y = 0, color = "#334155", inputs = [], outputs = [] }) {
    super({ title, x, y, color });
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

    this.transform = { x: 0, y: 0, k: 1 };

    this.draggingNode = null;
    this.panning = false;
    this.connecting = false;
    this.connectOrigin = null;
    this.selectedNode = null;
    this.selectedLink = null;
    this.startPos = { x: 0, y: 0 };
    this.initialTransform = { x: 0, y: 0 };

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

    this.root.appendChild(this.headerEl);
    this.root.appendChild(this.toolboxEl);
    this.root.appendChild(this.workspaceEl);
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
  }

  updateTransform() {
    this.transformEl.style.transform = `translate(${this.transform.x}px, ${this.transform.y}px) scale(${this.transform.k})`;
    this.gridEl.style.backgroundPosition = `${this.transform.x}px ${this.transform.y}px`;
    this.gridEl.style.backgroundSize = `${30 * this.transform.k}px ${30 * this.transform.k}px`;
  }

  drawBezier(x1, y1, x2, y2) {
    const dist = Math.max(Math.abs(x2 - x1) * 0.5, 50);
    return `M ${x1} ${y1} C ${x1 + dist} ${y1}, ${x2 - dist} ${y2}, ${x2} ${y2}`;
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
      path.setAttribute("d", this.drawBezier(start.x, start.y, end.x, end.y));
    });
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
      this.connecting = true;
      this.connectOrigin = {
        nodeId: Number(nodeEl.dataset.id),
        slot: Number(target.dataset.index),
        el: nodeEl,
      };
      this.tempWire.style.display = "block";
      return;
    }

    if (target.closest("[data-zgu-nodrag='true']") || target.tagName === "INPUT" || target.tagName === "TEXTAREA") {
      return;
    }

    const nodeEl = target.closest(".zgu-node");
    if (nodeEl) {
      this.draggingNode = this.graph.nodes.find((node) => node.id === Number(nodeEl.dataset.id));
      this.startPos = { x: event.clientX, y: event.clientY };
      this.clearSelection();
      this.selectedNode = this.draggingNode;
      this.selectedNode.el.classList.add("selected");
      this.nodesEl.appendChild(nodeEl);
      return;
    }

    this.panning = true;
    this.startPos = { x: event.clientX, y: event.clientY };
    this.initialTransform = { ...this.transform };
    this.clearSelection();
  }

  onPointerMove(event) {
    if (this.connecting) {
      const rect = this.transformEl.getBoundingClientRect();
      const mouseX = (event.clientX - rect.left) / this.transform.k;
      const mouseY = (event.clientY - rect.top) / this.transform.k;
      const start = this.getPortCenter(this.connectOrigin.el, "out", this.connectOrigin.slot);
      this.tempWire.setAttribute("d", this.drawBezier(start.x, start.y, mouseX, mouseY));
      return;
    }

    if (this.draggingNode) {
      const dx = (event.clientX - this.startPos.x) / this.transform.k;
      const dy = (event.clientY - this.startPos.y) / this.transform.k;
      this.draggingNode.x += dx;
      this.draggingNode.y += dy;
      this.draggingNode.el.style.transform = `translate(${this.draggingNode.x}px, ${this.draggingNode.y}px)`;
      this.updateWires();
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
    if (this.connecting) {
      this.connecting = false;
      this.tempWire.style.display = "none";
      const droppedOn = document.elementFromPoint(event.clientX, event.clientY);
      if (droppedOn?.classList.contains("zgu-port") && droppedOn.classList.contains("in")) {
        const targetNodeEl = droppedOn.closest(".zgu-node");
        const targetId = Number(targetNodeEl.dataset.id);
        const targetSlot = Number(droppedOn.dataset.index);
        if (targetId !== this.connectOrigin.nodeId) {
          this.graph.connect(this.connectOrigin.nodeId, this.connectOrigin.slot, targetId, targetSlot, this.options.defaultManualLinkOptions || {});
          this.updateWires();
        }
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

export function createPipelineScene(pipeline, options = {}) {
  const nodeSpacingX = Number(options.nodeSpacingX || 320);
  const nodeSpacingY = Number(options.nodeSpacingY || 170);
  const baseX = Number(options.baseX || 120);
  const baseY = Number(options.baseY || 120);
  const fallbackNodeColor = options.fallbackNodeColor || "#334155";
  const colorMap = options.kindColors || null;
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
        const outputs = sanitizePins(node?.output_pins, ["out"]);
        const kind = String(node?.kind || "node");
        const title = cfg.title || kind;
        const color = resolveNodeColor(kind, colorMap, fallbackNodeColor);

        const domNode = app.factory.custom(x, y, {
          title,
          color,
          inputs,
          outputs,
        });
        domNode.zfPipelineNodeId = String(node?.id || "");
        domNode.zfKind = kind;
        domNode.zfConfig = cloneJsonLike(cfg || {});
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
};

export default graphui;
