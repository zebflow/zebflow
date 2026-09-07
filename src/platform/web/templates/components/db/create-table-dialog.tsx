import { useState, cx } from "zeb/react";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";

/**
 * Table creation for engines that allow it.
 *
 * Declared by `capabilities.create_table`. Attribute kinds and index options
 * are data, so an engine widens them without the dialog changing.
 */

/**
 * Index kinds one type family can carry.
 *
 * Keyed by family rather than by type name, so an engine adding `timestamptz`
 * or `int8` needs no change here — the family it declares decides.
 */
const INDEX_OPTIONS_BY_FAMILY = {
  text: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  number: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  date_time: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  uuid: [{ id: "hash", label: "Exact" }],
  boolean: [{ id: "hash", label: "Exact" }],
  json: [],
  binary: [],
  geometry: [{ id: "spatial", label: "Spatial" }],
  vector: [{ id: "vector", label: "Vector" }],
  other: [],
};

/** The mark shown beside a column name, the way a database tool does it. */
const FAMILY_GLYPH = {
  text: "A",
  number: "#",
  boolean: "\u2713",
  json: "{}",
  date_time: "\u25f4",
  uuid: "\u2687",
  binary: "\u25a6",
  geometry: "\u25c8",
  vector: "\u2234",
  other: "\u00b7",
};

export function familyOfType(types, name) {
  const match = (types || []).find(
    (entry) => String(entry?.name || "").toLowerCase() === String(name || "").split("(")[0].trim().toLowerCase(),
  );
  return String(match?.family || "other");
}

export function glyphForFamily(family) {
  return FAMILY_GLYPH[family] || FAMILY_GLYPH.other;
}

export const DEFAULT_ATTRIBUTE = {
  name: "",
  kind: "",
  index_types: [],
};

/** The header the attribute rows line up under. */
export function AttributeEditorHeader({ compact = false }) {
  return (
    <div
      className={cx(
        "grid items-center gap-2 border-b border-ui-border/70 px-2 pb-1 text-[10px] font-medium uppercase tracking-[0.12em] text-ui-text-muted",
        compact
          ? "grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_3.5rem_minmax(0,1fr)_minmax(0,10rem)_1.75rem]"
          : "grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_3.5rem_minmax(0,1fr)_minmax(0,10rem)_1.75rem]",
      )}
    >
      <span>Column</span>
      <span>Data type</span>
      <span title="Not null — not yet applied by the writer">Not null</span>
      <span title="Default — not yet applied by the writer">Default</span>
      <span>Index</span>
      <span />
    </div>
  );
}

/**
 * One column, laid out as a row rather than a card so a table with twenty of
 * them stays readable without scrolling past each one.
 *
 * `Not null` and `Default` are shown because a column has them and hiding them
 * would misrepresent the table — but they are disabled, because nothing writes
 * them yet. A disabled control that says so is honest; an editable one that
 * silently drops the value would not be.
 */
export function AttributeEditorRow({ item, types, onChange, onRemove }) {
  const catalog = Array.isArray(types) ? types : [];
  const family = familyOfType(catalog, item?.kind);
  const options = INDEX_OPTIONS_BY_FAMILY[family] || [];

  return (
    <div className="grid min-w-0 items-center gap-2 border-b border-ui-border/40 px-2 py-1.5 grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_3.5rem_minmax(0,1fr)_minmax(0,10rem)_1.75rem]">
      <div className="flex min-w-0 items-center gap-1.5">
        <span className="w-4 shrink-0 text-center text-[11px] text-ui-text-muted" title={family}>
          {glyphForFamily(family)}
        </span>
        <Input
          className="min-w-0 h-7 text-xs"
          value={item?.name || ""}
          onInput={(event) => onChange({ ...item, name: event?.target?.value || "" })}
          placeholder="column_name"
        />
      </div>

      <Select
        className="min-w-0 h-7 text-xs"
        title={item?.full_type || item?.kind || ""}
        value={item?.kind || ""}
        onChange={(event) => onChange({ ...item, kind: event?.target?.value || "", index_types: [] })}
      >
        {catalog.length === 0 ? <SelectOption value="" label="—" /> : null}
        {/* The name alone, as the engine writes it. The note explains it on
            hover rather than crowding a column that has to stay narrow. */}
        {catalog.map((entry) => (
          <SelectOption key={entry.name} value={entry.name} label={entry.name} />
        ))}
      </Select>

      <label className="flex items-center justify-center" title="Not yet applied by the writer">
        <input type="checkbox" disabled className="opacity-40" />
      </label>

      <Input
        className="min-w-0 h-7 text-xs opacity-60"
        value=""
        disabled
        placeholder="not yet applied"
      />

      <div className="flex min-w-0 flex-wrap items-center gap-2">
        {options.length ? (
          options.map((option) => {
            const checked = Array.isArray(item?.index_types) && item.index_types.includes(option.id);
            return (
              <label key={option.id} className="inline-flex items-center gap-1 text-[11px] text-ui-text-soft">
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(event) => {
                    const next = new Set(Array.isArray(item?.index_types) ? item.index_types : []);
                    if (event?.target?.checked) {
                      next.add(option.id);
                    } else {
                      next.delete(option.id);
                    }
                    onChange({ ...item, index_types: Array.from(next) });
                  }}
                />
                <span>{option.label}</span>
              </label>
            );
          })
        ) : (
          <span className="text-[11px] text-ui-text-muted">—</span>
        )}
      </div>

      <Button type="button" variant="ghost" size="sm" className="h-7 w-7 shrink-0 p-0" onClick={onRemove} title="Remove column">
        {"\u2715"}
      </Button>
    </div>
  );
}

export default function CreateTableDialog({
  open,
  onOpenChange,
  tableSlug,
  setTableSlug,
  types,
  attributes,
  setAttributes,
  status,
  busy,
  onSubmit,
}) {
  function updateAttribute(index, nextValue) {
    setAttributes((prev) => prev.map((item, itemIndex) => (itemIndex === index ? nextValue : item)));
  }

  function removeAttribute(index) {
    setAttributes((prev) => prev.filter((_, itemIndex) => itemIndex !== index));
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="wide" className="border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Create Table</DialogTitle>
          <p className="text-sm text-body-soft">
            Define a sekejap table and its attributes. Index options change based on the selected kind.
          </p>
          <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : status.startsWith("Created") ? "text-success" : "text-body-soft")}>
            {status}
          </p>
        </DialogHeader>

        <form
          onSubmit={onSubmit}
          className="flex flex-col gap-4 px-6 py-4"
        >
          <Field label="Table Slug">
            <Input
              value={tableSlug}
              onInput={(event) => setTableSlug(event?.target?.value || "")}
              placeholder="posts"
              required
              disabled={busy}
            />
          </Field>

          <Field label="Columns">
            <div className="flex flex-col">
              <AttributeEditorHeader />
              {attributes.map((item, index) => (
                <AttributeEditorRow
                  key={`attr-${index}`}
                  item={item}
                  types={types}
                  onChange={(nextValue) => updateAttribute(index, nextValue)}
                  onRemove={() => removeAttribute(index)}
                />
              ))}
              <div className="flex items-center justify-between gap-3 rounded-lg border border-dashed border-ui-border px-3 py-2">
                <p className="text-xs text-ui-text-soft">Add only the attributes you want to predeclare. Sekejap still accepts dynamic JSON payloads.</p>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setAttributes((prev) => [...prev, { ...DEFAULT_ATTRIBUTE }])}
                  disabled={busy}
                >
                  Add Attribute
                </Button>
              </div>
            </div>
          </Field>

          <DialogFooter>
            <Button type="button" variant="ghost" size="sm" onClick={() => onOpenChange(false)} disabled={busy}>
              Cancel
            </Button>
            <Button type="submit" size="sm" disabled={busy}>
              {busy ? "Creating…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
