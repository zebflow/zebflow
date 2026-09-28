import { cx } from "zeb/react";

const BOX_W = 170;
const BOX_H = 44;
const COL_GAP = 270;
const ROW_GAP = 84;
const PAD = 40;
// Room above the first row for the loop of a table joined to itself.
const TOP = 72;
// Room on the left for the arrows that stay within one column.
const LEFT = 110;

/**
 * The tables of rows as boxes, one column per schema, and every kind of edge
 * between them as an arrow labelled with its type. An edge table's type is a
 * solid line, a loose type a dashed one. Choosing an arrow is handed up;
 * a box is a link to its table, at `tableHref(key)`.
 */
export default function GraphDiagram({ nodes, links, selectedId, onSelectLink, tableHref }) {
  const layout = layoutNodes(nodes);
  const width = LEFT + PAD + Math.max(1, layout.columns) * COL_GAP - (COL_GAP - BOX_W);
  const height = TOP + PAD + Math.max(1, layout.rows) * ROW_GAP;
  const bends = new Map();

  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className="block" role="img" aria-label="Tables and the edges between them">
      <defs>
        <marker id="graph-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M0 0L10 5L0 10z" fill="currentColor" />
        </marker>
      </defs>

      {layout.schemas.map((schema) => (
        <text key={schema.name} x={schema.x} y={24} className="fill-current text-[11px] uppercase tracking-[0.12em] text-muted-foreground">
          {schema.name}
        </text>
      ))}

      {links.map((link) => {
        const from = layout.at.get(link.from);
        const to = layout.at.get(link.to);
        if (!from || !to) return null;
        const pair = [link.from, link.to].sort().join("|");
        const nth = bends.get(pair) || 0;
        bends.set(pair, nth + 1);
        const path = linkPath(from, to, nth);
        const selected = link.id === selectedId;
        return (
          <g
            key={link.id}
            className={cx("cursor-pointer", selected ? "text-primary" : "text-muted-foreground hover:text-foreground")}
            onClick={() => onSelectLink(link.id)}
          >
            <path d={path.d} fill="none" stroke="transparent" strokeWidth="12" />
            <path
              d={path.d}
              fill="none"
              stroke="currentColor"
              strokeWidth={selected ? 2.25 : 1.5}
              strokeDasharray={link.loose ? "5 4" : undefined}
              markerEnd="url(#graph-arrow)"
            />
            <text x={path.labelX} y={path.labelY} textAnchor="middle" className="fill-current text-[11px]">
              {link.type}
            </text>
          </g>
        );
      })}

      {nodes.map((table) => {
        const at = layout.at.get(table.graphName);
        if (!at) return null;
        return (
          <a key={table.key} href={tableHref(table.key)} className="cursor-pointer text-foreground">
            <rect x={at.x} y={at.y} width={BOX_W} height={BOX_H} rx="8" className="fill-current text-card" />
            <rect x={at.x} y={at.y} width={BOX_W} height={BOX_H} rx="8" fill="none" stroke="currentColor" className="text-border hover:text-primary" />
            <text x={at.x + 12} y={at.y + 19} className="fill-current text-[13px] font-medium">
              {table.table}
            </text>
            <text x={at.x + 12} y={at.y + 34} className="fill-current text-[11px] text-muted-foreground">
              {table.rowCount} {table.rowCount === 1 ? "row" : "rows"}
            </text>
          </a>
        );
      })}
    </svg>
  );
}

/** Each table's box position, by the name edges use for it. */
function layoutNodes(nodes) {
  const bySchema = new Map();
  (nodes || []).forEach((table) => {
    const schema = table.schema === "default" ? "public" : table.schema;
    if (!bySchema.has(schema)) bySchema.set(schema, []);
    bySchema.get(schema).push(table);
  });
  const names = Array.from(bySchema.keys()).sort((a, b) => (a === "public" ? -1 : b === "public" ? 1 : a.localeCompare(b)));
  const at = new Map();
  let rows = 0;
  const schemas = names.map((name, column) => {
    const x = LEFT + column * COL_GAP;
    const tables = bySchema.get(name).slice().sort((a, b) => a.table.localeCompare(b.table));
    tables.forEach((table, row) => at.set(table.graphName, { x, y: TOP + row * ROW_GAP }));
    rows = Math.max(rows, tables.length);
    return { name, x };
  });
  return { at, schemas, columns: names.length, rows };
}

/**
 * One arrow between two boxes: side to side across columns; within one
 * column bowed out to the left, so the right side stays with the arrows
 * that cross; a table joined to itself loops over its top edge. `nth` bends
 * each further arrow between the same pair more, so labels part.
 */
function linkPath(from, to, nth) {
  const cy = (box) => box.y + BOX_H / 2;
  if (from === to) {
    const x = from.x + BOX_W - 70 - nth * 50;
    const reach = 30;
    return {
      d: `M${x} ${from.y} C${x - 6} ${from.y - reach}, ${x + 46} ${from.y - reach}, ${x + 40} ${from.y}`,
      labelX: x + 20,
      labelY: from.y - reach + 2,
    };
  }
  if (from.x === to.x) {
    const x = from.x;
    const bow = 60 + nth * 26;
    const mid = (cy(from) + cy(to)) / 2;
    return { d: `M${x} ${cy(from)} C${x - bow} ${cy(from)}, ${x - bow} ${cy(to)}, ${x} ${cy(to)}`, labelX: x - bow * 0.75, labelY: mid - 4 };
  }
  const forward = to.x > from.x;
  const x1 = forward ? from.x + BOX_W : from.x;
  const x2 = forward ? to.x : to.x + BOX_W;
  const lift = nth * 22;
  const dx = (x2 - x1) / 2;
  const d = `M${x1} ${cy(from)} C${x1 + dx} ${cy(from) - lift}, ${x2 - dx} ${cy(to) - lift}, ${x2} ${cy(to)}`;
  return { d, labelX: (x1 + x2) / 2, labelY: (cy(from) + cy(to)) / 2 - lift * 0.75 - 6 };
}
