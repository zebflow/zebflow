import { sqlStringLiteral } from "@/components/db/table-data";

/**
 * Edge tables and the graph walks over them, as the Studio asks for them.
 *
 * An edge table is a table with two REFERENCES columns that a property graph
 * declares as its edges; it is the one way the Studio writes an edge. An edge
 * type with no edge table behind it is a loose edge — written by an API or a
 * bulk import — and the Studio shows it without offering to change it.
 */

/** The edge table whose edges carry this type, if there is one. */
export function edgeTableFor(tables, type) {
  const wanted = String(type || "").trim();
  if (!wanted) return null;
  return (tables || []).find((table) => {
    const edge = table?.edge;
    if (!edge) return false;
    return String(edge.label || table.table) === wanted;
  }) || null;
}

/** Every edge table with this table at one of its two ends. */
export function edgeTablesTouching(tables, tableKey) {
  return (tables || []).filter((table) => {
    const edge = table?.edge;
    return edge && (edge.source_table === tableKey || edge.destination_table === tableKey);
  });
}

/** The table a catalog key names, if the catalog has it. */
export function tableByKey(tables, key) {
  return (tables || []).find((table) => table?.key === key || table?.table === key) || null;
}

const LABEL_COLUMNS = ["title", "name", "full_name", "fullname", "label", "slug", "email"];

/**
 * The column that names a row to a reader: a conventional name when the
 * table has one, else its first text column. Read from the catalog, so the
 * walk asks for a column that exists rather than guessing.
 */
export function labelColumn(table) {
  const attributes = Array.isArray(table?.attributes) ? table.attributes : [];
  const names = attributes.map((attr) => String(attr?.name || ""));
  const conventional = LABEL_COLUMNS.find((name) => names.includes(name));
  if (conventional) return conventional;
  const text = attributes.find((attr) => {
    const kind = String(attr?.declared || attr?.kind || "").toLowerCase();
    return kind === "text" || kind === "string";
  });
  return text ? String(text.name) : "";
}

/**
 * The rows one row reaches over one edge type, in one direction, as GQL.
 * Answers `_key` and, when the far table has one, `_label`.
 */
export function relationWalkSql({ from, type, to, key, outgoing, farLabel }) {
  const anchored = outgoing ? "a" : "b";
  const far = outgoing ? "b" : "a";
  const label = farLabel ? `, ${far}.${farLabel} AS _label` : "";
  return `SELECT * FROM GRAPH_TABLE (base MATCH (a:${from}${anchored === "a" ? ` WHERE a._key = '${sqlStringLiteral(key)}'` : ""})-[:${type}]->(b:${to}${anchored === "b" ? ` WHERE b._key = '${sqlStringLiteral(key)}'` : ""}) RETURN ${far}._key AS _key${label})`;
}

/** How many edges of one type join two tables, as GQL. */
export function relationCountSql({ from, type, to }) {
  return `SELECT count(*) AS count FROM GRAPH_TABLE (base MATCH (a:${from})-[:${type}]->(b:${to}) RETURN b._key AS _key)`;
}

/** Removing one edge from its edge table: the WHERE names both ends. */
export function edgeDeleteSql(edgeTable, sourceKey, destinationKey) {
  const edge = edgeTable.edge;
  return `DELETE FROM ${edgeTable.key} WHERE ${edge.source} = '${sqlStringLiteral(sourceKey)}' AND ${edge.destination} = '${sqlStringLiteral(destinationKey)}'`;
}

/**
 * Adding one edge to its edge table, its properties as the table's columns.
 * Empty property inputs are left out, so a column's DEFAULT applies.
 */
export function edgeInsertSql(edgeTable, sourceKey, destinationKey, properties) {
  const edge = edgeTable.edge;
  const columns = [edge.source, edge.destination];
  const values = [`'${sqlStringLiteral(sourceKey)}'`, `'${sqlStringLiteral(destinationKey)}'`];
  const declared = new Map(
    edgePropertyColumns(edgeTable).map((attr) => [attr.name, String(attr.declared || attr.kind || "").toUpperCase()]),
  );
  Object.entries(properties || {}).forEach(([name, value]) => {
    const text = String(value ?? "").trim();
    if (!text || !declared.has(name)) return;
    // The column's own type decides the literal, never the text's look: a
    // TEXT column holding "123" stays text.
    const type = declared.get(name);
    // Unquoted only when the text is exactly a number or a boolean, so a
    // value never reaches the statement as anything but a literal.
    const bare = /^(INT|BIGINT|SMALLINT|REAL|DOUBLE|NUMBER|NUMERIC|BOOLEAN)/.test(type)
      && /^(-?\d+(\.\d+)?|true|false)$/i.test(text);
    columns.push(name);
    values.push(bare ? text : `'${sqlStringLiteral(text)}'`);
  });
  return `INSERT INTO ${edgeTable.key} (${columns.join(", ")}) VALUES (${values.join(", ")})`;
}

/** An edge table's own columns: everything but its two ends. */
export function edgePropertyColumns(edgeTable) {
  const edge = edgeTable?.edge || {};
  return (edgeTable?.attributes || []).filter(
    (attr) => attr?.name !== edge.source && attr?.name !== edge.destination,
  );
}
