import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import CreateTableDialog from "@/components/db/create-table-dialog";
import { DEFAULT_ATTRIBUTE, attributesPayload } from "@/components/db/attribute-editor";

const READY = "Name the table, then add its columns.";

/**
 * Defining a new table.
 *
 * Owns the form and everything it is part-way through. Whether the dialog is
 * open stays with the page, because two separate buttons open it — the one in
 * the tree and the one on the schema tab — and neither belongs to the other.
 */
export default function CreateTablePanel({ open, onOpenChange, tablesApi, types, caps, onCreated }) {
  const [slug, setSlug] = useState("");
  const [keyDefault, setKeyDefault] = useState("");
  const [attributes, setAttributes] = useState([{ ...DEFAULT_ATTRIBUTE }]);
  const [status, setStatus] = useState(READY);
  const [busy, setBusy] = useState(false);

  async function submit(event) {
    event?.preventDefault?.();
    const table = String(slug || "").trim();
    if (!table) {
      setStatus("Error · A table name is required.");
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
          key_default: keyDefault,
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
      caps={caps}
      open={open}
      onOpenChange={(next) => {
        if (next) {
          // A fresh form each time it is opened; the previous attempt's name
          // and status are not an answer to this one.
          setStatus(READY);
          setSlug("");
          setKeyDefault("");
          setAttributes([{ ...DEFAULT_ATTRIBUTE }]);
        }
        onOpenChange(next);
      }}
      form={{ slug, setSlug, keyDefault, setKeyDefault, attributes, setAttributes, status, busy, submit }}
    />
  );
}
