import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import {
  groupTablesBySchema,
  normalizeSchemaNodes,
  normalizeTableNodes,
  selectedTableDefinition,
} from "@/components/db/table-data";

/**
 * Which tables this connection has, and which one is open.
 *
 * Held above every panel rather than inside one: the open table is reflected
 * in the URL and read by the grid, the query box, the properties editor and
 * the relation panels alike. That makes it routing state, not any one panel's
 * property.
 */
export function useTableCatalog({ schemasUrl, tablesUrl, initialTable, qualifySchema }) {
  const [schemas, setSchemas] = useState([]);
  const [tables, setTables] = useState([]);
  const [selectedTable, setSelectedTable] = useState(initialTable || "");
  const [collapsedSchemas, setCollapsedSchemas] = useState({});
  const [treeError, setTreeError] = useState("");
  const [reloadToken, setReloadToken] = useState(0);

  async function load(preferredTable = "") {
    const [schemasPayload, tablesPayload] = await Promise.all([
      requestJson(schemasUrl),
      requestJson(tablesUrl),
    ]);
    const nextSchemas = normalizeSchemaNodes(schemasPayload?.result?.nodes);
    const nextTables = normalizeTableNodes(tablesPayload?.result?.nodes);
    setSchemas(nextSchemas);
    setTables(nextTables);
    setTreeError("");

    const requested = String(preferredTable || initialTable || "").trim();
    const first = nextTables[0]?.key || "";
    setSelectedTable(nextTables.some((item) => item.key === requested) ? requested : first);
  }

  useEffect(() => {
    if (!schemasUrl || !tablesUrl) return;
    let active = true;
    load().catch((error) => {
      if (!active) return;
      setSchemas([]);
      setTables([]);
      setTreeError(`Failed to load tables: ${String(error?.message || error)}`);
    });
    return () => {
      active = false;
    };
  }, [schemasUrl, tablesUrl, reloadToken]);

  const grouped = groupTablesBySchema(tables);
  const schemaNames = (schemas.length ? schemas : Array.from(grouped.keys())).sort((a, b) =>
    a.localeCompare(b),
  );
  const activeTable = selectedTableDefinition(tables, selectedTable);

  // How this engine must be told which table to touch. An engine that
  // namespaces its tables needs the qualifier — `UPDATE orders` fails on
  // PostgreSQL when the table lives in `shop` — and one that does not would
  // choke on it. Decided by the declared capability, not by the engine's name.
  const tableRef = activeTable
    ? qualifySchema
      ? `${activeTable.schema}.${activeTable.table}`
      : activeTable.table
    : "";

  return {
    schemas,
    tables,
    selectedTable,
    setSelectedTable,
    collapsedSchemas,
    toggleSchema: (name) => setCollapsedSchemas((prev) => ({ ...prev, [name]: !prev[name] })),
    treeError,
    grouped,
    schemaNames,
    activeTable,
    tableRef,
    reloadToken,
    /** Re-read the tables now, preferring to leave `preferredTable` open. */
    reload: load,
    /** Ask everything keyed on `reloadToken` to re-read itself. */
    refresh: () => setReloadToken((token) => token + 1),
  };
}
