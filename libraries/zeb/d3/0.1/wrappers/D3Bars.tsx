/**
 * D3Bars — RWE wrapper for D3.js charts.
 *
 * IMPORT in TSX page:
 *   import D3Bars from "zeb/d3";
 *   import { mountBarChart, mountLineChart, mountPieChart, d3lib } from "zeb/d3";
 *
 * ─── Simple bar chart ─────────────────────────────────────────────────────
 *   <D3Bars
 *     data={[8, 14, 10, 18, 12, 16]}
 *     color="#22c55e"
 *     height="260px"
 *   />
 *
 * ─── Object data with keys ───────────────────────────────────────────────
 *   <D3Bars
 *     type="bar"
 *     data={[{ month: "Jan", sales: 120 }, { month: "Feb", sales: 95 }]}
 *     xKey="month"
 *     yKey="sales"
 *     color="#38bdf8"
 *     height="300px"
 *   />
 *
 * ─── Reactive: data from page state ──────────────────────────────────────
 *   const [chartData, setChartData] = usePageState("salesData", []);
 *   useEffect(() => {
 *     fetch("/api/sales").then(r => r.json()).then(setChartData);
 *   }, []);
 *
 *   <D3Bars type="bar" stateKey="salesData" height="300px" />
 *
 * ─── Line chart ───────────────────────────────────────────────────────────
 *   <D3Bars type="line" data={metrics} xKey="date" yKey="value" area color="#a855f7" />
 *
 * ─── Pie / donut chart ────────────────────────────────────────────────────
 *   <D3Bars type="pie" data={[{ label: "A", value: 40 }, { label: "B", value: 60 }]} donut />
 */
export const app = {};

export default function D3Bars(props) {
  const config = JSON.stringify({
    type:     props.type     || "bar",
    data:     props.data     || [],
    xKey:     props.xKey,
    yKey:     props.yKey,
    color:    props.color,
    height:   typeof props.height === "number" ? props.height : parseInt(props.height || "260"),
    stateKey: props.stateKey || null,
    area:     props.area,
    donut:    props.donut,
  });

  return (
    <div
      data-zeb-lib="d3"
      data-zeb-wrapper="D3Bars"
      data-config={config}
      id={props.id}
      className={props.className}
      style={{ width: "100%", height: props.height || "260px" }}
    />
  );
}
