import { cx } from "zeb/react";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import { AttributeEditorHeader, AttributeEditorRow, DEFAULT_ATTRIBUTE } from "@/components/db/attribute-editor";
import EdgeEndsFields from "@/components/db/edge-ends-fields";

/**
 * Table creation for engines that allow it.
 *
 * Declared by `capabilities.create_table`. What the form holds belongs to
 * `CreateTablePanel` and arrives as `form`; what the engine can declare
 * arrives as `caps`, so an engine without column constraints or generated
 * keys shows those parts disabled or not at all. An engine whose relations
 * are a graph can also make an edge table: then the columns are the edges'
 * properties, and `tables` is what its two ends may reach.
 */
export default function CreateTableDialog({ open, onOpenChange, types, form, caps, tables }) {
  const { attributes, setAttributes, busy, status, edge, setEdge } = form;
  const keyDefaults = Array.isArray(caps?.keyDefaults) ? caps.keyDefaults : [];
  const isEdge = edge.enabled === true;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="wide" className="border-border bg-card text-foreground">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Create Table</DialogTitle>
          <p className="text-sm text-muted-foreground">
            Name the table and declare its columns. Index options follow the column's type.
          </p>
          <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : status.startsWith("Created") ? "text-success" : "text-muted-foreground")}>
            {status}
          </p>
        </DialogHeader>

        <form onSubmit={form.submit} className="flex flex-col gap-4 px-6 py-4">
          {caps?.graphRelations ? (
            <Field label="Kind">
              <Select value={isEdge ? "edge" : "table"} onChange={(event) => setEdge({ ...edge, enabled: event?.target?.value === "edge" })} disabled={busy}>
                <SelectOption value="table" label="Table · rows with a key" />
                <SelectOption value="edge" label="Edge table · edges between two tables' rows" />
              </Select>
            </Field>
          ) : null}

          <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_minmax(0,14rem)]">
            <Field label="Table name">
              <Input
                value={form.slug}
                onInput={(event) => form.setSlug(event?.target?.value || "")}
                placeholder={caps?.qualifySchema ? "posts, or schema.posts" : "posts"}
                required
                disabled={busy}
              />
            </Field>
            {keyDefaults.length && !isEdge ? (
              <Field label="Row key">
                <Select value={form.keyDefault} onChange={(event) => form.setKeyDefault(event?.target?.value || "")} disabled={busy}>
                  <SelectOption value="" label="Given on each insert" />
                  {keyDefaults.map((generator) => (
                    <SelectOption key={generator} value={generator} label={`Generated · ${generator}`} />
                  ))}
                </Select>
              </Field>
            ) : null}
          </div>

          {isEdge ? <EdgeEndsFields edge={edge} setEdge={setEdge} tables={tables} busy={busy} /> : null}

          <Field label={isEdge ? "Edge properties" : "Columns"}>
            <div className="flex flex-col">
              <AttributeEditorHeader />
              {attributes.map((item, index) => (
                <AttributeEditorRow
                  key={`attr-${index}`}
                  item={item}
                  types={types}
                  constraints={caps?.columnConstraints === true}
                  onChange={(next) => setAttributes((prev) => prev.map((row, at) => (at === index ? next : row)))}
                  onRemove={() => setAttributes((prev) => prev.filter((_, at) => at !== index))}
                />
              ))}
              <div className="mt-2 flex items-center justify-between gap-3 rounded-lg border border-dashed border-border px-3 py-2">
                <p className="text-xs text-muted-foreground">A default is written as a value: 0, true, pending, or now().</p>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setAttributes((prev) => [...prev, { ...DEFAULT_ATTRIBUTE }])}
                  disabled={busy}
                >
                  Add Column
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
