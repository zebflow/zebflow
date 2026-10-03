import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";

type Row = [string, string];
type PairMap = Record<string, string | string[]>;

/** One row per value: a key given several times (two `Set-Cookie` headers)
 *  is stored as a list, and each of its values is a row of its own. */
function rowsFromMap(value: unknown): Row[] {
  if (!value || typeof value !== "object" || Array.isArray(value)) return [];
  const rows: Row[] = [];
  for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
    const values = Array.isArray(item) ? item : [item];
    for (const one of values) rows.push([key, one == null ? "" : String(one)]);
  }
  return rows;
}

/** Back to the stored shape: one value is a string, several are a list in
 *  row order — so a node with two `Set-Cookie` headers saves both unchanged. */
function mapFromRows(rows: Row[]): PairMap {
  const map: PairMap = {};
  for (const [key, val] of rows) {
    const existing = map[key];
    if (existing === undefined) map[key] = val;
    else if (Array.isArray(existing)) existing.push(val);
    else map[key] = [existing, val];
  }
  return map;
}

export default function NodeFieldKeyValuePairs({ field, value, onChange }) {
  const rows = rowsFromMap(value);

  function updateRow(idx: number, newKey: string, newVal: string) {
    const next = rows.map((row, i): Row => (i === idx ? [newKey, newVal] : row));
    onChange(mapFromRows(next));
  }

  function removeRow(idx: number) {
    onChange(mapFromRows(rows.filter((_, i) => i !== idx)));
  }

  function addRow() {
    onChange(mapFromRows([...rows, ["", ""]]));
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-col gap-1.5">
        {rows.map(([k, v], idx) => (
          <div key={idx} className="flex gap-1.5 items-center">
            <Input
              type="text"
              value={k}
              placeholder="key"
              onInput={(e) => updateRow(idx, e.currentTarget.value, v)}
            />
            <Input
              type="text"
              value={v}
              placeholder="value"
              onInput={(e) => updateRow(idx, k, e.currentTarget.value)}
            />
            <Button
              variant="ghost"
              size="xs"
              onClick={() => removeRow(idx)}
              title="Remove"
            >
              ×
            </Button>
          </div>
        ))}
        <div>
          <Button variant="outline" size="xs" onClick={addRow}>
            + Add
          </Button>
        </div>
      </div>
    </Field>
  );
}
