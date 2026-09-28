import { sqlStringLiteral } from "@/components/db/table-data";
import {
  edgePropertyColumns,
  edgeRowsSql,
  edgeTableFor,
  gqlLabel,
  labelColumn,
  relationCountSql,
  relationDefs,
  tableByKey,
} from "@/components/db/edge-tables";

/**
 * The graph as the Graph tab draws it, and the walks it offers over one edge
 * type. Pure: SQL in, SQL out, nothing run here.
 */

/**
 * The tables of rows and every kind of edge between them. A link carries the
 * edge table behind it, or `loose` when an API or an import wrote the edges
 * with no table.
 */
export function graphModel(tables, shownEdges) {
  const nodes = (tables || []).filter((table) => !table?.edge);
  const links = relationDefs(tables, shownEdges).map((def) => {
    const edgeTable = edgeTableFor(tables, def.type);
    return { ...def, id: `${def.from}:${def.type}:${def.to}`, edgeTable, loose: !edgeTable };
  });
  return { nodes, links };
}

function node(variable, name, key) {
  const where = key ? ` WHERE ${variable}._key = '${sqlStringLiteral(key)}'` : "";
  return `(${variable}:${gqlLabel(name)}${where})`;
}

/** A far row's key, and its readable name when its table has one. */
function farColumns(tables, variable, name) {
  const label = labelColumn(tableByKey(tables, name));
  return `${variable}._key AS _key${label ? `, ${variable}.${label} AS ${label}` : ""}`;
}

/**
 * The walks worth starting from over one edge type: forward and back from
 * one row, a count, the rows with the most edges, one hop further when
 * another edge type leaves the far table, and an edge table's own rows.
 * `key` is the row a walk starts from; left empty, the walks that need one
 * name a placeholder the reader replaces.
 */
export function walkTemplates(link, { tables, links, key }) {
  const start = String(key || "").trim() || "row_key";
  const type = gqlLabel(link.type);
  const properties = link.edgeTable
    ? edgePropertyColumns(link.edgeTable).map((attr) => `, e.${attr.name} AS ${attr.name}`).join("")
    : "";
  const walks = [
    {
      id: "forward",
      title: `What one ${link.from} row reaches`,
      sql: `SELECT * FROM GRAPH_TABLE (base MATCH ${node("a", link.from, start)}-[e:${type}]->${node("b", link.to, "")} RETURN ${farColumns(tables, "b", link.to)}${properties}) LIMIT 50`,
    },
    {
      id: "back",
      title: `What reaches one ${link.to} row`,
      sql: `SELECT * FROM GRAPH_TABLE (base MATCH ${node("a", link.from, "")}-[e:${type}]->${node("b", link.to, start)} RETURN ${farColumns(tables, "a", link.from)}${properties}) LIMIT 50`,
    },
    {
      id: "count",
      title: "How many edges",
      sql: relationCountSql(link),
    },
    {
      id: "busiest",
      title: `The ${link.from} rows with the most edges`,
      sql: `SELECT source, count(*) AS edges FROM GRAPH_TABLE (base MATCH ${node("a", link.from, "")}-[:${type}]->${node("b", link.to, "")} RETURN a._key AS source) GROUP BY source ORDER BY edges DESC LIMIT 10`,
    },
  ];
  const next = (links || []).find((other) => other.from === link.to && other.id !== link.id);
  if (next) {
    walks.push({
      id: "two-hops",
      title: `Two hops: then ${next.type} to ${next.to}`,
      sql: `SELECT * FROM GRAPH_TABLE (base MATCH ${node("a", link.from, start)}-[:${type}]->${node("b", link.to, "")}-[:${gqlLabel(next.type)}]->${node("c", next.to, "")} RETURN b._key AS via, ${farColumns(tables, "c", next.to)}) LIMIT 50`,
    });
  }
  if (link.edgeTable) {
    walks.push({ id: "rows", title: "The edge table's rows", sql: edgeRowsSql(link.edgeTable, 200) });
  }
  return walks;
}
