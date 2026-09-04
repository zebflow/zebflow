import { useState, cx } from "zeb";
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

const INDEX_OPTIONS_BY_KIND = {
  string: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
    { id: "fulltext", label: "Fulltext" },
  ],
  number: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  boolean: [{ id: "hash", label: "Exact" }],
  json: [],
  vector: [{ id: "vector", label: "Index" }],
  geo: [{ id: "spatial", label: "Index" }],
};

export const DEFAULT_ATTRIBUTE = {
  name: "",
  kind: "string",
  index_types: [],
};

export function AttributeEditorRow({ item, onChange, onRemove }) {
  const options = INDEX_OPTIONS_BY_KIND[item?.kind] || [];

  return (
    <div className="flex min-w-0 flex-col gap-3 rounded-lg border border-ui-border/80 bg-ui-bg-muted/40 p-3">
      <div className="grid min-w-0 gap-3 md:grid-cols-[minmax(0,1fr)_11rem]">
        <Input
          className="min-w-0"
          value={item?.name || ""}
          onInput={(event) => onChange({ ...item, name: event?.target?.value || "" })}
          placeholder="field_name"
        />
        <Select className="min-w-0" value={item?.kind || "string"} onChange={(event) => onChange({ ...item, kind: event?.target?.value || "string", index_types: [] })}>
          {Object.keys(INDEX_OPTIONS_BY_KIND).map((kind) => (
            <SelectOption key={kind} value={kind} label={kind} />
          ))}
        </Select>
      </div>
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <div className="flex min-h-9 min-w-0 flex-1 flex-wrap items-center gap-3 rounded-md border border-dashed border-ui-border px-3 py-2">
          {options.length ? (
            options.map((option) => {
              const checked = Array.isArray(item?.index_types) && item.index_types.includes(option.id);
              return (
                <label key={option.id} className="inline-flex items-center gap-2 text-xs text-ui-text-soft">
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
            <span className="text-xs text-ui-text-muted">No index</span>
          )}
        </div>
        <Button type="button" variant="ghost" size="sm" className="shrink-0" onClick={onRemove}>
          Remove
        </Button>
      </div>
    </div>
  );
}

export default function CreateTableDialog({
  open,
  onOpenChange,
  tableSlug,
  setTableSlug,
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

          <Field label="Attributes">
            <div className="flex flex-col gap-3">
              {attributes.map((item, index) => (
                <AttributeEditorRow
                  key={`attr-${index}`}
                  item={item}
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
