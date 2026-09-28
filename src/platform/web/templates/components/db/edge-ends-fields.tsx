import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";

export const DEFAULT_EDGE = {
  enabled: false,
  source_table: "",
  source: "",
  destination_table: "",
  destination: "",
  label: "",
  one_per_pair: true,
};

/** The column name an end gets unless the reader types one: `members_id`. */
function endColumn(table, taken) {
  const bare = String(table || "").split(".").pop();
  const name = bare ? `${bare}_id` : "";
  return name && name === taken ? `to_${name}` : name;
}

/**
 * The two ends of a new edge table: which table each end reaches and the
 * column holding its key, the label its edges carry, and whether a pair may
 * be joined only once. The tables offered are tables of rows; an edge never
 * reaches another edge table.
 */
export default function EdgeEndsFields({ edge, setEdge, tables, busy }) {
  const rowTables = (tables || []).filter((table) => !table.edge);
  const pick = (end, table) => {
    const column = end === "source" ? "source" : "destination";
    const other = end === "source" ? edge.destination : edge.source;
    setEdge({ ...edge, [`${column}_table`]: table, [column]: edge[column] || endColumn(table, other) });
  };

  return (
    <div className="grid gap-3 rounded-lg border border-border/70 p-3 sm:grid-cols-2">
      {["source", "destination"].map((end) => (
        <div key={end} className="flex flex-col gap-2">
          <Field label={end === "source" ? "From table" : "To table"}>
            <Select value={edge[`${end}_table`]} onChange={(event) => pick(end, event?.target?.value || "")} disabled={busy}>
              <SelectOption value="" label="—" />
              {rowTables.map((table) => (
                <SelectOption key={table.key} value={table.graphName} label={table.graphName} />
              ))}
            </Select>
          </Field>
          <Field label={end === "source" ? "From column" : "To column"}>
            <Input
              className="h-8 text-xs"
              value={edge[end]}
              placeholder="holds the row's key"
              onInput={(event) => setEdge({ ...edge, [end]: event?.target?.value || "" })}
              disabled={busy}
            />
          </Field>
        </div>
      ))}
      <Field label="Label">
        <Input
          className="h-8 text-xs"
          value={edge.label}
          placeholder="the table's name"
          onInput={(event) => setEdge({ ...edge, label: event?.target?.value || "" })}
          disabled={busy}
        />
      </Field>
      <label className="flex items-center gap-2 self-end pb-2 text-xs text-muted-foreground">
        <input
          type="checkbox"
          checked={edge.one_per_pair}
          onChange={(event) => setEdge({ ...edge, one_per_pair: event?.target?.checked === true })}
          disabled={busy}
        />
        One edge per pair of rows
      </label>
    </div>
  );
}
