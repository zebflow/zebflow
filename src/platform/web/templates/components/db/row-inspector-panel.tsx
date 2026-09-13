import ConfirmDialog from "@/components/ui/confirm-dialog";
import { RowRelationList } from "@/components/db/relations-graph";
import GeoPreviewMap from "@/components/db/geo-preview";
import { isGeoJsonGeometry } from "@/components/db/cell-format";

/** The cell the reader last clicked, rendered as a map when it is a geometry. */
function InspectedValue({ value, hasGeo, compact }) {
  const map = hasGeo && isGeoJsonGeometry(value.raw) ? <GeoPreviewMap geometry={value.raw} /> : null;
  if (!compact) {
    return (
      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto overflow-x-hidden px-3 py-3">
        {map}
        <pre className="db-suite-value-body" style={{ margin: 0 }}>{value.body}</pre>
      </div>
    );
  }
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">Selected Value</p>
      <p className="text-xs text-muted-foreground">{value.meta}</p>
      {map}
      <pre
        className="max-h-32 overflow-y-auto overflow-x-hidden rounded-md border border-border/70 bg-accent/20 p-2 text-xs text-foreground"
        style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}
      >
        {value.body}
      </pre>
    </div>
  );
}

/** What this row points at, and what points at it. */
function RowRelations({ relations, onDelete }) {
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">Relations</p>
        {relations.busy ? <span className="text-[11px] text-muted-foreground">Loading…</span> : null}
      </div>
      {relations.error ? (
        <p className="text-xs text-danger">Failed to load relations: {relations.error}</p>
      ) : null}
      <RowRelationList
        title="Outgoing"
        items={relations.outgoing}
        emptyText="No outgoing relations."
        onDelete={onDelete}
      />
      <RowRelationList
        title="Incoming"
        items={relations.incoming}
        emptyText="No incoming relations."
        onDelete={onDelete}
      />
    </div>
  );
}

/** The open table in four numbers. */
function TableFacts({ activeTable, fieldCount, indexCount }) {
  return (
    <div className="grid grid-cols-2 gap-2 text-xs uppercase tracking-[0.12em] text-muted-foreground">
      <span>Rows</span>
      <span className="text-right">{activeTable.rowCount || 0}</span>
      <span>Fields</span>
      <span className="text-right">{fieldCount}</span>
      <span>Indexes</span>
      <span className="text-right">{indexCount}</span>
      <span>Slug</span>
      <span className="truncate text-right normal-case tracking-normal text-foreground">
        {activeTable.table}
      </span>
    </div>
  );
}

/** Every column the reader can see, as chips. Internals starting `_` are not shown. */
function FieldChips({ names }) {
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">Fields</p>
      {names.length ? (
        <div className="flex flex-wrap gap-2">
          {names.map((name) => (
            <span
              key={name}
              className="inline-flex rounded-full border border-border/80 px-2 py-1 text-xs text-muted-foreground"
            >
              {name}
            </span>
          ))}
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">No field metadata available yet.</p>
      )}
    </div>
  );
}

/**
 * The column beside the grid: whichever of the row, the cell, or the table the
 * reader has actually asked about.
 *
 * Owns only the relation it is about to delete, because that confirmation is
 * this panel's own question. Everything else it is given.
 */
export default function RowInspectorPanel({ node, value, relations, facts, hasGeo, children }) {
  const { record, slug, label, activeTable } = node;
  const hasValue = !!String(value.body || "").trim();

  return (
    <aside className="db-suite-value-panel">
      <ConfirmDialog
        open={!!relations.pendingDelete}
        onClose={() => relations.setPendingDelete(null)}
        onConfirm={() => {
          const entry = relations.pendingDelete;
          relations.setPendingDelete(null);
          if (entry) relations.onDelete(entry);
        }}
        title="Delete Relation"
        message={
          relations.pendingDelete
            ? `Delete relation ${relations.pendingDelete.type} between ${slug} and ${relations.pendingDelete.otherSlug}?`
            : ""
        }
        confirmLabel="Delete"
        variant="destructive"
      />

      <div className="db-suite-value-head">
        {record ? "Node" : hasValue ? "Value" : "Overview"}
      </div>
      <div className="db-suite-value-meta">
        {record
          ? slug || value.meta
          : hasValue
            ? value.meta
            : activeTable
              ? `${activeTable.schema}.${activeTable.table}`
              : "Select a table"}
      </div>

      {record ? (
        <div
          className="flex min-h-0 flex-col gap-4 overflow-y-auto overflow-x-hidden px-3 py-3 text-sm text-foreground"
          style={{ wordBreak: "break-word" }}
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="truncate text-sm font-medium text-foreground">{label || slug}</p>
              <p className="truncate text-xs text-muted-foreground">{slug}</p>
            </div>
            {children}
          </div>

          {hasValue ? <InspectedValue value={value} hasGeo={hasGeo} compact /> : null}
          {relations.enabled ? (
            <RowRelations relations={relations} onDelete={relations.setPendingDelete} />
          ) : null}
          {activeTable ? (
            <TableFacts
              activeTable={activeTable}
              fieldCount={facts.fieldCount}
              indexCount={facts.indexCount}
            />
          ) : null}
          <FieldChips names={facts.fieldNames} />
          {relations.enabled ? (
            <p className="text-xs text-muted-foreground">
              Open the Relations tab for collection-level relation statistics.
            </p>
          ) : null}
        </div>
      ) : hasValue ? (
        <InspectedValue value={value} hasGeo={hasGeo} compact={false} />
      ) : (
        <pre className="db-suite-value-body">
          Choose a table from the left to inspect its data and structure.
        </pre>
      )}
    </aside>
  );
}
