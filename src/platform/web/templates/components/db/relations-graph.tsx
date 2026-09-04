import { useState, useEffect, cx } from "zeb";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import { StudioTable, StudioTd } from "@/components/ui/studio-data-table";

/**
 * Free edges between rows, for engines whose relation style is `graph`.
 *
 * Declared by `capabilities.relations === "graph"`. Only sekejap uses this
 * today; it is kept engine-neutral for multimodel engines such as ArangoDB
 * and SurrealDB, which express relationships the same way. Engines that
 * declare `foreign_key` render a different panel instead.
 */

export function relationNodeSlug(record, fallbackCollection = "") {
  const explicit = String(record?._id || record?.slug || "").trim();
  if (explicit) return explicit;
  const collection = String(record?._collection || fallbackCollection || "").trim();
  const key = String(record?._key || "").trim();
  return collection && key ? `${collection}/${key}` : "";
}

export function relationNodeLabel(record, fallbackCollection = "") {
  return (
    String(record?.title || "").trim() ||
    String(record?.fullname || "").trim() ||
    String(record?.full_name || "").trim() ||
    String(record?.name || "").trim() ||
    String(record?.label || "").trim() ||
    String(record?.post_id || "").trim() ||
    String(record?._key || "").trim() ||
    relationNodeSlug(record, fallbackCollection)
  );
}

export function normalizeRelationType(value) {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9_]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

export function relationSlugParts(slug) {
  const text = String(slug || "").trim();
  const [collection, key, ...rest] = text.split("/");
  if (!collection || !key || rest.length) return null;
  return { collection, key };
}

export function uniqueRelationDefs(defs) {
  const seen = new Set();
  return (defs || [])
    .map((item) => ({
      from: String(item?.from || "").trim(),
      to: String(item?.to || "").trim(),
      type: String(item?.type || "").trim(),
    }))
    .filter((item) => item.from && item.to && item.type)
    .filter((item) => {
      const key = `${item.from}:${item.type}:${item.to}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

export function relationCountFromRows(rows) {
  const first = Array.isArray(rows) && Array.isArray(rows[0]) ? rows[0][0] : null;
  const count = Number(first);
  return Number.isFinite(count) ? count : null;
}

export function RelationDialog({
  open,
  onOpenChange,
  busy,
  status,
  direction,
  setDirection,
  relationType,
  setRelationType,
  relatedNodeSlug,
  setRelatedNodeSlug,
  currentNodeSlug,
  relationTypeOptions,
  relatedSlugWarning,
  onOpenTargetSearch,
  onSubmit,
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Create Relation</DialogTitle>
          <p className="text-sm text-body-soft">
            Link the current node to another node in the Sekejap store.
          </p>
          <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : status.startsWith("Created") ? "text-success" : "text-body-soft")}>
            {status}
          </p>
        </DialogHeader>

        <form onSubmit={onSubmit} className="flex flex-col gap-4 px-6 py-4">
          <Field label="Current Node">
            <Input value={currentNodeSlug} disabled />
          </Field>

          <div className="grid gap-3 md:grid-cols-2">
            <Field label="Direction">
              <Select value={direction} onChange={(event) => setDirection(event?.target?.value || "outgoing")} disabled={busy}>
                <SelectOption value="outgoing" label="Current -> related" />
                <SelectOption value="incoming" label="Related -> current" />
              </Select>
            </Field>
            <Field label="Relation Type">
              <div className="flex flex-col gap-2">
                {relationTypeOptions?.length ? (
                  <Select value={relationTypeOptions.includes(relationType) ? relationType : ""} onChange={(event) => setRelationType(event?.target?.value || "")} disabled={busy}>
                    <SelectOption value="" label="New or custom type" />
                    {relationTypeOptions.map((type) => (
                      <SelectOption key={type} value={type} label={type} />
                    ))}
                  </Select>
                ) : null}
                <Input
                  value={relationType}
                  onInput={(event) => setRelationType(event?.target?.value || "")}
                  placeholder="references"
                  required
                  disabled={busy}
                />
              </div>
            </Field>
          </div>

          <Field label={direction === "outgoing" ? "Target Node Slug" : "Source Node Slug"}>
            <div className="flex gap-2">
              <Input
                value={relatedNodeSlug}
                onInput={(event) => setRelatedNodeSlug(event?.target?.value || "")}
                placeholder="people/alice"
                required
                disabled={busy}
              />
              <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onOpenTargetSearch}>
                Search
              </Button>
            </div>
            {relatedSlugWarning ? (
              <p className="mt-1 text-xs text-amber-500">{relatedSlugWarning}</p>
            ) : null}
          </Field>

          <p className="text-xs text-body-soft">
            Use the Sekejap node slug format: <span className="font-mono">collection/key</span>.
          </p>

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

export function RelationTargetSearchDialog({ open, onOpenChange, tables, onSearch, onSelect }) {
  const [collection, setCollection] = useState("");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState([]);
  const [status, setStatus] = useState("Choose a collection and search existing nodes.");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    const first = tables?.[0]?.table || "";
    setCollection((current) => current || first);
    setQuery("");
    setResults([]);
    setStatus("Choose a collection and search existing nodes.");
  }, [open, tables?.length]);

  async function runSearch() {
    if (!collection) return;
    setBusy(true);
    setStatus("Searching…");
    try {
      const items = await onSearch(collection, query);
      setResults(items);
      setStatus(`${items.length} node(s) found.`);
    } catch (error) {
      setResults([]);
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="wide" className="border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Search Node</DialogTitle>
          <p className="text-sm text-body-soft">Select an existing node for the relation target.</p>
          <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : "text-body-soft")}>{status}</p>
        </DialogHeader>
        <div className="flex flex-col gap-3 px-6 py-4">
          <div className="grid gap-3 md:grid-cols-[12rem_minmax(0,1fr)_auto]">
            <Select value={collection} onChange={(event) => setCollection(event?.target?.value || "")} disabled={busy}>
              {(tables || []).map((table) => (
                <SelectOption key={table.table} value={table.table} label={table.table} />
              ))}
            </Select>
            <Input value={query} onInput={(event) => setQuery(event?.target?.value || "")} placeholder="Search by _key, title, name, slug…" disabled={busy} />
            <Button type="button" size="sm" disabled={busy || !collection} onClick={runSearch}>Search</Button>
          </div>
          <div className="max-h-96 overflow-auto rounded-md border border-ui-border/70">
            {results.length ? (
              <StudioTable>
                <StudioThead>
                  <tr>
                    <StudioTh>Node</StudioTh>
                    <StudioTh>Label</StudioTh>
                    <StudioTh></StudioTh>
                  </tr>
                </StudioThead>
                <tbody>
                  {results.map((item) => (
                    <tr key={item.slug}>
                      <StudioTd>{item.slug}</StudioTd>
                      <StudioTd>{item.label}</StudioTd>
                      <StudioTd>
                        <Button type="button" variant="outline" size="sm" onClick={() => { onSelect(item.slug); onOpenChange(false); }}>
                          Use
                        </Button>
                      </StudioTd>
                    </tr>
                  ))}
                </tbody>
              </StudioTable>
            ) : (
              <p className="px-3 py-6 text-center text-sm text-ui-text-soft">No nodes loaded yet.</p>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function RelationStatsList({ title, items, emptyText, peerKey }) {
  return (
    <div className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/10 p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">{title}</p>
        <span className="text-xs text-ui-text-soft">{items?.length || 0}</span>
      </div>
      {items?.length ? (
        <div className="space-y-2">
          {items.map((item, index) => (
            <div key={`${title}-${item.type}-${item.from}-${item.to}-${index}`} className="grid grid-cols-[minmax(0,1fr)_auto] gap-3 rounded-md border border-ui-border/70 bg-ui-bg px-3 py-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium text-ui-text">{item.type}</p>
                <p className="truncate text-xs text-ui-text-soft">{peerKey === "to" ? `to ${item.to}` : `from ${item.from}`}</p>
                {item.countError ? (
                  <p className="mt-1 text-[11px] text-amber-500">Count unavailable</p>
                ) : null}
              </div>
              <div className="text-right">
                <p className="font-mono text-sm font-semibold tabular-nums text-ui-text">
                  {item.count === null || item.count === undefined ? "n/a" : Number(item.count).toLocaleString()}
                </p>
                <p className="text-[11px] uppercase tracking-[0.12em] text-ui-text-soft">edges</p>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-ui-text-soft">{emptyText}</p>
      )}
    </div>
  );
}

export function RowRelationList({ title, items, emptyText, onDelete }) {
  return (
    <div className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/10 p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">{title}</p>
        <span className="text-xs text-ui-text-soft">{items?.length || 0}</span>
      </div>
      {items?.length ? (
        <div className="space-y-2">
          {items.map((entry, index) => (
            <div key={`${title}-${entry.type}-${entry.otherSlug}-${index}`} className="rounded-md border border-ui-border/70 bg-ui-bg px-3 py-2">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="text-sm font-medium text-ui-text">{entry.type}</p>
                  <p className="truncate text-xs text-ui-text-soft">{entry.otherLabel}</p>
                  <p className="truncate text-[11px] text-ui-text-muted">{entry.otherSlug}</p>
                </div>
                <Button type="button" variant="ghost" size="sm" onClick={() => onDelete(entry)}>
                  Delete
                </Button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-ui-text-soft">{emptyText}</p>
      )}
    </div>
  );
}
