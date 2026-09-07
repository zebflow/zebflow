import { useState } from "zeb/react";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import Textarea from "@/components/ui/textarea";
import { requestJson } from "@/components/lib/http";
import { displayCellText } from "@/components/db/cell-format";

/**
 * Asking the engine a question directly.
 *
 * Owns the statement and its result and nothing else. The open table is
 * borrowed only to scope the request; clicking a result cell is handed up,
 * because the panel that describes a value lives beside the data grid.
 */
export default function QueryTabPanel({ queryUrl, initialSql, selectedTable, vectorFields, onCellInspect }) {
  const [sql, setSql] = useState(String(initialSql || ""));
  const [status, setStatus] = useState("Ready");
  const [columns, setColumns] = useState([]);
  const [rows, setRows] = useState([]);

  async function run() {
    if (!queryUrl) return;
    const statement = String(sql || "").trim();
    if (!statement) {
      setStatus("Error · Query is empty");
      return;
    }

    setStatus("Running...");
    try {
      const response = await requestJson(queryUrl, {
        method: "POST",
        body: JSON.stringify({
          sql: statement,
          read_only: true,
          limit: 1000,
          ...(selectedTable
            ? { table: selectedTable.split(".").pop() || selectedTable }
            : {}),
        }),
      });
      const result = response?.result || {};
      setColumns(
        Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : [],
      );
      setRows(Array.isArray(result?.rows) ? result.rows : []);
      setStatus(`OK · rows ${Number(result?.row_count || 0)} · ${Number(result?.duration_ms || 0)} ms`);
    } catch (error) {
      setColumns([]);
      setRows([]);
      setStatus(`Error · ${String(error?.message || error)}`);
    }
  }

  return (
    <section className="db-suite-panel db-suite-panel-fill">
      <div className="db-suite-query-split">
        <div className="db-suite-query-top">
          <div className="db-suite-query-toolbar">
            <button type="button" className="project-inline-chip project-inline-chip-action" onClick={run}>
              Run Query
            </button>
            <p className="db-suite-query-status">{status}</p>
          </div>
          <Textarea
            className="db-suite-query-editor-host"
            value={sql}
            onInput={(event) => setSql(event?.target?.value || "")}
            rows={10}
          />
        </div>

        <div className="db-suite-query-bottom">
          <div className="db-suite-grid-wrap">
            <StudioTable variant="dbGrid">
              <StudioThead>
                <tr>
                  {columns.map((col, index) => (
                    <StudioTh key={`qcol-${col}-${index}`}>{col}</StudioTh>
                  ))}
                </tr>
              </StudioThead>
              <tbody>
                {rows.map((row, rowIndex) => (
                  <tr key={`qrow-${rowIndex}`}>
                    {(Array.isArray(row) ? row : []).map((cell, cellIndex) => {
                      const colName = columns[cellIndex] || `column_${cellIndex + 1}`;
                      return (
                        <StudioTd
                          key={`qcell-${rowIndex}-${cellIndex}`}
                          onClick={() => onCellInspect(colName, rowIndex, cell)}
                        >
                          {displayCellText(cell, colName, vectorFields)}
                        </StudioTd>
                      );
                    })}
                  </tr>
                ))}
                {!rows.length ? (
                  <tr>
                    <StudioTd colSpan={Math.max(columns.length, 1)}>No rows available</StudioTd>
                  </tr>
                ) : null}
              </tbody>
            </StudioTable>
          </div>
        </div>
      </div>
    </section>
  );
}
