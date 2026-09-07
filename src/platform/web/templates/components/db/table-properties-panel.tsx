import { cx } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import {
  AttributeEditorHeader,
  AttributeEditorRow,
  DEFAULT_ATTRIBUTE,
} from "@/components/db/create-table-dialog";
import { PROPERTY_SECTIONS } from "@/components/db/table-data";

/**
 * The table's facets down the left.
 *
 * Only columns is wired; the rest are listed so the shape of the screen is
 * visible and each one has an obvious place to land.
 */
function SectionRail({ section, onSelect }) {
  return (
    <nav className="flex w-40 shrink-0 flex-col gap-0.5 overflow-y-auto border-r border-ui-border/70 bg-ui-bg-muted/20 p-2">
      {PROPERTY_SECTIONS.map((item) => (
        <button
          key={item.id}
          type="button"
          disabled={!item.ready}
          onClick={() => item.ready && onSelect(item.id)}
          title={item.ready ? item.label : `${item.label} — not built yet`}
          className={cx(
            "flex items-center gap-2 rounded px-2 py-1 text-left text-xs transition-colors",
            section === item.id ? "bg-ui-bg-muted text-ui-text" : "text-ui-text-soft hover:bg-ui-bg-muted/60",
            !item.ready ? "cursor-not-allowed opacity-40" : "",
          )}
        >
          <span className="w-3 shrink-0 text-center text-[10px]">{item.glyph}</span>
          <span className="truncate">{item.label}</span>
        </button>
      ))}
    </nav>
  );
}

/** What you are looking at, stated the way a database tool states it. */
function TableHeadline({ activeTable, engine, columnCount }) {
  const facts = [
    ["Table", activeTable.table],
    ["Schema", activeTable.schema],
    ["Engine", engine],
    ["Columns", columnCount],
  ];
  return (
    <div className="mb-3 grid gap-x-6 gap-y-1 border-b border-ui-border/60 pb-3 text-xs sm:grid-cols-2">
      {facts.map(([label, value]) => (
        <div key={label} className="flex gap-2">
          <span className="w-20 shrink-0 text-ui-text-muted">{label}</span>
          <span className="truncate text-ui-text">{value}</span>
        </div>
      ))}
    </div>
  );
}

/** Adding, changing and removing the table's columns. */
function ColumnsEditor({ properties, types }) {
  const { attributes, setAttributes } = properties;
  return (
    <form onSubmit={properties.submit} className="flex flex-col gap-5">
      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Columns</p>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setAttributes((prev) => [...prev, { ...DEFAULT_ATTRIBUTE }])}
          >
            Add Attribute
          </Button>
        </div>
        {attributes.length === 0 ? (
          <p className="text-xs text-ui-text-soft">This table has no columns yet.</p>
        ) : (
          <>
            <AttributeEditorHeader />
            {attributes.map((attr, idx) => (
              <AttributeEditorRow
                key={idx}
                item={attr}
                types={types}
                onChange={(next) => setAttributes((prev) => prev.map((a, i) => (i === idx ? next : a)))}
                onRemove={() => setAttributes((prev) => prev.filter((_, i) => i !== idx))}
              />
            ))}
          </>
        )}
      </div>

      <div className="flex items-center gap-3">
        <Button type="submit" size="sm" disabled={properties.busy}>
          {properties.busy ? "Saving…" : "Save Changes"}
        </Button>
        {properties.status ? (
          <span className="text-xs text-ui-text-soft">{properties.status}</span>
        ) : null}
      </div>
    </form>
  );
}

/**
 * Dropping the table, asked for by name.
 *
 * The warning shows even where the engine cannot drop a table and only the
 * button is withheld. That is what this screen did before and changing it is
 * not this refactor's business — but it is worth naming: a Danger Zone with
 * nothing in it tells the reader nothing.
 */
function DeleteTable({ activeTable, remove, canDropTable, onDeleted }) {
  return (
    <>
      <div className="mt-8 rounded-lg border border-red-300/60 bg-red-50/30 p-4 dark:border-red-800/50 dark:bg-red-950/20">
        <p className="text-sm font-medium text-red-700 dark:text-red-400">Danger Zone</p>
        <p className="mt-1 text-xs text-red-600/80 dark:text-red-400/70">
          Permanently delete this table and all its data. This action cannot be undone.
        </p>
        {canDropTable ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="mt-3 border-red-300 text-red-700 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-950/40"
            onClick={() => {
              remove.setInput("");
              remove.setOpen(true);
            }}
          >
            Delete Table
          </Button>
        ) : null}
      </div>

      {/* Asked through the shared dialog like every other yes/no question,
          with the name field as its body. */}
      <ConfirmDialog
        open={remove.open}
        onClose={() => remove.setOpen(false)}
        onConfirm={() => remove.run(onDeleted)}
        title="Delete table"
        message={`This permanently deletes ${activeTable.table} and every row in it. Type the table name to confirm.`}
        confirmLabel={remove.busy ? "Deleting…" : "Delete"}
        variant="destructive"
        busy={remove.busy}
        confirmDisabled={remove.input !== activeTable.table}
      >
        <Input
          className="mt-3"
          value={remove.input}
          onInput={(e) => remove.setInput(e.currentTarget.value)}
          placeholder={activeTable.table}
        />
      </ConfirmDialog>
    </>
  );
}

/** Everything about the table itself rather than the rows in it. */
export default function TablePropertiesPanel({
  activeTable,
  engine,
  types,
  properties,
  canDropTable,
  onDeleted,
}) {
  const chosen = PROPERTY_SECTIONS.find((s) => s.id === properties.section) || {};

  return (
    <div className="flex min-h-0 flex-1 flex-row overflow-hidden">
      <SectionRail section={properties.section} onSelect={properties.setSection} />

      <div className="flex min-w-0 flex-1 flex-col overflow-y-auto px-4 py-3">
        <TableHeadline
          activeTable={activeTable}
          engine={engine}
          columnCount={properties.attributes.length}
        />

        {properties.section !== "columns" ? (
          <div className="flex flex-1 items-center justify-center px-6 py-10 text-center">
            <div className="max-w-sm">
              <p className="text-sm text-ui-text">{chosen.label}</p>
              <p className="mt-2 text-xs text-ui-text-soft">
                Not built yet. The engine reports this, but nothing reads it back into the studio.
              </p>
            </div>
          </div>
        ) : (
          <>
            <ColumnsEditor properties={properties} types={types} />
            <DeleteTable
              activeTable={activeTable}
              remove={properties.remove}
              canDropTable={canDropTable}
              onDeleted={onDeleted}
            />
          </>
        )}
      </div>
    </div>
  );
}
