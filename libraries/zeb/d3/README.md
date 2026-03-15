# zeb/d3

D3.js v7 for data visualization in RWE templates.
Fully offline — full `d3` v7 namespace bundled inline, no CDN.
Bar, line, and pie charts via `D3Bars` component or imperative mount helpers.

## Import

```tsx
import D3Bars from "zeb/d3";
// or specific helpers:
import { mountBarChart, mountLineChart, mountPieChart, d3lib } from "zeb/d3";
```

---

## `D3Bars` Component

```tsx
<D3Bars
  type="bar"
  data={[8, 14, 10, 18, 12]}
  color="#22c55e"
  height="260px"
  stateKey="chartData"
  className="rounded-lg"
/>
```

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `type` | `"bar" \| "line" \| "pie"` | `"bar"` | Chart type |
| `data` | `number[] \| object[]` | `[]` | Chart data |
| `xKey` | `string` | `"label"` | Object property for x-axis / category |
| `yKey` | `string` | `"value"` | Object property for y-axis / numeric value |
| `color` | `string` | type-specific | Fill or stroke color |
| `height` | `string \| number` | `"260px"` | CSS height of the container |
| `stateKey` | `string` | — | Page state key for reactive data updates |
| `id` | `string` | auto | Container id for `window.__zebD3.get(id)` |
| `className` | `string` | — | Tailwind classes on container |
| `area` | `boolean` | `false` | Area fill under line (line chart only) |
| `donut` | `boolean` | `false` | Donut hole (pie chart only) |

---

## Chart types

### Bar

```tsx
// Numbers array:
<D3Bars data={[8, 14, 10, 18]} color="#22c55e" />

// Objects:
<D3Bars
  type="bar"
  data={[{ month: "Jan", sales: 120 }, { month: "Feb", sales: 95 }]}
  xKey="month"
  yKey="sales"
  color="#38bdf8"
/>
```

### Line

```tsx
<D3Bars
  type="line"
  data={[{ date: "Mon", value: 30 }, { date: "Tue", value: 55 }]}
  xKey="date"
  yKey="value"
  color="#a855f7"
  area
/>
```

### Pie / Donut

```tsx
<D3Bars
  type="pie"
  data={[{ label: "Mobile", value: 60 }, { label: "Desktop", value: 30 }, { label: "Tablet", value: 10 }]}
  donut
  height="280px"
/>
```

---

## Patterns

### API-first: data loads after fetch

```tsx
const [data, setData] = usePageState("salesData", []);

useEffect(() => {
  fetch("/api/sales/monthly")
    .then(r => r.json())
    .then(setData);
}, []);

<D3Bars
  type="bar"
  stateKey="salesData"
  xKey="month"
  yKey="revenue"
  color="#22c55e"
  height="300px"
/>
```

When `setData(rows)` fires `rwe:state:change`, the chart updates automatically.

### Real-time updates

```tsx
const [metrics, setMetrics] = usePageState("metrics", []);

useEffect(() => {
  const id = setInterval(() => {
    setMetrics(prev => [...prev.slice(-19), { t: Date.now(), v: Math.random() * 100 }]);
  }, 1000);
  return () => clearInterval(id);
}, []);

<D3Bars type="line" stateKey="metrics" xKey="t" yKey="v" color="#f59e0b" area />
```

### Dashboard with multiple charts

```tsx
<div class="grid grid-cols-2 gap-4">
  <D3Bars type="bar"  stateKey="barData"  height="220px" />
  <D3Bars type="line" stateKey="lineData" height="220px" area />
  <D3Bars type="pie"  stateKey="pieData"  height="220px" donut />
</div>
```

---

## Imperative API — `window.__zebD3`

```ts
const inst = window.__zebD3.get("my-chart");  // instance | undefined

inst.update(newData);  // replace data + re-render
inst.destroy();        // remove chart, clean up listeners

// Full d3 namespace:
window.__zebD3.d3.select("...").attr("fill", "red");

// Mount helpers:
window.__zebD3.mountBarChart(el, options);
window.__zebD3.mountLineChart(el, options);
window.__zebD3.mountPieChart(el, options);
```

---

## Mount helpers

```tsx
import { mountBarChart, mountLineChart, mountPieChart } from "zeb/d3";

// Bar
const bar = mountBarChart(document.getElementById("chart"), {
  data:     [{ label: "Q1", value: 120 }, { label: "Q2", value: 95 }],
  xKey:     "label",
  yKey:     "value",
  color:    "#22c55e",
  height:   260,
  xLabel:   "Quarter",
  yLabel:   "Revenue",
  tooltip:  true,   // default
});

bar.update([...]);  // re-render with new data
bar.destroy();      // clean up

// Line
const line = mountLineChart(el, {
  data:  [0, 2, 1, 5, 3, 8],
  color: "#38bdf8",
  area:  true,
});

// Pie
const pie = mountPieChart(el, {
  data:   [{ label: "A", value: 40 }, { label: "B", value: 60 }],
  donut:  true,
  colors: ["#22c55e", "#38bdf8"],
});
```

---

## Full d3 namespace

```tsx
import { d3lib } from "zeb/d3";
// d3lib is the full d3 v7 namespace:
// d3lib.select, d3lib.scaleLinear, d3lib.axisBottom, d3lib.csv, d3lib.zoom, etc.
```

---

## Events

### `zeb:d3:ready`

Fires once when the chart finishes mounting.

```tsx
document.getElementById("my-chart").addEventListener("zeb:d3:ready", (e) => {
  const { instance, id } = e.detail;
  // Imperative post-mount customisation:
  instance.update(freshData);
});
```

---

## Bundle details

| Property | Value |
|----------|-------|
| Package | `d3` v7 |
| Bundle | `runtime/d3.bundle.mjs` (~286 KB minified) |
| CDN fetches | **None** — fully offline |
| Build tool | esbuild |

### Rebuild

```sh
cd /tmp/zeb-d3-build
cp libraries/zeb/d3/0.1/runtime/entry.mjs .
cp libraries/zeb/d3/0.1/runtime/package.json .
npm install
node_modules/.bin/esbuild entry.mjs --bundle --format=esm --minify \
  --outfile=d3.bundle.mjs
cp d3.bundle.mjs libraries/zeb/d3/0.1/runtime/
cargo build
```
