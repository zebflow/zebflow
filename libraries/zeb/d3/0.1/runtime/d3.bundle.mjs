function ensureElement(host) {
  if (!(host instanceof Element)) {
    throw new Error("zeb/d3: host element is required");
  }
}

export function ensureD3(explicit) {
  if (explicit) return explicit;
  if (typeof globalThis !== "undefined" && globalThis.d3) return globalThis.d3;
  throw new Error("zeb/d3: global d3 is missing");
}

export function mountBarChart(host, options = {}) {
  ensureElement(host);
  const d3 = ensureD3(options.d3);
  const data = Array.isArray(options.data) && options.data.length > 0
    ? options.data
    : [8, 14, 10, 18, 12, 16];

  host.innerHTML = "";
  const width = Number(options.width || host.clientWidth || 560);
  const height = Number(options.height || 260);
  const margin = { top: 16, right: 16, bottom: 24, left: 24 };
  const innerW = Math.max(10, width - margin.left - margin.right);
  const innerH = Math.max(10, height - margin.top - margin.bottom);

  const svg = d3
    .select(host)
    .append("svg")
    .attr("width", width)
    .attr("height", height)
    .attr("viewBox", `0 0 ${width} ${height}`);

  const x = d3.scaleBand().domain(data.map((_, i) => String(i))).range([0, innerW]).padding(0.16);
  const y = d3.scaleLinear().domain([0, d3.max(data) || 1]).nice().range([innerH, 0]);

  const g = svg.append("g").attr("transform", `translate(${margin.left},${margin.top})`);
  g.selectAll("rect")
    .data(data)
    .enter()
    .append("rect")
    .attr("x", (_, i) => x(String(i)))
    .attr("y", (d) => y(d))
    .attr("width", x.bandwidth())
    .attr("height", (d) => innerH - y(d))
    .attr("rx", 6)
    .attr("fill", options.color || "#22c55e");

  return {
    destroy() {
      host.innerHTML = "";
    },
  };
}

export const d3lib = {
  ensureD3,
  mountBarChart,
};
