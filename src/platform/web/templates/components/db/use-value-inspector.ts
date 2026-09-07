import { useState } from "zeb/react";
import { prettyValue, rawCellValue } from "@/components/db/cell-format";

/**
 * The one cell the reader is looking at.
 *
 * Both grids write here — the table's and the query result's — so it is held
 * once above them rather than duplicated into each.
 */
export function useValueInspector() {
  const [meta, setMeta] = useState("Click a cell to inspect value");
  const [body, setBody] = useState("");
  const [raw, setRaw] = useState(null);

  function inspectCell(columnName, rowIndex, cellValue) {
    setMeta(`${columnName} · row ${rowIndex + 1}`);
    setBody(prettyValue(rawCellValue(cellValue)));
    setRaw(cellValue);
  }

  /** Back to the resting state, used when the open table changes. */
  function reset() {
    setMeta("Click a cell to inspect value");
    setBody("");
    setRaw(null);
  }

  return { meta, body, raw, inspectCell, reset };
}
