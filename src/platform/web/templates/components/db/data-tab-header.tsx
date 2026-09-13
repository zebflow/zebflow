import { cx } from "zeb/react";

/** One of the three facets of the open table. */
function SubTab({ label, active, disabled, onClick }) {
  return (
    <button
      type="button"
      disabled={disabled}
      className={cx(
        "px-3 py-1.5 text-xs font-medium transition-colors",
        active ? "border-b-2 border-foreground text-foreground" : "text-muted-foreground hover:text-foreground",
        disabled ? "cursor-not-allowed opacity-40" : "",
      )}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

/**
 * What is open, what is known about it, and which facet of it is showing.
 *
 * The schema actions sit here rather than on the properties tab because they
 * are about the store, not about one table, and the reader reaches for them
 * from wherever they happen to be.
 */
export default function DataTabHeader({ table, facts, tabs, schema }) {
  return (
    <>
      <div className="flex items-center justify-between gap-3 border-b border-border/70 bg-accent/30 px-3 py-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-foreground">
            {table.name || "No table selected"}
          </p>
          <p className="text-xs text-muted-foreground">
            {table.active
              ? `${table.active.schema} schema`
              : "Choose a table from the left or create a new one."}
          </p>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          {table.active ? (
            <div className="flex flex-wrap items-center justify-end gap-2 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
              <span className="rounded-full border border-border/80 px-2 py-1">
                {table.active.rowCount || 0} rows
              </span>
              <span className="rounded-full border border-border/80 px-2 py-1">
                {facts.fieldCount} fields
              </span>
              <span className="rounded-full border border-border/80 px-2 py-1">
                {facts.indexCount} indexed
              </span>
            </div>
          ) : null}
          <div className="flex items-center gap-2">
            {schema.canSync ? (
              <button
                type="button"
                className="rounded border border-border px-2 py-1 text-xs font-medium text-muted-foreground hover:text-foreground disabled:opacity-50"
                disabled={schema.busy}
                onClick={schema.onSync}
              >
                Sync schema to repo
              </button>
            ) : null}
            {schema.exportHref ? (
              <a
                href={schema.exportHref}
                download={schema.exportFilename}
                className="rounded border border-border px-2 py-1 text-xs font-medium text-muted-foreground hover:text-foreground"
              >
                Download schema
              </a>
            ) : null}
          </div>
        </div>
      </div>

      {schema.status ? (
        <div className="border-b border-border/70 px-3 py-1 text-xs text-muted-foreground">
          {schema.status}
        </div>
      ) : null}

      <div className="flex items-center gap-1 border-b border-border/70 px-3">
        <SubTab label="Data" active={tabs.current === "data"} onClick={() => tabs.onSelect("data")} />
        {tabs.showRelations ? (
          <SubTab
            label="Relations"
            active={tabs.current === "relations"}
            disabled={!table.active}
            onClick={() => tabs.onSelect("relations")}
          />
        ) : null}
        {tabs.showProperties ? (
          <SubTab
            label="Properties"
            active={tabs.current === "properties"}
            disabled={!table.active}
            onClick={() => tabs.onSelect("properties")}
          />
        ) : null}
      </div>
    </>
  );
}
