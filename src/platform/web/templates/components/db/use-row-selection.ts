import { useEffect, useState } from "zeb/react";
import { mapRowToObject } from "@/components/db/table-data";

/**
 * The row the inspector is describing.
 *
 * Re-chosen whenever the rows change: the same row if it is still there, the
 * first one otherwise, so the panel beside the grid is never left describing a
 * row that no longer exists.
 */
export function useRowSelection({ activeTable, columns, rows, rowIdentity }) {
  const [key, setKey] = useState("");
  const [data, setData] = useState(null);

  useEffect(() => {
    if (!activeTable || !rows.length) {
      setKey("");
      setData(null);
      return;
    }
    const records = rows.map((row) => mapRowToObject(columns, row));
    const chosen =
      records.find((record) => String(record?.[rowIdentity] ?? "") === key) || records[0] || null;
    if (!chosen) {
      setKey("");
      setData(null);
      return;
    }
    setKey(String(chosen?.[rowIdentity] ?? ""));
    setData(chosen);
  }, [activeTable, columns, rows, key]);

  return { key, setKey, data, setData };
}
