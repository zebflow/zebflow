import { useEffect, useState } from "zeb/react";
import { edgeCountSql, edgeDeleteSql, edgeInsertSql, edgeRowsSql } from "@/components/db/edge-tables";

const PAGE = 200;

/**
 * The edges of one edge table, as rows: loading them, adding one, and
 * removing one.
 *
 * Read through the graph rather than from the table, because an edge table
 * answers a SELECT only when its WHERE names one end. Refilled whenever a
 * different edge table is opened.
 */
export function useEdgeRows({ edgeTable, runDbQuery }) {
  const [rows, setRows] = useState([]);
  const [count, setCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");

  async function load() {
    if (!edgeTable?.edge) return;
    setBusy(true);
    try {
      const [listed, counted] = await Promise.all([
        runDbQuery(edgeRowsSql(edgeTable, PAGE), { readOnly: true, tableName: edgeTable.key, limit: PAGE }),
        runDbQuery(edgeCountSql(edgeTable), { readOnly: true, tableName: edgeTable.key, limit: 1 }),
      ]);
      setRows(listed.objects || []);
      setCount(Number(counted.objects?.[0]?.count || 0));
      setStatus("");
    } catch (error) {
      setRows([]);
      setCount(0);
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    setRows([]);
    setCount(0);
    load();
  }, [edgeTable?.key]);

  /** Runs one write, then reloads; answers whether it was written. */
  async function write(sql, done) {
    setBusy(true);
    try {
      await runDbQuery(sql, { readOnly: false, tableName: edgeTable.key, limit: 0 });
      setStatus(done);
      await load();
      return true;
    } catch (error) {
      setStatus(`Error · ${String(error?.message || error)}`);
      setBusy(false);
      return false;
    }
  }

  const edge = edgeTable?.edge || {};
  return {
    rows,
    count,
    shown: PAGE,
    busy,
    status,
    reload: load,
    add: (source, destination, properties) =>
      write(edgeInsertSql(edgeTable, source, destination, properties), "Edge added"),
    remove: (row) =>
      write(edgeDeleteSql(edgeTable, String(row?.[edge.source] ?? ""), String(row?.[edge.destination] ?? "")), "Edge removed"),
  };
}
