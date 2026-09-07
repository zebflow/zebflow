import { useEffect, useState } from "zeb/react";
import { relationCountFromRows, uniqueRelationDefs } from "@/components/db/relations-graph";

/**
 * How many rows of each relation the open table takes part in.
 *
 * A table-level summary, distinct from the relations of one row: this asks the
 * engine what edges exist and counts each, which is a question about the table
 * and not about whatever the reader has selected.
 *
 * The edge types it discovers are reported back through `onTypeOptions`
 * because the row-level hook discovers the same list from the same query, and
 * the create-relation dialog needs whichever ran last.
 */
export function useRelationStats({ runDbQuery, enabled, tableName, reloadToken, onTypeOptions }) {
  const [outgoing, setOutgoing] = useState([]);
  const [incoming, setIncoming] = useState([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

async function load(tableName) {
  if (!enabled || !tableName) {
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
    const allTypes = edgeDefs
      .map((item) => item.type)
      .filter((item, index, arr) => item && arr.indexOf(item) === index)
      .sort((a, b) => a.localeCompare(b));
    onTypeOptions(allTypes);

    const outgoingDefs = edgeDefs.filter((item) => item.from === tableName);
    const incomingDefs = edgeDefs.filter((item) => item.to === tableName);

    async function withCount(def, direction) {
      const query = direction === "outgoing"
        ? `SELECT COUNT(*) AS count FROM MATCH (a:${def.from})-[:${def.type}]->(b:${def.to})`
        : `SELECT COUNT(*) AS count FROM MATCH (a:${def.from})-[:${def.type}]->(b:${def.to})`;
      try {
        const counted = await runDbQuery(query, { readOnly: true, tableName, limit: 1 });
        return { ...def, direction, count: relationCountFromRows(counted.rows), countError: "" };
      } catch (error) {
        return { ...def, direction, count: null, countError: String(error?.message || error) };
      }
    }

    const [outgoing, incoming] = await Promise.all([
      Promise.all(outgoingDefs.map((def) => withCount(def, "outgoing"))),
      Promise.all(incomingDefs.map((def) => withCount(def, "incoming"))),
    ]);
    setOutgoing(outgoing.sort((a, b) => `${a.type}:${a.to}`.localeCompare(`${b.type}:${b.to}`)));
    setIncoming(incoming.sort((a, b) => `${a.type}:${a.from}`.localeCompare(`${b.type}:${b.from}`)));
  } catch (error) {
    setOutgoing([]);
    setIncoming([]);
    setError(String(error?.message || error));
  } finally {
    setBusy(false);
  }
}

  useEffect(() => {
    load(tableName);
  }, [tableName, reloadToken]);

  return {
    outgoing,
    incoming,
    busy,
    error,
    /** Re-ask, for the open table unless a different one is named. */
    reload: (t = tableName) => load(t),
  };
}
