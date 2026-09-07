import { parseGeoJsonGeometry } from "@/components/db/geo-preview";

/**
 * Reading a table's shape and its cells, for every engine.
 *
 * Pure functions over what `describe` and `query` answer: no fetching, no
 * state, no markup. They were inline in the connection page, which is why that
 * page had grown past two thousand lines.
 */

export function normalizeSchemaNodes(nodes) {
  return (Array.isArray(nodes) ? nodes : [])
    .map((node) => String(node?.name || ""))
    .filter((name) => name && !name.startsWith("_"))
    .sort((a, b) => a.localeCompare(b));
}

/**
 * Editable attributes for one table.
 *
 * Sekejap reports `attributes` in this platform's own shape. The SQL engines
 * report `columns` in theirs, so those are read back into attributes — carrying
 * whether each column is indexed, because saving without that would silently
 * drop every index the table has.
 */
export function attributesFromNode(meta) {
  if (Array.isArray(meta?.attributes) && meta.attributes.length) {
    return meta.attributes;
  }
  const columns = Array.isArray(meta?.columns) ? meta.columns : [];
  return columns
    .filter((col) => col?.pk !== true)
    .map((col) => ({
      name: String(col?.name || ""),
      // The engine's own type name, which is what the picker lists and what
      // the writer takes back. Arguments are dropped for the picker and kept
      // in `full_type` for display.
      kind: String(col?.type ?? col?.data_type ?? "").split("(")[0].trim(),
      full_type: String(col?.full_type ?? col?.type ?? ""),
      index_types: col?.indexed === true ? ["hash"] : [],
    }))
    .filter((attr) => attr.name);
}

/**
 * The facets of a table, in the order a database tool lists them.
 *
 * `ready` marks what is wired. The rest are shown disabled rather than hidden,
 * because a screen that quietly omits constraints and triggers reads as though
 * the table has none.
 */
/**
 * One typed cell as JSON.
 *
 * A grid cell is text, but a database column is typed and checks what it is
 * given: a number bound as text is refused by an integer column. Anything that
 * reads as a number is sent as one, and an empty cell as null.
 */
/**
 * The part of a driver error worth showing.
 *
 * Every engine prefixes its message with its own plumbing; the sentence after
 * it is the one that names the column and the constraint.
 */
export function readableDbError(error) {
  const text = String(error?.message || error || "");
  return text.replace(/^error returned from database:\s*/i, "");
}

export function jsonValueForCell(raw) {
  if (raw === null || raw === undefined) return null;
  const text = String(raw).trim();
  if (text === "") return null;
  if (text === "true") return true;
  if (text === "false") return false;
  if (/^-?\d+(\.\d+)?$/.test(text)) {
    const num = Number(text);
    if (Number.isFinite(num)) return num;
  }
  return String(raw);
}

/** Marks a row the author is still composing, before it reaches the database. */
export const DRAFT_ROW_PREFIX = "__draft:";


export const PROPERTY_SECTIONS = [
  { id: "columns", label: "Columns", glyph: "\u25a4", ready: true },
  { id: "constraints", label: "Constraints", glyph: "\u25c9", ready: false },
  { id: "foreign_keys", label: "Foreign Keys", glyph: "\u2192", ready: false },
  { id: "indexes", label: "Indexes", glyph: "\u2261", ready: false },
  { id: "triggers", label: "Triggers", glyph: "\u26a1", ready: false },
  { id: "permissions", label: "Permissions", glyph: "\u26bf", ready: false },
  { id: "statistics", label: "Statistics", glyph: "\u2211", ready: false },
  { id: "ddl", label: "DDL", glyph: "\u2328", ready: false },
];

export function normalizeTableNodes(nodes) {
  return (Array.isArray(nodes) ? nodes : [])
    .filter((node) => String(node?.kind || "") === "table")
    .map((node) => {
      const schema = String(node?.schema || "default");
      const table = String(node?.name || "");
      const key = schema === "default" ? table : `${schema}.${table}`;
      return {
        schema,
        table,
        key,
        rowCount: Number(node?.meta?.row_count || 0),
        attributes: attributesFromNode(node?.meta),
        columns: Array.isArray(node?.meta?.columns) ? node.meta.columns : [],
        hashIndexed: Array.isArray(node?.meta?.hash_indexed_fields) ? node.meta.hash_indexed_fields : [],
        rangeIndexed: Array.isArray(node?.meta?.range_indexed_fields) ? node.meta.range_indexed_fields : [],
        fulltextFields: Array.isArray(node?.meta?.fulltext_fields) ? node.meta.fulltext_fields : [],
        vectorFields: Array.isArray(node?.meta?.vector_fields) ? node.meta.vector_fields : [],
        spatialFields: Array.isArray(node?.meta?.spatial_fields) ? node.meta.spatial_fields : [],
      };
    })
    .filter((item) => item.schema && item.table && !item.schema.startsWith("_"))
    .sort((a, b) => a.key.localeCompare(b.key));
}

export function sqlStringLiteral(value) {
  return String(value || "").replace(/'/g, "''");
}

export function mapRowToObject(columns, row) {
  const output = {};
  (Array.isArray(columns) ? columns : []).forEach((column, index) => {
    output[String(column || `column_${index + 1}`)] = Array.isArray(row) ? row[index] : undefined;
  });
  return output;
}

export function orderedDataColumns(columns) {
  const trailing = ["_created_unix", "_updated_unix"];
  const seen = new Set();
  const unique = (columns || []).map(String).filter((name) => {
    if (!name || seen.has(name)) return false;
    seen.add(name);
    return true;
  });
  return [
    ...unique.filter((name) => !trailing.includes(name)),
    ...trailing.filter((name) => unique.includes(name)),
  ];
}

export function reorderRowsForColumns(sourceColumns, rows, targetColumns) {
  const indexByName = new Map((sourceColumns || []).map((name, index) => [String(name), index]));
  return (rows || []).map((row) =>
    targetColumns.map((name) => {
      const sourceIndex = indexByName.get(name);
      return Array.isArray(row) && sourceIndex !== undefined ? row[sourceIndex] : null;
    })
  );
}

export function fieldKindForColumn(table, colName) {
  const name = String(colName || "");
  const attr = (table?.attributes || []).find((item) => String(item?.name || "") === name);
  if (attr?.kind) return String(attr.kind);
  if ((table?.spatialFields || []).includes(name)) return "geo";
  if ((table?.vectorFields || []).includes(name)) return "vector";
  return "";
}

export function validateCellEditValue(table, colName, value) {
  const kind = fieldKindForColumn(table, colName);
  const text = String(value ?? "").trim();
  if (!text) return null;
  if (kind === "geo") {
    const geo = parseGeoJsonGeometry(text);
    if (!geo.ok) {
      return {
        title: geo.title,
        message: `${geo.message} Column: ${colName}. Coordinates use [longitude, latitude].`,
        example: geo.example,
      };
    }
  }
  if (kind === "number" && Number.isNaN(Number(text))) {
    return {
      title: "Invalid Number",
      message: `Column ${colName} expects a number. Use plain numeric values such as 12, 12.5, or -3.`,
      example: "12.5",
    };
  }
  if (kind === "json") {
    try {
      JSON.parse(text);
    } catch (_) {
      return {
        title: "Invalid JSON",
        message: `Column ${colName} expects valid JSON.`,
        example: '{"status":"active","tags":["demo"]}',
      };
    }
  }
  if (kind === "vector") {
    try {
      const parsed = JSON.parse(text);
      if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === "number" && Number.isFinite(item))) {
        throw new Error("vector must be numeric array");
      }
    } catch (_) {
      return {
        title: "Invalid Vector",
        message: `Column ${colName} expects a JSON array of numbers.`,
        example: "[0.12, 0.34, 0.56]",
      };
    }
  }
  return null;
}

export function groupTablesBySchema(tables) {
  const map = new Map();
  (tables || []).forEach((item) => {
    if (!map.has(item.schema)) {
      map.set(item.schema, []);
    }
    map.get(item.schema).push(item);
  });
  return map;
}

export function selectedTableDefinition(tables, selectedTable) {
  return (tables || []).find((item) => item.key === selectedTable) || null;
}

/**
 * The grid's shape for one table: the preview's own columns plus any declared
 * column the preview did not return, in the order the reader expects.
 *
 * Pure, and deliberately unaware of draft rows — the editor concatenates its
 * own drafts onto this, which is what keeps the old merge → drafts → merge
 * circle from forming again.
 */
export function computeMergedGrid(activeTable, previewColumns, previewRows) {
  const columns = Array.isArray(previewColumns) ? previewColumns : [];
  const rows = Array.isArray(previewRows) ? previewRows : [];
  const declared = (activeTable?.attributes || []).map((a) => String(a.name || "")).filter(Boolean);
  const extra = declared.filter((name) => !columns.includes(name));

  const rawColumns = [...columns, ...extra];
  const rawRows = extra.length
    ? rows.map((row) => [...(Array.isArray(row) ? row : []), ...extra.map(() => null)])
    : rows;

  const mergedColumns = orderedDataColumns(rawColumns);
  const mergedRows = reorderRowsForColumns(rawColumns, rawRows, mergedColumns);

  // What the engine reports about each column, keyed by name.
  const columnMeta = {};
  for (const col of activeTable?.columns || []) {
    if (col?.name) columnMeta[col.name] = col;
  }

  return { mergedColumns, mergedRows, columnMeta };
}

/** The grid as CSV text, quoting only the cells that need it. */
export function gridAsCsv(columns, rows) {
  const esc = (value) => {
    const text = String(value ?? "");
    return text.includes(",") || text.includes('"') || text.includes("\n")
      ? `"${text.replace(/"/g, '""')}"`
      : text;
  };
  const header = (columns || []).map(esc).join(",");
  const body = (rows || [])
    .map((row) =>
      (Array.isArray(row) ? row : [])
        .map((cell) => esc(typeof cell === "object" ? JSON.stringify(cell) : cell))
        .join(","),
    )
    .join("\n");
  return `${header}\n${body}`;
}
