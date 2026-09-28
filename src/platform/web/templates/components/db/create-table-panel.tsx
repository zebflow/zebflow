import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import CreateTableDialog from "@/components/db/create-table-dialog";
import { DEFAULT_ATTRIBUTE, attributesPayload } from "@/components/db/attribute-editor";
import { DEFAULT_EDGE } from "@/components/db/edge-ends-fields";

const READY = "Name the table, then add its columns.";

/**
 * Defining a new table, or a new edge table.
 *
 * Owns the form and everything it is part-way through. Whether the dialog is
 * open stays with the page, because two separate buttons open it — the one in
 * the tree and the one on the schema tab — and neither belongs to the other.
 * `catalog` is the tables an edge may reach and the reload after creating.
 */
export default function CreateTablePanel({ open, onOpenChange, tablesApi, types, caps, catalog }) {
  const [slug, setSlug] = useState("");
  const [keyDefault, setKeyDefault] = useState("");
  const [attributes, setAttributes] = useState([{ ...DEFAULT_ATTRIBUTE }]);
  const [edge, setEdge] = useState({ ...DEFAULT_EDGE });
  const [status, setStatus] = useState(READY);
  const [busy, setBusy] = useState(false);

  async function submit(event) {
    event?.preventDefault?.();
    const table = String(slug || "").trim();
    if (!table) {
      setStatus("Error · A table name is required.");
      return;
    }
    if (edge.enabled && !(edge.source_table && edge.destination_table && edge.source.trim() && edge.destination.trim())) {
      setStatus("Error · An edge table names both ends: a table and a column for each.");
      return;
    }

    setBusy(true);
    setStatus("Creating table…");
    try {
      const payload = await requestJson(tablesApi, {
        method: "POST",
        body: JSON.stringify({
          table,
          attributes: attributesPayload(attributes),
          key_default: edge.enabled ? "" : keyDefault,
          edge: edge.enabled
            ? {
                source: edge.source.trim(),
                source_table: edge.source_table,
                destination: edge.destination.trim(),
                destination_table: edge.destination_table,
                label: edge.label.trim(),
                one_per_pair: edge.one_per_pair,
              }
            : null,
        }),
      });
      const created = String(payload?.table?.table || table).trim();
      setStatus(`Created · ${created}`);
      onOpenChange(false);
      await catalog.reload(created);
    } catch (error) {
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <CreateTableDialog
      types={types}
      caps={caps}
      tables={catalog.tables}
      open={open}
      onOpenChange={(next) => {
        if (next) {
          // A fresh form each time it is opened; the previous attempt's name
          // and status are not an answer to this one.
          setStatus(READY);
          setSlug("");
          setKeyDefault("");
          setAttributes([{ ...DEFAULT_ATTRIBUTE }]);
          setEdge({ ...DEFAULT_EDGE });
        }
        onOpenChange(next);
      }}
      form={{ slug, setSlug, keyDefault, setKeyDefault, attributes, setAttributes, edge, setEdge, status, busy, submit }}
    />
  );
}
