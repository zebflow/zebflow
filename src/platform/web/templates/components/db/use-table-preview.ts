import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";

const DESCRIBE_COLUMNS = ["field", "type", "nullable", "primary_key", "default"];

/**
 * The first page of rows of the open table, and what the engine says its
 * columns are.
 *
 * Both answers arrive from the same table name and are wanted by the same
 * screens, so they load together rather than from two places that could
 * disagree about which table is open.
 */
export function useTablePreview({ previewUrl, describeUrl, table }) {
  const [previewColumns, setPreviewColumns] = useState([]);
  const [previewRows, setPreviewRows] = useState([]);
  const [previewError, setPreviewError] = useState("");
  const [schemaColumns, setSchemaColumns] = useState([]);
  const [schemaRows, setSchemaRows] = useState([]);
  const [schemaError, setSchemaError] = useState("");

  async function reloadPreview(target = table) {
    if (!previewUrl || !target) return { columns: [], rows: [] };
    const payload = await requestJson(
      `${previewUrl}?table=${encodeURIComponent(target)}&limit=120`,
    );
    const result = payload?.result || {};
    const columns = Array.isArray(result?.columns)
      ? result.columns.map((item) => String(item?.name || ""))
      : [];
    const rows = Array.isArray(result?.rows) ? result.rows : [];
    setPreviewColumns(columns);
    setPreviewRows(rows);
    setPreviewError("");
    return { columns, rows };
  }

  useEffect(() => {
    if (!previewUrl) return;
    if (!table) {
      setPreviewColumns([]);
      setPreviewRows([]);
      setPreviewError("");
      writeTableToUrl("");
      return;
    }
    let active = true;
    reloadPreview(table).catch((error) => {
      if (!active) return;
      setPreviewColumns([]);
      setPreviewRows([]);
      setPreviewError(String(error?.message || error));
    });
    writeTableToUrl(table);
    return () => {
      active = false;
    };
  }, [previewUrl, table]);

  useEffect(() => {
    if (!describeUrl || !table) return;
    let active = true;
    // Sent exactly as the tree named it, qualifier included. An engine that
    // namespaces its tables needs the qualifier to find the right one, and an
    // engine that does not simply drops it.
    requestJson(`${describeUrl}?scope=columns&table=${encodeURIComponent(table)}`)
      .then((payload) => {
        if (!active) return;
        const nodes = Array.isArray(payload?.result?.nodes) ? payload.result.nodes : [];
        setSchemaColumns(nodes.length ? DESCRIBE_COLUMNS : []);
        setSchemaRows(
          nodes.map((node) => {
            const meta = node?.meta ?? {};
            return [
              String(node?.name ?? ""),
              meta.type ?? meta.data_type ?? "",
              meta.nullable === undefined ? "" : String(meta.nullable),
              meta.pk === true ? "true" : "false",
              meta.default ?? "",
            ];
          }),
        );
        setSchemaError("");
      })
      .catch((error) => {
        if (!active) return;
        setSchemaColumns([]);
        setSchemaRows([]);
        setSchemaError(String(error?.message || error));
      });
    return () => {
      active = false;
    };
  }, [describeUrl, table]);

  return {
    previewColumns,
    previewRows,
    previewError,
    reloadPreview,
    schemaColumns,
    schemaRows,
    schemaError,
  };
}

/** Keeps `?table=` in step with what is open, so the page can be linked to. */
function writeTableToUrl(table) {
  if (typeof window === "undefined") return;
  const next = new URL(window.location.href);
  if (table) {
    next.searchParams.set("table", table);
  } else {
    next.searchParams.delete("table");
  }
  window.history.replaceState({}, "", next.toString());
}
