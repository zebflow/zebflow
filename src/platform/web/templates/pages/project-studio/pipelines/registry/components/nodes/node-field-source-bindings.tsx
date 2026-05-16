import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

type SourceBindingRow = {
  source: string;
  alias: string;
};

function slugifyAlias(raw: string, fallback = "source"): string {
  const out = String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/\.[a-z0-9]+$/i, "")
    .replace(/^\$input\.?/, "")
    .replace(/^\$nodes\./, "")
    .split("/")
    .filter(Boolean)
    .pop();
  const normalized = String(out || "")
    .replace(/[^a-z0-9_]+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_|_$/g, "");
  const safe = normalized && !/^[0-9]/.test(normalized) ? normalized : fallback;
  return safe || fallback;
}

function parseLegacyBinding(value: string, index: number): SourceBindingRow {
  const raw = String(value || "").trim();
  if (!raw) return { source: "", alias: "" };
  const match = raw.match(/^(.*)\s+as\s+([A-Za-z_][A-Za-z0-9_]*)$/i);
  if (match) {
    return {
      source: match[1].trim(),
      alias: match[2].trim(),
    };
  }
  return {
    source: raw,
    alias: slugifyAlias(raw, `source_${index + 1}`),
  };
}

function normalizeValue(value: unknown): SourceBindingRow[] {
  if (!Array.isArray(value)) return [];
  return value.map((item, index) => {
    if (typeof item === "string") return parseLegacyBinding(item, index);
    const source = item && typeof item === "object" ? item as Record<string, unknown> : {};
    const sourceValue = String(source.source || "").trim();
    const explicitAlias = String(source.alias || "").trim();
    const alias = explicitAlias || (sourceValue ? slugifyAlias(sourceValue, `source_${index + 1}`) : "");
    return { source: sourceValue, alias };
  });
}

function validAlias(alias: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(alias);
}

export default function NodeFieldSourceBindings({ field, value, onChange }) {
  const rows = normalizeValue(value);
  const aliases = rows.map((row) => row.alias).filter(Boolean);
  const duplicates = new Set(aliases.filter((alias, index) => aliases.indexOf(alias) !== index));

  function emit(next: SourceBindingRow[]) {
    onChange(next.map((row) => ({
      source: row.source,
      alias: row.alias,
    })));
  }

  function updateRow(index: number, patch: Partial<SourceBindingRow>) {
    const next = rows.map((row, i) => {
      if (i !== index) return row;
      const updated = { ...row, ...patch };
      if (patch.source !== undefined && (!row.alias || row.alias === slugifyAlias(row.source, `source_${index + 1}`))) {
        updated.alias = slugifyAlias(patch.source, `source_${index + 1}`);
      }
      return updated;
    });
    emit(next);
  }

  function moveRow(index: number, offset: number) {
    const target = index + offset;
    if (target < 0 || target >= rows.length) return;
    const next = rows.slice();
    const [item] = next.splice(index, 1);
    next.splice(target, 0, item);
    emit(next);
  }

  function removeRow(index: number) {
    emit(rows.filter((_row, i) => i !== index));
  }

  function addRow() {
    emit([...rows, { source: "", alias: "" }]);
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-col gap-2">
        {rows.length > 0 ? (
          <div className="rounded border border-dark-border overflow-hidden">
            <div
              className="grid gap-1.5 px-2 py-1 bg-dark-accent3/40 border-b border-dark-border"
              style={{ gridTemplateColumns: "minmax(0, 1.7fr) minmax(8rem, 0.8fr) 4.75rem" }}
            >
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Source</span>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Alias</span>
              <span />
            </div>
            {rows.map((row, index) => {
              const aliasInvalid = !!row.alias && (!validAlias(row.alias) || duplicates.has(row.alias));
              return (
                <div
                  key={index}
                  className="grid items-center gap-1.5 px-2 py-1.5 border-b border-dark-border last:border-0"
                  style={{ gridTemplateColumns: "minmax(0, 1.7fr) minmax(8rem, 0.8fr) 4.75rem" }}
                >
                  <Input
                    value={row.source}
                    placeholder="datasets/posts.parquet or $input.rows"
                    onInput={(e) => updateRow(index, { source: e.currentTarget.value })}
                  />
                  <Input
                    value={row.alias}
                    placeholder="posts"
                    className={aliasInvalid ? "border-red-500/80 focus:ring-red-500/70" : ""}
                    onInput={(e) => updateRow(index, { alias: e.currentTarget.value })}
                  />
                  <div className="flex items-center justify-end gap-1">
                    <button type="button" title="Move up" onClick={() => moveRow(index, -1)} disabled={index === 0} className="w-6 h-6 rounded text-body-soft hover:text-body hover:bg-dark-accent3 disabled:opacity-30">↑</button>
                    <button type="button" title="Move down" onClick={() => moveRow(index, 1)} disabled={index === rows.length - 1} className="w-6 h-6 rounded text-body-soft hover:text-body hover:bg-dark-accent3 disabled:opacity-30">↓</button>
                    <button type="button" title="Remove" onClick={() => removeRow(index)} className="w-6 h-6 rounded text-body-soft hover:text-red-400 hover:bg-dark-accent3">×</button>
                  </div>
                </div>
              );
            })}
          </div>
        ) : null}

        <div>
          <button
            type="button"
            onClick={addRow}
            className="rounded border border-dark-border bg-dark-accent3 px-2.5 py-1.5 text-xs text-body hover:bg-dark-accent2 transition-colors"
          >
            + Add source
          </button>
        </div>

        {duplicates.size > 0 ? (
          <div className="text-xs text-red-400">Aliases must be unique.</div>
        ) : null}
      </div>
    </Field>
  );
}
