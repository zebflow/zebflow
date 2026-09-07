import { useEffect, useState } from "zeb/react";
import { sqlStringLiteral } from "@/components/db/table-data";
import {
  relationNodeLabel,
  relationNodeSlug,
  uniqueRelationDefs,
} from "@/components/db/relations-graph";

/**
 * What one row is related to, in both directions.
 *
 * Asked per selected row rather than per table, so it reloads whenever the
 * reader picks a different row.
 */
export function useNodeRelations({ runDbQuery, enabled, tableName, record, reloadToken, onTypeOptions }) {
  const [outgoing, setOutgoing] = useState([]);
  const [incoming, setIncoming] = useState([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

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
    const edgeDefs = uniqueRelationDefs(show.objects);
    onTypeOptions(
      edgeDefs
        .map((item) => String(item?.type || "").trim())
        .filter(Boolean)
        .filter((item, index, arr) => arr.indexOf(item) === index)
        .sort((a, b) => a.localeCompare(b))
    );

    const outgoingTypes = edgeDefs
      .filter((item) => String(item?.from || "") === tableName)
      .map((item) => String(item?.type || "").trim())
      .filter(Boolean)
      .filter((item, index, arr) => arr.indexOf(item) === index);

    const incomingEdges = edgeDefs
      .filter((item) => String(item?.to || "") === tableName)
      .map((item) => ({
        from: String(item?.from || "").trim(),
        type: String(item?.type || "").trim(),
      }))
      .filter((item) => item.from && item.type)
      .filter((item, index, arr) => arr.findIndex((other) => other.from === item.from && other.type === item.type) === index);

    const escapedKey = sqlStringLiteral(nodeKey);

    const outgoingLists = await Promise.all(
      outgoingTypes.map(async (type) => {
        const query = `SELECT b._collection AS _collection, b._key AS _key, b.title AS title, b.name AS name, b.post_id AS post_id, b.slug AS slug FROM MATCH (a:${tableName})-[:${type}]->(b) WHERE a._key = '${escapedKey}'`;
        const response = await runDbQuery(query, { readOnly: true, tableName, limit: 200 });
        return response.objects.map((target) => ({
          direction: "outgoing",
          type,
          other: target,
          otherSlug: relationNodeSlug(target, ""),
          otherLabel: relationNodeLabel(target, ""),
        }));
      })
    );

    const incomingLists = await Promise.all(
      incomingEdges.map(async ({ from, type }) => {
        const query = `SELECT a._collection AS _collection, a._key AS _key, a.title AS title, a.name AS name, a.post_id AS post_id, a.slug AS slug FROM MATCH (a:${from})-[:${type}]->(b:${tableName}) WHERE b._key = '${escapedKey}'`;
        const response = await runDbQuery(query, { readOnly: true, tableName, limit: 200 });
        return response.objects.map((source) => ({
          direction: "incoming",
          type,
          other: source,
          otherSlug: relationNodeSlug(source, ""),
          otherLabel: relationNodeLabel(source, ""),
        }));
      })
    );

    setOutgoing(
      outgoingLists
        .flat()
        .filter((item) => item.otherSlug)
        .sort((a, b) => `${a.type}:${a.otherLabel}`.localeCompare(`${b.type}:${b.otherLabel}`))
    );
    setIncoming(
      incomingLists
        .flat()
        .filter((item) => item.otherSlug)
        .sort((a, b) => `${a.type}:${a.otherLabel}`.localeCompare(`${b.type}:${b.otherLabel}`))
    );
  } catch (error) {
    setOutgoing([]);
    setIncoming([]);
    setError(String(error?.message || error));
  } finally {
    setBusy(false);
  }
}

  /** Removing one edge, then re-reading what is left. */
  async function deleteRelation(entry) {
    if (!entry || !tableName || !record) return;
    const currentSlug = relationNodeSlug(record, tableName);
    if (!currentSlug) return;
    const fromSlug = entry.direction === "outgoing" ? currentSlug : entry.otherSlug;
    const toSlug = entry.direction === "outgoing" ? entry.otherSlug : currentSlug;
    try {
      await runDbQuery(
        `DELETE ('${sqlStringLiteral(fromSlug)}')-[:${entry.type}]->('${sqlStringLiteral(toSlug)}')`,
        { readOnly: false, tableName, limit: 50 },
      );
      await load(tableName, record);
    } catch (error) {
      setError(String(error?.message || error));
    }
  }

  useEffect(() => {
    load(tableName, record);
  }, [tableName, record?._key, reloadToken]);

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
