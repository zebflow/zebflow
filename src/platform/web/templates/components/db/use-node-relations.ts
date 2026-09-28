import { useEffect, useState } from "zeb/react";
import { uniqueRelationDefs } from "@/components/db/relations-graph";
import {
  edgeDeleteSql,
  edgeTableFor,
  labelColumn,
  relationWalkSql,
  tableByKey,
} from "@/components/db/edge-tables";

/**
 * What one row is related to, in both directions.
 *
 * Asked per selected row rather than per table, so it reloads whenever the
 * reader picks a different row. Each edge type is walked with GQL, and the
 * far row is named by a column its table really has. An edge type backed by
 * an edge table can be removed here; a loose edge is shown and not offered.
 */
export function useNodeRelations({ runDbQuery, enabled, tableName, tables, record, reloadToken, onTypeOptions }) {
  const [outgoing, setOutgoing] = useState([]);
  const [incoming, setIncoming] = useState([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function walk(def, key, outgoingSide) {
    const farTable = tableByKey(tables, outgoingSide ? def.to : def.from);
    const sql = relationWalkSql({
      from: def.from,
      type: def.type,
      to: def.to,
      key,
      outgoing: outgoingSide,
      farLabel: labelColumn(farTable),
    });
    const response = await runDbQuery(sql, { readOnly: true, tableName, limit: 200 });
    const edgeTable = edgeTableFor(tables, def.type);
    const farCollection = outgoingSide ? def.to : def.from;
    return (response.objects || []).map((row) => ({
      direction: outgoingSide ? "outgoing" : "incoming",
      type: def.type,
      other: { ...row, _collection: farCollection },
      otherSlug: `${farCollection}/${row._key}`,
      otherLabel: String(row._label || row._key || ""),
      otherKey: String(row._key || ""),
      edgeTable,
      loose: !edgeTable,
    }));
  }

  async function load(tableName, record) {
    const nodeKey = String(record?._key || "").trim();
    if (!enabled || !tableName || !nodeKey) {
      setOutgoing([]);
      setIncoming([]);
      setError("");
      return;
    }

    setBusy(true);
    setError("");
    try {
      const show = await runDbQuery("SHOW EDGES", { readOnly: true, tableName, limit: 500 });
      const defs = uniqueRelationDefs(show.objects);
      onTypeOptions(
        defs.map((def) => def.type).filter((type, index, all) => all.indexOf(type) === index).sort(),
      );
      const [outLists, inLists] = await Promise.all([
        Promise.all(defs.filter((def) => def.from === tableName).map((def) => walk(def, nodeKey, true))),
        Promise.all(defs.filter((def) => def.to === tableName).map((def) => walk(def, nodeKey, false))),
      ]);
      const byTypeAndLabel = (a, b) => `${a.type}:${a.otherLabel}`.localeCompare(`${b.type}:${b.otherLabel}`);
      setOutgoing(outLists.flat().filter((item) => item.otherKey).sort(byTypeAndLabel));
      setIncoming(inLists.flat().filter((item) => item.otherKey).sort(byTypeAndLabel));
    } catch (error) {
      setOutgoing([]);
      setIncoming([]);
      setError(String(error?.message || error));
    } finally {
      setBusy(false);
    }
  }

  /**
   * Removing one edge from its edge table, then re-reading what is left. A
   * loose edge has no table to delete from and is not offered.
   */
  async function deleteRelation(entry) {
    if (!entry?.edgeTable || !tableName || !record) return;
    const currentKey = String(record?._key || "").trim();
    const [sourceKey, destinationKey] = entry.direction === "outgoing"
      ? [currentKey, entry.otherKey]
      : [entry.otherKey, currentKey];
    try {
      await runDbQuery(edgeDeleteSql(entry.edgeTable, sourceKey, destinationKey), {
        readOnly: false,
        tableName,
        limit: 0,
      });
      await load(tableName, record);
    } catch (error) {
      setError(String(error?.message || error));
    }
  }

  useEffect(() => {
    load(tableName, record);
  }, [tableName, record?._key, reloadToken, tables?.length]);

  return {
    outgoing,
    incoming,
    busy,
    error,
    deleteRelation,
    /** Re-ask, for the open row unless a different one is named. */
    reload: (t = tableName, r = record) => load(t, r),
  };
}
