/**
 * zeb/d3 0.1 — D3.js v7 runtime for RWE templates.
 *
 * ── FEATURES ────────────────────────────────────────────────────────────────
 *  • Offline: full d3 v7 bundled inline — no CDN.
 *  • Full namespace: d3lib exposes all d3.* symbols.
 *  • MutationObserver auto-mounts [data-zeb-lib="d3"] elements.
 *  • Reactive: stateKey syncs data from page state.
 *  • Built-in chart helpers: bar, line, pie.
 *  • window.__zebD3 registry for imperative access.
 *
 * ── OFFLINE BUNDLE ───────────────────────────────────────────────────────────
 *  cd /tmp/zeb-d3-build
 *  npm install
 *  node_modules/.bin/esbuild entry.mjs \
 *    --bundle --format=esm --minify \
 *    --outfile=d3.bundle.mjs
 *  cp d3.bundle.mjs libraries/zeb/d3/0.1/runtime/
 *
 * ── QUICK REFERENCE ─────────────────────────────────────────────────────────
 *  TSX import:   import D3Bars from "zeb/d3";
 *  Full d3:      import { d3lib } from "zeb/d3";
 *  Imperative:   window.__zebD3.get("chart-id").destroy()
 */

/* ── Static imports — bundled inline by esbuild ── */
import * as d3 from "d3";

/* ── Expose d3 globally for user code ── */
if (typeof window !== "undefined") {
  window.d3 = d3;
}

/* ── Default chart dimensions ── */
const DEFAULT_MARGIN = { top: 20, right: 20, bottom: 32, left: 40 };

/* ── Instance registry ── */
const _instances = new Map();

/* ─────────────────────────────────────────────────────────────────────────────
 * mountBarChart — render a vertical bar chart into a host element.
 *
 * Options:
 *   data        array of numbers, or array of { [xKey]: label, [yKey]: value }
 *   xKey        string  property name for x-axis labels (default "label")
 *   yKey        string  property name for y values (default "value")
 *   color       string  fill color (default "#22c55e")
 *   width       number  explicit width (default: host.clientWidth or 560)
 *   height      number  explicit height (default 260)
 *   margin      object  { top, right, bottom, left }
 *   xLabel      string  optional x-axis label
 *   yLabel      string  optional y-axis label
 *   tooltip     boolean show hover tooltip (default true)
 *
 * Returns:
 *   { svg, update(data), destroy() }
 * ─────────────────────────────────────────────────────────────────────────── */
export function mountBarChart(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/d3: host element is required");

  const xKey    = options.xKey  || "label";
  const yKey    = options.yKey  || "value";
  const color   = options.color || "#22c55e";
  const margin  = { ...DEFAULT_MARGIN, ...(options.margin || {}) };
  const tooltip = options.tooltip !== false;

  /* Normalise data: numbers become { label: i, value: n } */
  function normalise(raw) {
    if (!Array.isArray(raw) || raw.length === 0) return [{ [xKey]: "A", [yKey]: 8 }, { [xKey]: "B", [yKey]: 14 }, { [xKey]: "C", [yKey]: 10 }];
    if (typeof raw[0] === "number") return raw.map((v, i) => ({ [xKey]: String(i), [yKey]: v }));
    return raw;
  }

  let data = normalise(options.data);

  host.innerHTML = "";
  host.style.position = "relative";

  const width  = Number(options.width  || host.clientWidth  || 560);
  const height = Number(options.height || 260);
  const innerW = Math.max(10, width  - margin.left - margin.right);
  const innerH = Math.max(10, height - margin.top  - margin.bottom);

  const svg = d3.select(host)
    .append("svg")
    .attr("width", "100%")
    .attr("viewBox", `0 0 ${width} ${height}`)
    .attr("preserveAspectRatio", "xMidYMid meet");

  const g = svg.append("g").attr("transform", `translate(${margin.left},${margin.top})`);
  const xAxis = g.append("g").attr("transform", `translate(0,${innerH})`);
  const yAxis = g.append("g");
  const bars  = g.append("g").attr("class", "bars");

  /* Tooltip element */
  let tip = null;
  if (tooltip) {
    tip = d3.select(host).append("div")
      .style("position",   "absolute")
      .style("pointer-events", "none")
      .style("background", "rgba(15,23,42,0.9)")
      .style("color",      "#f1f5f9")
      .style("font-size",  "12px")
      .style("padding",    "4px 10px")
      .style("border-radius", "6px")
      .style("opacity",    "0")
      .style("transition", "opacity 0.15s");
  }

  /* Axis labels */
  if (options.xLabel) {
    g.append("text")
      .attr("x", innerW / 2).attr("y", innerH + margin.bottom - 2)
      .attr("text-anchor", "middle").attr("font-size", 11)
      .attr("fill", "#94a3b8").text(options.xLabel);
  }
  if (options.yLabel) {
    g.append("text")
      .attr("transform", "rotate(-90)")
      .attr("x", -innerH / 2).attr("y", -margin.left + 12)
      .attr("text-anchor", "middle").attr("font-size", 11)
      .attr("fill", "#94a3b8").text(options.yLabel);
  }

  function render(d) {
    const x = d3.scaleBand()
      .domain(d.map(row => String(row[xKey])))
      .range([0, innerW])
      .padding(0.2);

    const y = d3.scaleLinear()
      .domain([0, d3.max(d, row => +row[yKey]) || 1])
      .nice()
      .range([innerH, 0]);

    /* Axes */
    xAxis.call(
      d3.axisBottom(x).tickSize(0)
    ).select(".domain").attr("stroke", "#334155");
    xAxis.selectAll("text").attr("fill", "#94a3b8").attr("font-size", 11);

    yAxis.call(
      d3.axisLeft(y).ticks(5).tickSize(-innerW)
    ).select(".domain").attr("stroke", "none");
    yAxis.selectAll(".tick line").attr("stroke", "#1e293b");
    yAxis.selectAll("text").attr("fill", "#94a3b8").attr("font-size", 11);

    /* Bars */
    const rects = bars.selectAll("rect").data(d, row => row[xKey]);

    rects.join(
      enter => enter.append("rect")
        .attr("rx", 4)
        .attr("fill", typeof color === "function" ? color : color)
        .attr("x",      row => x(String(row[xKey])))
        .attr("width",  x.bandwidth())
        .attr("y",      innerH)
        .attr("height", 0)
        .call(enter => enter.transition().duration(400)
          .attr("y",      row => y(+row[yKey]))
          .attr("height", row => innerH - y(+row[yKey]))),
      update => update.call(update => update.transition().duration(400)
        .attr("x",      row => x(String(row[xKey])))
        .attr("width",  x.bandwidth())
        .attr("y",      row => y(+row[yKey]))
        .attr("height", row => innerH - y(+row[yKey]))),
      exit => exit.call(exit => exit.transition().duration(200)
        .attr("height", 0).attr("y", innerH).remove()),
    );

    /* Tooltip handlers */
    if (tip) {
      bars.selectAll("rect")
        .on("mouseover", (event, row) => {
          tip.style("opacity", "1")
             .text(`${row[xKey]}: ${row[yKey]}`);
        })
        .on("mousemove", (event) => {
          const [mx, my] = d3.pointer(event, host);
          tip.style("left", `${mx + 12}px`).style("top", `${my - 20}px`);
        })
        .on("mouseleave", () => {
          tip.style("opacity", "0");
        });
    }
  }

  render(data);

  const instance = {
    svg,
    update(newData) {
      data = normalise(newData);
      render(data);
    },
    destroy() {
      host.innerHTML = "";
    },
  };

  return instance;
}

/* ─────────────────────────────────────────────────────────────────────────────
 * mountLineChart — render a line chart into a host element.
 * Options: data (array of { x, y } or flat numbers), color, width, height,
 *          xKey, yKey, dots (bool, default true), area (bool, default false)
 * ─────────────────────────────────────────────────────────────────────────── */
export function mountLineChart(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/d3: host element is required");

  const xKey   = options.xKey  || "x";
  const yKey   = options.yKey  || "y";
  const color  = options.color || "#38bdf8";
  const margin = { ...DEFAULT_MARGIN, ...(options.margin || {}) };
  const dots   = options.dots !== false;
  const area   = !!options.area;

  function normalise(raw) {
    if (!Array.isArray(raw) || raw.length === 0) {
      return [0,2,1,4,3,5].map((v, i) => ({ [xKey]: i, [yKey]: v }));
    }
    if (typeof raw[0] === "number") return raw.map((v, i) => ({ [xKey]: i, [yKey]: v }));
    return raw;
  }

  let data = normalise(options.data);
  host.innerHTML = "";
  host.style.position = "relative";

  const width  = Number(options.width  || host.clientWidth  || 560);
  const height = Number(options.height || 260);
  const innerW = Math.max(10, width  - margin.left - margin.right);
  const innerH = Math.max(10, height - margin.top  - margin.bottom);

  const svg = d3.select(host)
    .append("svg")
    .attr("width", "100%")
    .attr("viewBox", `0 0 ${width} ${height}`)
    .attr("preserveAspectRatio", "xMidYMid meet");

  const g = svg.append("g").attr("transform", `translate(${margin.left},${margin.top})`);
  g.append("g").attr("transform", `translate(0,${innerH})`).attr("class", "x-axis");
  g.append("g").attr("class", "y-axis");
  const lineG = g.append("g").attr("class", "line-group");

  function render(d) {
    const x = d3.scalePoint()
      .domain(d.map(row => String(row[xKey])))
      .range([0, innerW])
      .padding(0.1);

    const y = d3.scaleLinear()
      .domain([0, d3.max(d, row => +row[yKey]) || 1])
      .nice()
      .range([innerH, 0]);

    g.select(".x-axis")
      .call(d3.axisBottom(x).tickSize(0))
      .select(".domain").attr("stroke", "#334155");
    g.select(".x-axis").selectAll("text").attr("fill", "#94a3b8").attr("font-size", 11);

    g.select(".y-axis")
      .call(d3.axisLeft(y).ticks(5).tickSize(-innerW))
      .select(".domain").attr("stroke", "none");
    g.select(".y-axis").selectAll(".tick line").attr("stroke", "#1e293b");
    g.select(".y-axis").selectAll("text").attr("fill", "#94a3b8").attr("font-size", 11);

    const lineGen = d3.line()
      .x(row => x(String(row[xKey])))
      .y(row => y(+row[yKey]))
      .curve(d3.curveMonotoneX);

    if (area) {
      const areaGen = d3.area()
        .x(row => x(String(row[xKey])))
        .y0(innerH)
        .y1(row => y(+row[yKey]))
        .curve(d3.curveMonotoneX);

      let areaPath = lineG.select(".area-path");
      if (areaPath.empty()) areaPath = lineG.append("path").attr("class", "area-path");
      areaPath
        .datum(d)
        .attr("fill", color)
        .attr("fill-opacity", 0.15)
        .attr("d", areaGen);
    }

    let linePath = lineG.select(".line-path");
    if (linePath.empty()) linePath = lineG.append("path").attr("class", "line-path");
    linePath
      .datum(d)
      .attr("fill", "none")
      .attr("stroke", color)
      .attr("stroke-width", 2.5)
      .attr("d", lineGen);

    if (dots) {
      lineG.selectAll(".dot")
        .data(d, row => row[xKey])
        .join("circle")
        .attr("class", "dot")
        .attr("cx", row => x(String(row[xKey])))
        .attr("cy", row => y(+row[yKey]))
        .attr("r", 4)
        .attr("fill", color)
        .attr("stroke", "#0f172a")
        .attr("stroke-width", 2);
    }
  }

  render(data);

  return {
    svg,
    update(newData) { data = normalise(newData); render(data); },
    destroy() { host.innerHTML = ""; },
  };
}

/* ─────────────────────────────────────────────────────────────────────────────
 * mountPieChart — render a pie / donut chart into a host element.
 * Options: data (array of { label, value }), colors (array), donut (bool),
 *          width, height, showLabels (bool)
 * ─────────────────────────────────────────────────────────────────────────── */
export function mountPieChart(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/d3: host element is required");

  const labelKey = options.labelKey || "label";
  const valueKey = options.valueKey || "value";
  const donut    = !!options.donut;
  const COLORS   = options.colors || ["#38bdf8", "#22c55e", "#f59e0b", "#e11d48", "#a855f7", "#14b8a6"];

  function normalise(raw) {
    if (!Array.isArray(raw) || raw.length === 0)
      return ["A","B","C"].map((l, i) => ({ [labelKey]: l, [valueKey]: [40,35,25][i] }));
    if (typeof raw[0] === "number") return raw.map((v, i) => ({ [labelKey]: String.fromCharCode(65+i), [valueKey]: v }));
    return raw;
  }

  let data = normalise(options.data);
  host.innerHTML = "";

  const size   = Number(options.width || options.height || Math.min(host.clientWidth || 300, 300));
  const radius = size / 2 - 8;
  const inner  = donut ? radius * 0.55 : 0;

  const svg = d3.select(host)
    .append("svg")
    .attr("width", "100%")
    .attr("viewBox", `0 0 ${size} ${size}`)
    .attr("preserveAspectRatio", "xMidYMid meet")
    .append("g")
    .attr("transform", `translate(${size/2},${size/2})`);

  const pieGen  = d3.pie().value(row => +row[valueKey]).sort(null);
  const arcGen  = d3.arc().innerRadius(inner).outerRadius(radius);
  const arcHover = d3.arc().innerRadius(inner).outerRadius(radius + 6);

  function render(d) {
    const arcs = svg.selectAll(".slice").data(pieGen(d), (_, i) => i);

    arcs.join(
      enter => enter.append("path")
        .attr("class", "slice")
        .attr("fill", (_, i) => COLORS[i % COLORS.length])
        .attr("stroke", "#0f172a")
        .attr("stroke-width", 2)
        .attr("d", arcGen)
        .on("mouseenter", function() {
          d3.select(this).transition().duration(150).attr("d", arcHover);
        })
        .on("mouseleave", function() {
          d3.select(this).transition().duration(150).attr("d", arcGen);
        }),
      update => update
        .attr("fill", (_, i) => COLORS[i % COLORS.length])
        .attr("d", arcGen),
      exit => exit.remove(),
    );

    if (options.showLabels !== false) {
      const labelArcGen = d3.arc().innerRadius(radius * 0.75).outerRadius(radius * 0.75);
      svg.selectAll(".slice-label").data(pieGen(d), (_, i) => i)
        .join("text")
        .attr("class", "slice-label")
        .attr("transform", arc => `translate(${labelArcGen.centroid(arc)})`)
        .attr("text-anchor", "middle")
        .attr("font-size", 11)
        .attr("fill", "#f1f5f9")
        .text(arc => arc.data[labelKey]);
    }
  }

  render(data);

  return {
    svg,
    update(newData) { data = normalise(newData); render(data); },
    destroy() { host.innerHTML = ""; },
  };
}

/* ── Auto-mount system ──────────────────────────────────────────────────────
 * Watches for [data-zeb-lib="d3"] elements in the DOM. Parses data-config
 * and calls the appropriate chart mount helper. Reactive via rwe:state:change.
 *
 * type "raw" — general-purpose canvas: zeb:d3:ready fires with { d3, container, id }
 *              so the user can call any d3 function directly.
 */
const CHART_MOUNTS = {
  bar:  mountBarChart,
  line: mountLineChart,
  pie:  mountPieChart,
  raw:  function mountD3Raw(container) {
    container.innerHTML = "";
    return { svg: null, update() {}, destroy() { container.innerHTML = ""; } };
  },
};

function mountD3Chart(container) {
  if (container._zdMounted) return;
  container._zdMounted = true;

  let config = {};
  try { config = JSON.parse(container.dataset.config || "{}"); } catch {}

  const type     = config.type     || "bar";
  const stateKey = config.stateKey || null;
  const chartFn  = CHART_MOUNTS[type];

  if (!chartFn) {
    console.warn(`zeb/d3: unknown chart type "${type}"`);
    return;
  }

  /* Read initial data from page state if stateKey set */
  let opts = { ...config };
  if (stateKey && window.__rwePageState?.[stateKey]) {
    opts.data = window.__rwePageState[stateKey];
  }

  if (!container.id) container.id = `zd3-${Math.random().toString(36).slice(2, 8)}`;
  const instanceId = container.id;

  const instance = chartFn(container, opts);

  /* Reactive: update when page state changes */
  let _listener = null;
  if (stateKey) {
    _listener = (e) => {
      if (e.detail?.[stateKey] !== undefined) {
        instance.update(e.detail[stateKey]);
      }
    };
    window.addEventListener("rwe:state:change", _listener);
  }

  const wrapped = {
    ...instance,
    destroy() {
      if (_listener) window.removeEventListener("rwe:state:change", _listener);
      instance.destroy();
      _instances.delete(instanceId);
      container._zdMounted = false;
    },
  };

  _instances.set(instanceId, wrapped);
  container.dispatchEvent(new CustomEvent("zeb:d3:ready", {
    bubbles: true,
    /* d3 always included so raw-type handlers can access the full namespace */
    detail: { instance: wrapped, id: instanceId, d3: d3, container },
  }));
}

function destroyD3Chart(node) {
  if (node.nodeType !== 1) return;
  if (node._zdMounted && node.id) _instances.get(node.id)?.destroy();
  node.querySelectorAll?.("[data-zeb-lib='d3']").forEach((el) => {
    if (el._zdMounted && el.id) _instances.get(el.id)?.destroy();
  });
}

const _observer = new MutationObserver((mutations) => {
  for (const mut of mutations) {
    for (const node of mut.addedNodes) {
      if (node.nodeType !== 1) continue;
      if (node.matches?.("[data-zeb-lib='d3']")) mountD3Chart(node);
      node.querySelectorAll?.("[data-zeb-lib='d3']").forEach(mountD3Chart);
    }
    for (const node of mut.removedNodes) destroyD3Chart(node);
  }
});

if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      _observer.observe(document.body, { childList: true, subtree: true });
      document.querySelectorAll("[data-zeb-lib='d3']").forEach(mountD3Chart);
    });
  } else {
    _observer.observe(document.body, { childList: true, subtree: true });
    document.querySelectorAll("[data-zeb-lib='d3']").forEach(mountD3Chart);
  }
}

/* ── Public surface ── */
if (typeof window !== "undefined") {
  window.__zebD3 = {
    get(id)      { return _instances.get(id); },
    d3,
    mountBarChart,
    mountLineChart,
    mountPieChart,
  };
}

/* ── Exports ── */

export { d3 };

/**
 * useD3(callback, deps?) — hook for arbitrary d3 work inside Preact components.
 *
 * Returns a ref — attach it to any DOM element. The callback fires after mount
 * with (container, d3) so you can call any d3 function directly.
 * Return a cleanup function from the callback to run on unmount.
 *
 * Example:
 *   const ref = useD3((el, d3) => {
 *     const svg = d3.select(el).append("svg");
 *     return () => svg.remove();
 *   });
 *   return <div ref={ref} class="w-full h-[400px]" />;
 */
export function useD3(callback, deps) {
  const ref = useRef(null);
  useEffect(() => {
    if (!ref.current) return;
    return callback(ref.current, d3);
  }, deps ?? []);
  return ref;
}

/**
 * D3Bars — Preact component for bar charts in RWE templates.
 *
 * Uses useRef + useEffect to prevent Preact hydration conflicts.
 * The component renders a display:contents wrapper; useEffect appends
 * the real [data-zeb-lib="d3"] div — MutationObserver catches it → mounts chart.
 *
 * Props:
 *   type        "bar" | "line" | "pie"   chart type (default "bar")
 *   data        array                    chart data
 *   xKey        string                   x-axis key for object data
 *   yKey        string                   y-axis key for object data
 *   color       string                   fill / stroke color
 *   width       number                   explicit width
 *   height      string | number          CSS height (default "260px")
 *   stateKey    string                   page state key for reactive data
 *   className   string                   Tailwind classes on container
 *   id          string                   container id
 *   area        boolean                  area fill under line (line only)
 *   donut       boolean                  donut mode (pie only)
 */
function D3Bars(props) {
  const _h         = globalThis.h;
  const _useRef    = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = {
    type:     props.type     || "bar",
    data:     props.data     || [],
    xKey:     props.xKey,
    yKey:     props.yKey,
    color:    props.color,
    height:   typeof props.height === "number" ? props.height : parseInt(props.height || "260"),
    stateKey: props.stateKey || null,
    area:     props.area,
    donut:    props.donut,
  };

  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const inner = document.createElement("div");
      inner.setAttribute("data-zeb-lib", "d3");
      inner.setAttribute("data-config", JSON.stringify(config));
      if (props.id) inner.id = props.id;
      inner.style.width  = "100%";
      inner.style.height = props.height || "260px";
      if (props.className) inner.className = props.className;
      wrap.appendChild(inner);

      return () => { inner.remove(); };
    }, []);

    return _h("div", {
      ref:                wrapRef,
      "data-zeb-wrapper": "D3Bars",
      style:              { display: "contents" },
    });
  }

  /* SSR fallback */
  return _h("div", {
    "data-zeb-lib":     "d3",
    "data-zeb-wrapper": "D3Bars",
    "data-config":      JSON.stringify(config),
    id:                 props.id,
    style:              { width: "100%", height: props.height || "260px" },
    class:              props.className,
  });
}

/**
 * D3Canvas — general-purpose D3 canvas for arbitrary d3 usage.
 *
 * After mount, the element dispatches `zeb:d3:ready` with:
 *   { d3, container, id, instance }
 * where `d3` is the full d3 namespace — call any d3 function from there.
 *
 * Props:
 *   height      string | number  CSS height (default "300px")
 *   config      object           arbitrary config passed to data-config
 *   id          string           container id
 *   className   string           Tailwind classes
 */
function D3Canvas(props) {
  const _h         = globalThis.h;
  const _useRef    = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = Object.assign({ type: "raw" }, (props && props.config) || {});

  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const inner = document.createElement("div");
      inner.setAttribute("data-zeb-lib", "d3");
      inner.setAttribute("data-config", JSON.stringify(config));
      if (props.id) inner.id = props.id;
      inner.style.width  = "100%";
      inner.style.height = props.height || "300px";
      if (props.className) inner.className = props.className;
      wrap.appendChild(inner);

      return () => { inner.remove(); };
    }, []);

    return _h("div", {
      ref:                wrapRef,
      "data-zeb-wrapper": "D3Canvas",
      style:              { display: "contents" },
    });
  }

  /* SSR fallback */
  return _h("div", {
    "data-zeb-lib":     "d3",
    "data-zeb-wrapper": "D3Canvas",
    "data-config":      JSON.stringify(config),
    id:                 props.id,
    style:              { width: "100%", height: props.height || "300px" },
    class:              props.className,
  });
}
