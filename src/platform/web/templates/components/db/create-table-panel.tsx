import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import CreateTableDialog, { DEFAULT_ATTRIBUTE } from "@/components/db/create-table-dialog";

/**
 * Defining a new table.
 *
 * Owns the form and everything it is part-way through. Whether the dialog is
 * open stays with the page, because two separate buttons open it — the one in
 * the tree and the one on the schema tab — and neither belongs to the other.
 */
export default function CreateTablePanel({ open, onOpenChange, tablesApi, types, onCreated }) {
  const [slug, setSlug] = useState("");
  const [attributes, setAttributes] = useState([{ ...DEFAULT_ATTRIBUTE }]);
  const [status, setStatus] = useState(
    "Define the table and save it into the project-local sekejap store.",
  );
  const [busy, setBusy] = useState(false);

  async function submit(event) {
    event?.preventDefault?.();
    const table = String(slug || "").trim();
    if (!table) {
      setStatus("Error · Table slug is required.");
      return;
    }

    setBusy(true);
    setStatus("Creating table…");
    try {
      const payload = await requestJson(tablesApi, {
        method: "POST",
        body: JSON.stringify({
          table,
          attributes: (attributes || [])
            .map((item) => ({
              name: String(item?.name || "").trim(),
              kind: String(item?.kind || "string"),
              index_types: Array.isArray(item?.index_types) ? item.index_types : [],
            }))
            .filter((item) => item.name),
        }),
      });
      const created = String(payload?.table?.table || table).trim();
      setStatus(`Created · ${created}`);
      onOpenChange(false);
      await onCreated(created);
    } catch (error) {
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <CreateTableDialog
      types={types}
      open={open}
      onOpenChange={(next) => {
        if (next) {
          // A fresh form each time it is opened; the previous attempt's slug
          // and status are not an answer to this one.
          setStatus("Define the table and save it into the project-local sekejap store.");
          setSlug("");
          setAttributes([{ ...DEFAULT_ATTRIBUTE }]);
        }
        onOpenChange(next);
      }}
      tableSlug={slug}
      setTableSlug={setSlug}
      attributes={attributes}
      setAttributes={setAttributes}
      status={status}
      busy={busy}
      onSubmit={submit}
    />
  );
}
