import { useState, cx } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import { StudioTable, StudioTd, StudioTh, StudioThead } from "@/components/ui/studio-data-table";
import { stringifyCell } from "@/components/db/cell-format";
import { edgePropertyColumns } from "@/components/db/edge-tables";
import { useEdgeRows } from "@/components/db/use-edge-rows";

/**
 * An edge table's Data tab: its edges as rows, each removable, and a line
 * to add one.
 *
 * An edge has no key of its own to select it by — its two ends are its
 * address — so this is not the row grid, and nothing here edits a cell.
 */
export default function EdgeTablePanel({ edgeTable, runDbQuery }) {
  const edges = useEdgeRows({ edgeTable, runDbQuery });
  const [pendingRemove, setPendingRemove] = useState(null);
  const edge = edgeTable.edge;
  const properties = edgePropertyColumns(edgeTable);
  const columns = [edge.source, edge.destination, ...properties.map((attr) => attr.name)];

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-3 border-b border-border/70 px-3 py-2 text-xs text-muted-foreground">
        <span>
          <span className="text-foreground">{edge.source_table}</span> → <span className="text-foreground">{edge.destination_table}</span>
        </span>
        <span>type {edge.label}</span>
        <span>{edges.count} {edges.count === 1 ? "edge" : "edges"}</span>
        {edges.count > edges.shown ? <span>first {edges.shown} shown</span> : null}
        <Button type="button" variant="ghost" size="sm" className="ml-auto" onClick={edges.reload} disabled={edges.busy}>
          Refresh
        </Button>
      </div>

      <AddEdgeRow edgeTable={edgeTable} busy={edges.busy} onAdd={edges.add} />

      {edges.status ? (
        <p className={cx("px-3 py-1 text-xs", edges.status.startsWith("Error") ? "text-danger" : "text-muted-foreground")}>
          {edges.status}
        </p>
      ) : null}

      <div className="min-h-0 flex-1 overflow-auto">
        {edges.rows.length === 0 ? (
          <div className="db-suite-empty">{edges.busy ? "Loading edges…" : "No edges yet."}</div>
        ) : (
          <StudioTable>
            <StudioThead>
              <tr>
                {columns.map((name) => <StudioTh key={name}>{name}</StudioTh>)}
                <StudioTh />
              </tr>
            </StudioThead>
            <tbody>
              {edges.rows.map((row, index) => (
                <tr key={`${row[edge.source]}-${row[edge.destination]}-${index}`}>
                  {columns.map((name) => <StudioTd key={name}>{stringifyCell(row[name])}</StudioTd>)}
                  <StudioTd className="text-right">
                    <Button type="button" variant="ghost" size="sm" onClick={() => setPendingRemove(row)} disabled={edges.busy}>
                      Delete
                    </Button>
                  </StudioTd>
                </tr>
              ))}
            </tbody>
          </StudioTable>
        )}
      </div>

      <ConfirmDialog
        open={!!pendingRemove}
        onClose={() => setPendingRemove(null)}
        onConfirm={async () => {
          const row = pendingRemove;
          setPendingRemove(null);
          await edges.remove(row);
        }}
        title="Delete edge"
        message={`Delete the edge from ${String(pendingRemove?.[edge.source] ?? "")} to ${String(pendingRemove?.[edge.destination] ?? "")}? The two rows stay.`}
        confirmLabel="Delete"
        variant="destructive"
      />
    </div>
  );
}

/** One line of inputs: the two ends' keys and the edge's properties. */
function AddEdgeRow({ edgeTable, busy, onAdd }) {
  const edge = edgeTable.edge;
  const [source, setSource] = useState("");
  const [destination, setDestination] = useState("");
  const [values, setValues] = useState({});

  async function submit(event) {
    event?.preventDefault?.();
    if (!source.trim() || !destination.trim()) return;
    if (await onAdd(source.trim(), destination.trim(), values)) {
      setSource("");
      setDestination("");
      setValues({});
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-wrap items-center gap-2 border-b border-border/70 px-3 py-2">
      <Input className="h-7 w-40 text-xs" value={source} placeholder={`${edge.source} · ${edge.source_table} key`} onInput={(event) => setSource(event?.target?.value || "")} disabled={busy} />
      <span className="text-xs text-muted-foreground">→</span>
      <Input className="h-7 w-40 text-xs" value={destination} placeholder={`${edge.destination} · ${edge.destination_table} key`} onInput={(event) => setDestination(event?.target?.value || "")} disabled={busy} />
      {edgePropertyColumns(edgeTable).map((attr) => (
        <Input
          key={attr.name}
          className="h-7 w-32 text-xs"
          value={values[attr.name] || ""}
          placeholder={attr.default ? `${attr.name} · default ${attr.default}` : attr.name}
          onInput={(event) => setValues({ ...values, [attr.name]: event?.target?.value || "" })}
          disabled={busy}
        />
      ))}
      <Button type="submit" size="sm" variant="outline" disabled={busy || !source.trim() || !destination.trim()}>
        Add edge
      </Button>
    </form>
  );
}
