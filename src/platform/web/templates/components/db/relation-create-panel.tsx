import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import { sqlStringLiteral } from "@/components/db/table-data";
import {
  RelationDialog,
  RelationTargetSearchDialog,
  normalizeRelationType,
  relationNodeLabel,
  relationNodeSlug,
  relationSlugParts,
} from "@/components/db/relations-graph";

/**
 * Drawing a new edge from the selected row.
 *
 * Owns the whole form and its own trigger. What it will not decide is whether
 * the input was acceptable to the reader — a bad slug or a missing target is
 * reported through `onInvalidInput`, because the dialog that explains it
 * belongs to the page and is shown over everything.
 */
export default function RelationCreatePanel({
  runDbQuery,
  tables,
  current,
  typeOptions,
  onCreated,
  onInvalidInput,
}) {
  const { table, record } = current;

  const [open, setOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState(
    "Choose the direction, relation type, and related node slug.",
  );
  const [direction, setDirection] = useState("outgoing");
  const [type, setType] = useState("");
  const [relatedSlug, setRelatedSlug] = useState("");

function openDialog() {
  setStatus("Choose the direction, relation type, and related node slug.");
  setDirection("outgoing");
  setType("");
  setRelatedSlug("");
  setOpen(true);
}

async function submit(event) {
  event?.preventDefault?.();
  const currentNodeSlug = relationNodeSlug(record, table?.table || "");
  const currentKey = String(record?._key || "").trim();
  const normalizedType = normalizeRelationType(type);
  const otherSlug = String(relatedSlug || "").trim();
  const otherParts = relationSlugParts(otherSlug);

  if (!table?.table || !currentNodeSlug || !currentKey) {
    setStatus("Error · Select a concrete row first.");
    return;
  }
  if (!normalizedType) {
    setStatus("Error · Relation type is required.");
    return;
  }
  if (!otherParts) {
    onInvalidInput({
      title: "Invalid Node Slug",
      message: "Related node slug must use collection/key format.",
      example: "people/alice",
    });
    setStatus("Error · Related node slug must use collection/key.");
    return;
  }
  if (!tables.some((table) => table.table === otherParts.collection)) {
    onInvalidInput({
      title: "Unknown Collection",
      message: `Collection '${otherParts.collection}' does not exist in this Sekejap store. Choose an existing collection before creating the relation.`,
      example: `${table.table}/${currentKey}`,
    });
    setStatus(`Error · Unknown collection '${otherParts.collection}'.`);
    return;
  }

  const fromSlug = direction === "outgoing" ? currentNodeSlug : otherSlug;
  const toSlug = direction === "outgoing" ? otherSlug : currentNodeSlug;
  const sql = `INSERT ('${sqlStringLiteral(fromSlug)}')-[:${normalizedType}]->('${sqlStringLiteral(toSlug)}')`;

  setBusy(true);
  setStatus("Creating relation…");
  try {
    const exists = await runDbQuery(`SELECT _key FROM ${otherParts.collection} WHERE _key = '${sqlStringLiteral(otherParts.key)}'`, {
      readOnly: true,
      tableName: otherParts.collection,
      limit: 1,
    });
    if (!exists.rows.length) {
      onInvalidInput({
        title: "Target Node Not Found",
        message: `Node '${otherSlug}' does not exist. Search and select an existing node, or create the node first.`,
        example: `${otherParts.collection}/existing_key`,
      });
      setStatus(`Error · Node '${otherSlug}' not found.`);
      return;
    }
    await runDbQuery(sql, { readOnly: false, tableName: table.table, limit: 50 });
    setStatus(`Created · ${normalizedType}`);
    setOpen(false);
    await onCreated();
  } catch (error) {
    setStatus(`Error · ${String(error?.message || error)}`);
  } finally {
    setBusy(false);
  }
}

async function searchTargets(collection, query) {
  const q = String(query || "").trim();
  const table = (tables || []).find((item) => String(item?.table || "") === String(collection || ""));
  const candidateCols = [
    "_collection",
    "_key",
    "_id",
    ...(table?.attributes || [])
      .filter((attr) => {
        const kind = String(attr?.kind || "").toLowerCase();
        return kind !== "geo" && kind !== "vector";
      })
      .map((attr) => String(attr?.name || "").trim())
      .filter(Boolean),
  ];
  const seenCols = new Set();
  const columns = candidateCols.filter((col) => {
    if (!col || seenCols.has(col)) return false;
    seenCols.add(col);
    return true;
  });
  const result = await runDbQuery(
    `SELECT ${columns.join(", ")} FROM ${collection}`,
    { readOnly: true, tableName: collection, limit: 200 }
  );
  return result.objects.map((item) => ({
    slug: relationNodeSlug(item, collection),
    label: relationNodeLabel(item, collection),
    searchText: Object.values(item || {}).map((value) => {
      if (value === null || value === undefined) return "";
      if (typeof value === "object") {
        try { return JSON.stringify(value); } catch (_) { return ""; }
      }
      return String(value);
    }).join(" "),
  })).filter((item) => {
    if (!item.slug) return false;
    const needle = q.toLowerCase();
    return !needle || item.slug.toLowerCase().includes(needle) || item.label.toLowerCase().includes(needle) || item.searchText.toLowerCase().includes(needle);
  });
}

  const currentSlug = relationNodeSlug(record, table?.table || "");
  const relatedParts = relationSlugParts(String(relatedSlug || "").trim());
  const relatedWarning =
    relatedSlug.trim() && relatedParts && !tables.some((t) => t.table === relatedParts.collection)
      ? `Collection '${relatedParts.collection}' does not exist in this store.`
      : "";

  return (
    <>
      <Button type="button" variant="outline" size="sm" onClick={openDialog}>
        New Relation
      </Button>

      <RelationDialog
        open={open}
        onOpenChange={setOpen}
        busy={busy}
        status={status}
        direction={direction}
        setDirection={setDirection}
        relationType={type}
        setRelationType={setType}
        relatedNodeSlug={relatedSlug}
        setRelatedNodeSlug={setRelatedSlug}
        currentNodeSlug={currentSlug}
        relationTypeOptions={typeOptions}
        relatedSlugWarning={relatedWarning}
        onOpenTargetSearch={() => setSearchOpen(true)}
        onSubmit={submit}
      />

      <RelationTargetSearchDialog
        open={searchOpen}
        onOpenChange={setSearchOpen}
        tables={tables}
        onSearch={searchTargets}
        onSelect={setRelatedSlug}
      />
    </>
  );
}
