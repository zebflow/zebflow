import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import { attributeFromTable, attributesPayload } from "@/components/db/attribute-editor";

/**
 * Editing the open table: its columns, its index kinds, and deleting it.
 *
 * The form is refilled from the table itself whenever a different one is
 * opened, so what the reader sees is always the table named in the tree.
 *
 * `onTableChanged` is how the page hears that the catalog is stale — the hook
 * does not reach into it, because the catalog belongs to the page.
 */
export function useTableProperties({
  propertiesApi,
  tablesApi,
  schemaSyncApi,
  table,
  selectedTable,
  columnConstraints,
  onTableChanged,
}) {
  const [attributes, setAttributes] = useState([]);
  // Which facet of the table the pane is showing. Only columns is wired; the
  // rest are listed so the shape of the screen is visible and each one has an
  // obvious place to land.
  const [section, setSection] = useState("columns");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [syncBusy, setSyncBusy] = useState(false);
  const [syncStatus, setSyncStatus] = useState("");
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteInput, setDeleteInput] = useState("");
  const [deleteBusy, setDeleteBusy] = useState(false);

  // Keyed on what the catalog says the columns are, so a save that changed
  // them refills the form with the table as it now stands.
  useEffect(() => {
    if (!table) return;
    setAttributes((table.attributes || []).map(attributeFromTable));
  }, [table?.key, JSON.stringify(table?.attributes || [])]);

  // A status belongs to the table it was about; "Saved" stays after a save.
  useEffect(() => {
    setStatus("");
  }, [table?.key]);

  async function save(message = "Saving…") {
    if (!table) return null;
    setBusy(true);
    setStatus(message);
    const payload = { attributes: attributesPayload(attributes) };
    // Qualified as the tree named it, so a schema-namespaced engine alters the
    // right table.
    const response = await requestJson(
      `${propertiesApi}/${encodeURIComponent(selectedTable)}`,
      { method: "PUT", body: JSON.stringify(payload) },
    );
    setStatus("Saved");
    onTableChanged();
    return response;
  }

  async function submit(event) {
    event?.preventDefault?.();
    if (!table) return;
    try {
      await save("Saving…");
    } catch (error) {
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function syncSchemaToRepo() {
    setSyncBusy(true);
    setSyncStatus(table ? "Saving table and syncing schema…" : "Syncing schema…");
    try {
      if (table) await save("Saving before schema sync…");
      const payload = await requestJson(schemaSyncApi, { method: "POST" });
      const sync = payload?.sync || {};
      setSyncStatus(
        `Synced · ${Number(sync?.table_count || 0)} tables · ${Number(sync?.files_written?.length || 0)} files`,
      );
      onTableChanged();
    } catch (error) {
      setSyncStatus(`Sync failed · ${String(error?.message || error)}`);
    } finally {
      setSyncBusy(false);
      setBusy(false);
    }
  }

  async function deleteTable(onDeleted) {
    if (!table) return;
    setDeleteBusy(true);
    try {
      await requestJson(`${tablesApi}/${encodeURIComponent(selectedTable)}`, { method: "DELETE" });
      setDeleteOpen(false);
      setDeleteInput("");
      onDeleted();
      onTableChanged();
    } catch (error) {
      setStatus(`Delete failed · ${String(error?.message || error)}`);
    } finally {
      setDeleteBusy(false);
    }
  }

  return {
    attributes,
    setAttributes,
    section,
    setSection,
    // Whether the column editor may set NOT NULL, DEFAULT and UNIQUE.
    constraints: columnConstraints === true,
    busy,
    status,
    submit,
    sync: { busy: syncBusy, status: syncStatus, run: syncSchemaToRepo },
    remove: {
      open: deleteOpen,
      setOpen: setDeleteOpen,
      input: deleteInput,
      setInput: setDeleteInput,
      busy: deleteBusy,
      run: deleteTable,
    },
  };
}
