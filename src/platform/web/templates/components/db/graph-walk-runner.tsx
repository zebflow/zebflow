import { useEffect, useState, cx } from "zeb/react";
import Button from "@/components/ui/button";
import Textarea from "@/components/ui/textarea";
import { StudioTable, StudioTd, StudioTh, StudioThead } from "@/components/ui/studio-data-table";
import { stringifyCell } from "@/components/db/cell-format";

/**
 * The walks offered for one edge type: pick one, change it if you like, run
 * it, read the rows. Owns the statement being edited and its answer; a new
 * set of walks — another arrow, another start row — puts the first one back.
 */
export default function GraphWalkRunner({ walks, runDbQuery }) {
  const [chosen, setChosen] = useState("");
  const [sql, setSql] = useState("");
  const [result, setResult] = useState(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const signature = walks.map((walk) => walk.sql).join("\n");

  useEffect(() => {
    const first = walks.find((walk) => walk.id === chosen) || walks[0];
    setChosen(first?.id || "");
    setSql(first?.sql || "");
    setResult(null);
    setStatus("");
  }, [signature]);

  async function run() {
    if (!sql.trim()) return;
    setBusy(true);
    setStatus("Running…");
    try {
      const answer = await runDbQuery(sql, { readOnly: true, limit: 200 });
      setResult(answer);
      setStatus(`${answer.rows.length} ${answer.rows.length === 1 ? "row" : "rows"}`);
    } catch (error) {
      setResult(null);
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex min-h-0 flex-col gap-3">
      <div className="flex flex-wrap gap-1.5">
        {walks.map((walk) => (
          <button
            key={walk.id}
            type="button"
            onClick={() => {
              setChosen(walk.id);
              setSql(walk.sql);
              setResult(null);
              setStatus("");
            }}
            className={cx(
              "rounded-md border px-2 py-1 text-xs transition-colors",
              walk.id === chosen ? "border-primary text-foreground" : "border-border text-muted-foreground hover:text-foreground",
            )}
          >
            {walk.title}
          </button>
        ))}
      </div>

      <Textarea className="font-mono text-xs" value={sql} rows={5} onInput={(event) => setSql(event?.target?.value || "")} />

      <div className="flex items-center gap-3">
        <Button type="button" size="sm" onClick={run} disabled={busy || !sql.trim()}>
          {busy ? "Running…" : "Run"}
        </Button>
        {status ? (
          <span className={cx("text-xs", status.startsWith("Error") ? "text-danger" : "text-muted-foreground")}>{status}</span>
        ) : null}
      </div>

      {result && result.rows.length ? (
        <div className="max-h-72 overflow-auto rounded-md border border-border/70">
          <StudioTable>
            <StudioThead>
              <tr>
                {result.columns.map((column, index) => <StudioTh key={`${column}-${index}`}>{column}</StudioTh>)}
              </tr>
            </StudioThead>
            <tbody>
              {result.rows.map((row, index) => (
                <tr key={index}>
                  {row.map((cell, at) => <StudioTd key={at}>{stringifyCell(cell)}</StudioTd>)}
                </tr>
              ))}
            </tbody>
          </StudioTable>
        </div>
      ) : null}
    </div>
  );
}
