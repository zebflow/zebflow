import { requestJson } from "@/components/lib/http";
import { mapRowToObject } from "@/components/db/table-data";

/**
 * One way to ask the connected engine a question.
 *
 * Answers rows twice — as the engine's positional arrays and as objects keyed
 * by column — because the grid wants the first and every caller that reads a
 * named field wants the second.
 */
export function useDbQuery(queryUrl) {
  return async function runDbQuery(sql, { readOnly = true, tableName = "", limit = 500 } = {}) {
    if (!queryUrl) {
      throw new Error("Query endpoint is not available");
    }
    const payload = await requestJson(queryUrl, {
      method: "POST",
      body: JSON.stringify({
        sql,
        read_only: readOnly,
        limit,
        ...(tableName ? { table: tableName } : {}),
      }),
    });
    const result = payload?.result || {};
    const columns = Array.isArray(result?.columns)
      ? result.columns.map((item) => String(item?.name || ""))
      : [];
    const rows = Array.isArray(result?.rows) ? result.rows : [];
    return { columns, rows, objects: rows.map((row) => mapRowToObject(columns, row)), result };
  };
}
