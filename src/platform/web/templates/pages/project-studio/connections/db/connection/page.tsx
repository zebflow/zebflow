import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { useEffect, useState, cx } from "zeb";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import { Select } from "@/components/ui/select";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import MapPicker from "@/components/ui/map-picker";
import ResizableDataGrid from "@/components/db/data-grid";
import GeoPreviewMap, { parseGeoJsonGeometry } from "@/components/db/geo-preview";
import StructureTable from "@/components/db/structure-table";
import MaintenancePanel from "@/components/db/maintenance-panel";
import CreateTableDialog, { AttributeEditorRow, DEFAULT_ATTRIBUTE } from "@/components/db/create-table-dialog";
import {
  RelationDialog,
  RelationTargetSearchDialog,
  RelationStatsList,
  RowRelationList,
  relationNodeSlug,
  relationNodeLabel,
  normalizeRelationType,
  relationSlugParts,
  uniqueRelationDefs,
  relationCountFromRows,
} from "@/components/db/relations-graph";
import {
  rawCellValue,
  displayCellText,
  prettyValue,
  isGeoJsonGeometry,
} from "@/components/db/cell-format";

export const page = {
  head: {
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-suite.css" },
      { rel: "stylesheet", href: "/assets/libraries/zeb/icons/0.1/runtime/devicons.css" },
    ],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}



function requestJson(url, options = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (response) => {
    if (response.status === 401) {
      window.location.href = "/login";
      return null;
    }
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const message =
        payload?.error?.message || payload?.message || `${response.status} ${response.statusText}`;
      throw new Error(message);
    }
    return payload;
  });
}

function normalizeSchemaNodes(nodes) {
  return (Array.isArray(nodes) ? nodes : [])
    .map((node) => String(node?.name || ""))
    .filter((name) => name && !name.startsWith("_"))
    .sort((a, b) => a.localeCompare(b));
}

function normalizeTableNodes(nodes) {
  return (Array.isArray(nodes) ? nodes : [])
    .filter((node) => String(node?.kind || "") === "table")
    .map((node) => {
      const schema = String(node?.schema || "default");
      const table = String(node?.name || "");
      const key = schema === "default" ? table : `${schema}.${table}`;
      return {
        schema,
        table,
        key,
        rowCount: Number(node?.meta?.row_count || 0),
        attributes: Array.isArray(node?.meta?.attributes) ? node.meta.attributes : [],
        hashIndexed: Array.isArray(node?.meta?.hash_indexed_fields) ? node.meta.hash_indexed_fields : [],
        rangeIndexed: Array.isArray(node?.meta?.range_indexed_fields) ? node.meta.range_indexed_fields : [],
        fulltextFields: Array.isArray(node?.meta?.fulltext_fields) ? node.meta.fulltext_fields : [],
        vectorFields: Array.isArray(node?.meta?.vector_fields) ? node.meta.vector_fields : [],
        spatialFields: Array.isArray(node?.meta?.spatial_fields) ? node.meta.spatial_fields : [],
      };
    })
    .filter((item) => item.schema && item.table && !item.schema.startsWith("_"))
    .sort((a, b) => a.key.localeCompare(b.key));
}



function sqlStringLiteral(value) {
  return String(value || "").replace(/'/g, "''");
}

function mapRowToObject(columns, row) {
  const output = {};
  (Array.isArray(columns) ? columns : []).forEach((column, index) => {
    output[String(column || `column_${index + 1}`)] = Array.isArray(row) ? row[index] : undefined;
  });
  return output;
}




function orderedDataColumns(columns) {
  const trailing = ["_created_unix", "_updated_unix"];
  const seen = new Set();
  const unique = (columns || []).map(String).filter((name) => {
    if (!name || seen.has(name)) return false;
    seen.add(name);
    return true;
  });
  return [
    ...unique.filter((name) => !trailing.includes(name)),
    ...trailing.filter((name) => unique.includes(name)),
  ];
}

function reorderRowsForColumns(sourceColumns, rows, targetColumns) {
  const indexByName = new Map((sourceColumns || []).map((name, index) => [String(name), index]));
  return (rows || []).map((row) =>
    targetColumns.map((name) => {
      const sourceIndex = indexByName.get(name);
      return Array.isArray(row) && sourceIndex !== undefined ? row[sourceIndex] : null;
    })
  );
}

function fieldKindForColumn(table, colName) {
  const name = String(colName || "");
  const attr = (table?.attributes || []).find((item) => String(item?.name || "") === name);
  if (attr?.kind) return String(attr.kind);
  if ((table?.spatialFields || []).includes(name)) return "geo";
  if ((table?.vectorFields || []).includes(name)) return "vector";
  return "";
}

function validateCellEditValue(table, colName, value) {
  const kind = fieldKindForColumn(table, colName);
  const text = String(value ?? "").trim();
  if (!text) return null;
  if (kind === "geo") {
    const geo = parseGeoJsonGeometry(text);
    if (!geo.ok) {
      return {
        title: geo.title,
        message: `${geo.message} Column: ${colName}. Coordinates use [longitude, latitude].`,
        example: geo.example,
      };
    }
  }
  if (kind === "number" && Number.isNaN(Number(text))) {
    return {
      title: "Invalid Number",
      message: `Column ${colName} expects a number. Use plain numeric values such as 12, 12.5, or -3.`,
      example: "12.5",
    };
  }
  if (kind === "json") {
    try {
      JSON.parse(text);
    } catch (_) {
      return {
        title: "Invalid JSON",
        message: `Column ${colName} expects valid JSON.`,
        example: '{"status":"active","tags":["demo"]}',
      };
    }
  }
  if (kind === "vector") {
    try {
      const parsed = JSON.parse(text);
      if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === "number" && Number.isFinite(item))) {
        throw new Error("vector must be numeric array");
      }
    } catch (_) {
      return {
        title: "Invalid Vector",
        message: `Column ${colName} expects a JSON array of numbers.`,
        example: "[0.12, 0.34, 0.56]",
      };
    }
  }
  return null;
}




function groupTablesBySchema(tables) {
  const map = new Map();
  (tables || []).forEach((item) => {
    if (!map.has(item.schema)) {
      map.set(item.schema, []);
    }
    map.get(item.schema).push(item);
  });
  return map;
}

function selectedTableDefinition(tables, selectedTable) {
  return (tables || []).find((item) => item.key === selectedTable) || null;
}







function DataWarningDialog({ notice, onClose }) {
  return (
    <Dialog open={!!notice} onOpenChange={(value) => { if (!value) onClose(); }}>
      <DialogContent className="max-w-xl border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>{notice?.title || "Invalid Input"}</DialogTitle>
          <p className="text-sm text-body-soft">{notice?.message || ""}</p>
        </DialogHeader>
        {notice?.example ? (
          <div className="px-6 py-2">
            <p className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-body-soft">Example</p>
            <pre className="overflow-auto rounded-md border border-ui-border/70 bg-ui-bg-muted/30 p-3 text-xs text-body">{notice.example}</pre>
          </div>
        ) : null}
        <DialogFooter className="px-6 pb-6">
          <Button type="button" size="sm" onClick={onClose}>OK</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}




export default function Page(input) {
  const navLinks = input?.nav?.links ?? {};
  const suiteTabs = Array.isArray(input?.suite_tabs) ? input.suite_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const preview = input?.preview ?? { columns: [], rows: [], empty: true };
  const connection = input?.connection ?? {};
  const dbApi = input?.db_runtime_api ?? {};
  // What this engine supports, declared by its driver. Every optional panel
  // below is gated on these rather than on the connection's kind name.
  const caps = input?.capabilities ?? {};
  const canCreateTable = caps.create_table === true;
  const canDropTable = caps.drop_table === true;
  const canInlineEdit = caps.inline_edit === true;
  const hasMaintenance = caps.maintenance === true;
  const hasGraphRelations = caps.relations === "graph";
  const hasGeo = caps.geo === true;
  // Schema-definition routes arrive only when the engine supports them, so an
  // absent URL is what disables the surface.
  const schemaApi = input?.db_schema_api ?? {};
  // Table definition, scoped to this connection so it reaches the engine on
  // screen. Absent unless the driver declares create or drop.
  const simpleTablesApi = schemaApi.tables || "";
  // Editing attributes and index kinds after creation is a separate capability
  // and travels its own route.
  const tablePropertiesApi = schemaApi.properties || "";
  const canEditProperties = caps.edit_table_properties === true;
  // The column that addresses one row. Declared by the driver because it
  // differs: sekejap answers `_key`, SQL engines answer their primary key.
  const rowIdentity = String(caps.row_identity || "_key");
  const sekejapSchemaExportApi = tablePropertiesApi ? `${tablePropertiesApi}/schema/export` : "";
  const sekejapSchemaSyncApi = schemaApi.schema_sync || "";
  const sekejapMaintenanceApi = schemaApi.maintenance || "";
  const initialTable = typeof window !== "undefined" ? new URLSearchParams(window.location.search).get("table") || "" : "";

  const [schemas, setSchemas] = useState([]);
  const [tables, setTables] = useState([]);
  const [selectedTable, setSelectedTable] = useState(initialTable);
  const [collapsedSchemas, setCollapsedSchemas] = useState({});
  const [treeError, setTreeError] = useState("");
  const [previewColumns, setPreviewColumns] = useState(Array.isArray(preview?.columns) ? preview.columns : []);
  const [previewRows, setPreviewRows] = useState(Array.isArray(preview?.rows) ? preview.rows : []);
  const [previewError, setPreviewError] = useState("");
  const [querySql, setQuerySql] = useState(String(input?.query_example || ""));
  const [queryStatus, setQueryStatus] = useState("Ready");
  const [queryColumns, setQueryColumns] = useState(Array.isArray(preview?.columns) ? preview.columns : []);
  const [queryRows, setQueryRows] = useState(Array.isArray(preview?.rows) ? preview.rows : []);
  const [schemaColumns, setSchemaColumns] = useState([]);
  const [schemaRows, setSchemaRows] = useState([]);
  const [schemaError, setSchemaError] = useState("");
  const [valueMeta, setValueMeta] = useState("Click a cell to inspect value");
  const [valueBody, setValueBody] = useState("");
  const [inspectedCellRaw, setInspectedCellRaw] = useState(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createBusy, setCreateBusy] = useState(false);
  const [createStatus, setCreateStatus] = useState("Define the table and save it into the project-local sekejap store.");
  const [createTableSlug, setCreateTableSlug] = useState("");
  const [createAttributes, setCreateAttributes] = useState([{ ...DEFAULT_ATTRIBUTE }]);
  const [reloadToken, setReloadToken] = useState(0);
  const [selectedPreviewRowKey, setSelectedPreviewRowKey] = useState("");
  const [selectedPreviewRowData, setSelectedPreviewRowData] = useState(null);
  const [outgoingRelations, setOutgoingRelations] = useState([]);
  const [incomingRelations, setIncomingRelations] = useState([]);
  const [relationsBusy, setRelationsBusy] = useState(false);
  const [relationsError, setRelationsError] = useState("");
  const [outgoingRelationStats, setOutgoingRelationStats] = useState([]);
  const [incomingRelationStats, setIncomingRelationStats] = useState([]);
  const [relationStatsBusy, setRelationStatsBusy] = useState(false);
  const [relationStatsError, setRelationStatsError] = useState("");
  const [relationCreateOpen, setRelationCreateOpen] = useState(false);
  const [relationCreateBusy, setRelationCreateBusy] = useState(false);
  const [relationCreateStatus, setRelationCreateStatus] = useState("Choose the direction, relation type, and related node slug.");
  const [relationDirection, setRelationDirection] = useState("outgoing");
  const [relationType, setRelationType] = useState("");
  const [relatedNodeSlug, setRelatedNodeSlug] = useState("");
  const [relationTypeOptions, setRelationTypeOptions] = useState([]);
  const [relationTargetSearchOpen, setRelationTargetSearchOpen] = useState(false);
  const [validationNotice, setValidationNotice] = useState(null);
  const [pendingRelationDelete, setPendingRelationDelete] = useState(null);
  const [contentTab, setContentTab] = useState("data");
  const [propsAttributes, setPropsAttributes] = useState([]);
  const [propsBusy, setPropsBusy] = useState(false);
  const [propsStatus, setPropsStatus] = useState("");
  const [schemaSyncBusy, setSchemaSyncBusy] = useState(false);
  const [schemaSyncStatus, setSchemaSyncStatus] = useState("");
  const [maintenanceHealth, setMaintenanceHealth] = useState(null);
  const [maintenanceReport, setMaintenanceReport] = useState(null);
  const [maintenanceBusy, setMaintenanceBusy] = useState(false);
  const [maintenanceStatus, setMaintenanceStatus] = useState("");
  const [pendingMaintenanceAction, setPendingMaintenanceAction] = useState("");
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [deleteConfirmInput, setDeleteConfirmInput] = useState("");
  const [deleteBusy, setDeleteBusy] = useState(false);
  const grouped = groupTablesBySchema(tables);
  const schemaNames = (schemas.length ? schemas : Array.from(grouped.keys())).sort((a, b) => a.localeCompare(b));
  const activeTable = selectedTableDefinition(tables, selectedTable);
  const declaredCols = (activeTable?.attributes || []).map((a) => String(a.name || "")).filter(Boolean);
  const extraCols = declaredCols.filter((name) => !previewColumns.includes(name));
  const rawMergedColumns = [...previewColumns, ...extraCols];
  const rawMergedRows = extraCols.length
    ? previewRows.map((row) => [...(Array.isArray(row) ? row : []), ...extraCols.map(() => null)])
    : previewRows;
  const mergedColumns = orderedDataColumns(rawMergedColumns);
  const mergedRows = reorderRowsForColumns(rawMergedColumns, rawMergedRows, mergedColumns);

  async function loadTreeData(preferredTable = "") {
    const [schemasPayload, tablesPayload] = await Promise.all([
      requestJson(dbApi.schemas),
      requestJson(dbApi.tables),
    ]);
    const nextSchemas = normalizeSchemaNodes(schemasPayload?.result?.nodes);
    const nextTables = normalizeTableNodes(tablesPayload?.result?.nodes);
    setSchemas(nextSchemas);
    setTables(nextTables);
    setTreeError("");

    const requested = String(preferredTable || initialTable || "").trim();
    const first = nextTables[0]?.key || "";
    const target = nextTables.some((item) => item.key === requested) ? requested : first;
    setSelectedTable(target);
  }

  useEffect(() => {
    if (!dbApi.schemas || !dbApi.tables) return;
    let active = true;
    loadTreeData()
      .catch((error) => {
        if (!active) return;
        setSchemas([]);
        setTables([]);
        setTreeError(`Failed to load tables: ${String(error?.message || error)}`);
      });
    return () => {
      active = false;
    };
  }, [dbApi.schemas, dbApi.tables, reloadToken]);

  useEffect(() => {
    loadMaintenanceHealth({ silent: true });
  }, [sekejapMaintenanceApi, reloadToken]);

  async function loadPreviewData(table) {
    if (!dbApi.preview || !table) return { columns: [], rows: [] };
    const url = `${dbApi.preview}?table=${encodeURIComponent(table)}&limit=120`;
    const payload = await requestJson(url);
    const result = payload?.result || {};
    const columns = Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : [];
    const rows = Array.isArray(result?.rows) ? result.rows : [];
    setPreviewColumns(columns);
    setPreviewRows(rows);
    setPreviewError("");
    return { columns, rows };
  }

  useEffect(() => {
    if (!dbApi.preview) return;
    if (!selectedTable) {
      setPreviewColumns([]);
      setPreviewRows([]);
      setPreviewError("");
      if (typeof window !== "undefined") {
        const next = new URL(window.location.href);
        next.searchParams.delete("table");
        window.history.replaceState({}, "", next.toString());
      }
      return;
    }
    let active = true;
    loadPreviewData(selectedTable).catch((error) => {
      if (!active) return;
      setPreviewColumns([]);
      setPreviewRows([]);
      setPreviewError(String(error?.message || error));
    });

    if (typeof window !== "undefined") {
      const next = new URL(window.location.href);
      next.searchParams.set("table", selectedTable);
      window.history.replaceState({}, "", next.toString());
    }

    return () => {
      active = false;
    };
  }, [dbApi.preview, selectedTable]);

  useEffect(() => {
    if (!activeTable || !mergedRows.length) {
      setSelectedPreviewRowKey("");
      setSelectedPreviewRowData(null);
      return;
    }

    const records = mergedRows.map((row) => mapRowToObject(mergedColumns, row));
    const existing = records.find((record) => String(record?.[rowIdentity] ?? "") === selectedPreviewRowKey);
    const chosen = existing || records[0] || null;
    if (!chosen) {
      setSelectedPreviewRowKey("");
      setSelectedPreviewRowData(null);
      return;
    }
    setSelectedPreviewRowKey(String(chosen?.[rowIdentity] ?? ""));
    setSelectedPreviewRowData(chosen);
  }, [activeTable, previewColumns, previewRows, selectedPreviewRowKey]);

  useEffect(() => {
    if (!dbApi.describe || !selectedTable) return;
    let active = true;
    // Sent exactly as the tree named it, qualifier included. An engine that
    // namespaces its tables needs the qualifier to find the right one, and an
    // engine that does not simply drops it.
    requestJson(`${dbApi.describe}?scope=columns&table=${encodeURIComponent(selectedTable)}`)
      .then((payload) => {
        if (!active) return;
        const nodes = Array.isArray(payload?.result?.nodes) ? payload.result.nodes : [];
        const columns = ["field", "type", "nullable", "primary_key", "default"];
        const rows = nodes.map((node) => {
          const meta = node?.meta ?? {};
          return [
            String(node?.name ?? ""),
            meta.type ?? meta.data_type ?? "",
            meta.nullable === undefined ? "" : String(meta.nullable),
            meta.pk === true ? "true" : "false",
            meta.default ?? "",
          ];
        });
        setSchemaColumns(nodes.length ? columns : []);
        setSchemaRows(rows);
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
  }, [dbApi.describe, selectedTable]);

  function onCellInspect(columnName, rowIndex, cellValue) {
    setValueMeta(`${columnName} · row ${rowIndex + 1}`);
    setValueBody(prettyValue(rawCellValue(cellValue)));
    setInspectedCellRaw(cellValue);
  }

  async function runDbQuery(sql, { readOnly = true, tableName = "", limit = 500 } = {}) {
    if (!dbApi.query) {
      throw new Error("Query endpoint is not available");
    }
    const payload = await requestJson(dbApi.query, {
      method: "POST",
      body: JSON.stringify({
        sql,
        read_only: readOnly,
        limit,
        ...(tableName ? { table: tableName } : {}),
      }),
    });
    const result = payload?.result || {};
    const columns = Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : [];
    const rows = Array.isArray(result?.rows) ? result.rows : [];
    return {
      columns,
      rows,
      objects: rows.map((row) => mapRowToObject(columns, row)),
      result,
    };
  }

  async function loadRelationSummaryForTable(tableName) {
    if (!dbApi.query || !tableName) {
      setOutgoingRelationStats([]);
      setIncomingRelationStats([]);
      setRelationStatsError("");
      return;
    }

    setRelationStatsBusy(true);
    setRelationStatsError("");
    try {
      if (!hasGraphRelations) return;
      const show = await runDbQuery("SHOW EDGES", { readOnly: true, tableName, limit: 500 });
      const edgeDefs = uniqueRelationDefs(show.objects);
      const allTypes = edgeDefs
        .map((item) => item.type)
        .filter((item, index, arr) => item && arr.indexOf(item) === index)
        .sort((a, b) => a.localeCompare(b));
      setRelationTypeOptions(allTypes);

      const outgoingDefs = edgeDefs.filter((item) => item.from === tableName);
      const incomingDefs = edgeDefs.filter((item) => item.to === tableName);

      async function withCount(def, direction) {
        const query = direction === "outgoing"
          ? `SELECT COUNT(*) AS count FROM MATCH (a:${def.from})-[:${def.type}]->(b:${def.to})`
          : `SELECT COUNT(*) AS count FROM MATCH (a:${def.from})-[:${def.type}]->(b:${def.to})`;
        try {
          const counted = await runDbQuery(query, { readOnly: true, tableName, limit: 1 });
          return { ...def, direction, count: relationCountFromRows(counted.rows), countError: "" };
        } catch (error) {
          return { ...def, direction, count: null, countError: String(error?.message || error) };
        }
      }

      const [outgoing, incoming] = await Promise.all([
        Promise.all(outgoingDefs.map((def) => withCount(def, "outgoing"))),
        Promise.all(incomingDefs.map((def) => withCount(def, "incoming"))),
      ]);
      setOutgoingRelationStats(outgoing.sort((a, b) => `${a.type}:${a.to}`.localeCompare(`${b.type}:${b.to}`)));
      setIncomingRelationStats(incoming.sort((a, b) => `${a.type}:${a.from}`.localeCompare(`${b.type}:${b.from}`)));
    } catch (error) {
      setOutgoingRelationStats([]);
      setIncomingRelationStats([]);
      setRelationStatsError(String(error?.message || error));
    } finally {
      setRelationStatsBusy(false);
    }
  }

  async function loadRelationsForNode(tableName, record) {
    const nodeKey = String(record?._key || "").trim();
    if (!tableName || !nodeKey) {
      setOutgoingRelations([]);
      setIncomingRelations([]);
      setRelationsError("");
      return;
    }

    setRelationsBusy(true);
    setRelationsError("");
    try {
      if (!hasGraphRelations) return;
      const show = await runDbQuery("SHOW EDGES", { readOnly: true, tableName, limit: 500 });
      const edgeDefs = uniqueRelationDefs(show.objects);
      setRelationTypeOptions(
        edgeDefs
          .map((item) => String(item?.type || "").trim())
          .filter(Boolean)
          .filter((item, index, arr) => arr.indexOf(item) === index)
          .sort((a, b) => a.localeCompare(b))
      );

      const outgoingTypes = edgeDefs
        .filter((item) => String(item?.from || "") === tableName)
        .map((item) => String(item?.type || "").trim())
        .filter(Boolean)
        .filter((item, index, arr) => arr.indexOf(item) === index);

      const incomingEdges = edgeDefs
        .filter((item) => String(item?.to || "") === tableName)
        .map((item) => ({
          from: String(item?.from || "").trim(),
          type: String(item?.type || "").trim(),
        }))
        .filter((item) => item.from && item.type)
        .filter((item, index, arr) => arr.findIndex((other) => other.from === item.from && other.type === item.type) === index);

      const escapedKey = sqlStringLiteral(nodeKey);

      const outgoingLists = await Promise.all(
        outgoingTypes.map(async (type) => {
          const query = `SELECT b._collection AS _collection, b._key AS _key, b.title AS title, b.name AS name, b.post_id AS post_id, b.slug AS slug FROM MATCH (a:${tableName})-[:${type}]->(b) WHERE a._key = '${escapedKey}'`;
          const response = await runDbQuery(query, { readOnly: true, tableName, limit: 200 });
          return response.objects.map((target) => ({
            direction: "outgoing",
            type,
            other: target,
            otherSlug: relationNodeSlug(target, ""),
            otherLabel: relationNodeLabel(target, ""),
          }));
        })
      );

      const incomingLists = await Promise.all(
        incomingEdges.map(async ({ from, type }) => {
          const query = `SELECT a._collection AS _collection, a._key AS _key, a.title AS title, a.name AS name, a.post_id AS post_id, a.slug AS slug FROM MATCH (a:${from})-[:${type}]->(b:${tableName}) WHERE b._key = '${escapedKey}'`;
          const response = await runDbQuery(query, { readOnly: true, tableName, limit: 200 });
          return response.objects.map((source) => ({
            direction: "incoming",
            type,
            other: source,
            otherSlug: relationNodeSlug(source, ""),
            otherLabel: relationNodeLabel(source, ""),
          }));
        })
      );

      setOutgoingRelations(
        outgoingLists
          .flat()
          .filter((item) => item.otherSlug)
          .sort((a, b) => `${a.type}:${a.otherLabel}`.localeCompare(`${b.type}:${b.otherLabel}`))
      );
      setIncomingRelations(
        incomingLists
          .flat()
          .filter((item) => item.otherSlug)
          .sort((a, b) => `${a.type}:${a.otherLabel}`.localeCompare(`${b.type}:${b.otherLabel}`))
      );
    } catch (error) {
      setOutgoingRelations([]);
      setIncomingRelations([]);
      setRelationsError(String(error?.message || error));
    } finally {
      setRelationsBusy(false);
    }
  }

  async function runQuery() {
    if (!dbApi.query) return;
    const sql = String(querySql || "").trim();
    if (!sql) {
      setQueryStatus("Error · Query is empty");
      return;
    }

    const payload = {
      sql,
      read_only: true,
      limit: 1000,
      ...(selectedTable ? { table: selectedTable.split(".").pop() || selectedTable } : {}),
    };

    setQueryStatus("Running...");
    try {
      const response = await requestJson(dbApi.query, {
        method: "POST",
        body: JSON.stringify(payload),
      });
      const result = response?.result || {};
      setQueryColumns(Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : []);
      setQueryRows(Array.isArray(result?.rows) ? result.rows : []);
      setQueryStatus(`OK · rows ${Number(result?.row_count || 0)} · ${Number(result?.duration_ms || 0)} ms`);
    } catch (error) {
      setQueryColumns([]);
      setQueryRows([]);
      setQueryStatus(`Error · ${String(error?.message || error)}`);
    }
  }

  function resetCreateForm(open) {
    setCreateOpen(open);
    if (open) {
      setCreateStatus("Define the table and save it into the project-local sekejap store.");
      setCreateTableSlug("");
      setCreateAttributes([{ ...DEFAULT_ATTRIBUTE }]);
    }
  }

  async function handleCreateTable(event) {
    event?.preventDefault?.();
    const table = String(createTableSlug || "").trim();
    const attributes = (createAttributes || [])
      .map((item) => ({
        name: String(item?.name || "").trim(),
        kind: String(item?.kind || "string"),
        index_types: Array.isArray(item?.index_types) ? item.index_types : [],
      }))
      .filter((item) => item.name);

    if (!table) {
      setCreateStatus("Error · Table slug is required.");
      return;
    }

    setCreateBusy(true);
    setCreateStatus("Creating table…");
    try {
      const payload = await requestJson(simpleTablesApi, {
        method: "POST",
        body: JSON.stringify({
          table,
          attributes,
        }),
      });
      const createdTable = String(payload?.table?.table || table).trim();
      setCreateStatus(`Created · ${createdTable}`);
      setCreateOpen(false);
      await loadTreeData(createdTable);
      setQueryStatus(`Created table '${createdTable}'.`);
    } catch (error) {
      setCreateStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setCreateBusy(false);
    }
  }

  function openRelationDialog() {
    setRelationCreateStatus("Choose the direction, relation type, and related node slug.");
    setRelationDirection("outgoing");
    setRelationType("");
    setRelatedNodeSlug("");
    setRelationCreateOpen(true);
  }

  async function handleCreateRelation(event) {
    event?.preventDefault?.();
    const currentNodeSlug = relationNodeSlug(selectedPreviewRowData, activeTable?.table || "");
    const currentKey = String(selectedPreviewRowData?._key || "").trim();
    const normalizedType = normalizeRelationType(relationType);
    const otherSlug = String(relatedNodeSlug || "").trim();
    const otherParts = relationSlugParts(otherSlug);

    if (!activeTable?.table || !currentNodeSlug || !currentKey) {
      setRelationCreateStatus("Error · Select a concrete row first.");
      return;
    }
    if (!normalizedType) {
      setRelationCreateStatus("Error · Relation type is required.");
      return;
    }
    if (!otherParts) {
      setValidationNotice({
        title: "Invalid Node Slug",
        message: "Related node slug must use collection/key format.",
        example: "people/alice",
      });
      setRelationCreateStatus("Error · Related node slug must use collection/key.");
      return;
    }
    if (!tables.some((table) => table.table === otherParts.collection)) {
      setValidationNotice({
        title: "Unknown Collection",
        message: `Collection '${otherParts.collection}' does not exist in this Sekejap store. Choose an existing collection before creating the relation.`,
        example: `${activeTable.table}/${currentKey}`,
      });
      setRelationCreateStatus(`Error · Unknown collection '${otherParts.collection}'.`);
      return;
    }

    const fromSlug = relationDirection === "outgoing" ? currentNodeSlug : otherSlug;
    const toSlug = relationDirection === "outgoing" ? otherSlug : currentNodeSlug;
    const sql = `INSERT ('${sqlStringLiteral(fromSlug)}')-[:${normalizedType}]->('${sqlStringLiteral(toSlug)}')`;

    setRelationCreateBusy(true);
    setRelationCreateStatus("Creating relation…");
    try {
      const exists = await runDbQuery(`SELECT _key FROM ${otherParts.collection} WHERE _key = '${sqlStringLiteral(otherParts.key)}'`, {
        readOnly: true,
        tableName: otherParts.collection,
        limit: 1,
      });
      if (!exists.rows.length) {
        setValidationNotice({
          title: "Target Node Not Found",
          message: `Node '${otherSlug}' does not exist. Search and select an existing node, or create the node first.`,
          example: `${otherParts.collection}/existing_key`,
        });
        setRelationCreateStatus(`Error · Node '${otherSlug}' not found.`);
        return;
      }
      await runDbQuery(sql, { readOnly: false, tableName: activeTable.table, limit: 50 });
      setRelationCreateStatus(`Created · ${normalizedType}`);
      setRelationCreateOpen(false);
      await loadRelationsForNode(activeTable.table, selectedPreviewRowData);
    } catch (error) {
      setRelationCreateStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setRelationCreateBusy(false);
    }
  }

  async function handleDeleteRelation(entry) {
    if (!entry || !activeTable?.table || !selectedPreviewRowData) return;
    const currentNodeSlug = relationNodeSlug(selectedPreviewRowData, activeTable.table);
    if (!currentNodeSlug) return;
    const fromSlug = entry.direction === "outgoing" ? currentNodeSlug : entry.otherSlug;
    const toSlug = entry.direction === "outgoing" ? entry.otherSlug : currentNodeSlug;
    const sql = `DELETE ('${sqlStringLiteral(fromSlug)}')-[:${entry.type}]->('${sqlStringLiteral(toSlug)}')`;
    try {
      await runDbQuery(sql, { readOnly: false, tableName: activeTable.table, limit: 50 });
      await loadRelationsForNode(activeTable.table, selectedPreviewRowData);
    } catch (error) {
      setRelationsError(String(error?.message || error));
    }
  }

  // Sync properties form when active table changes
  useEffect(() => {
    if (!activeTable) return;
    setPropsAttributes(
      (activeTable.attributes || []).map((a) => ({
        name: a.name || "",
        kind: a.kind || "string",
        index_types: Array.isArray(a.index_types) ? [...a.index_types] : [],
      }))
    );
    setPropsStatus("");
    setContentTab("data");
  }, [activeTable?.table, activeTable?.updatedAt]);

  function currentPropertiesPayload() {
    const attrs = (propsAttributes || [])
      .map((item) => ({
        name: String(item?.name || "").trim(),
        kind: String(item?.kind || "string"),
        index_types: Array.isArray(item?.index_types) ? item.index_types : [],
      }))
      .filter((item) => item.name);
    return { attributes: attrs };
  }

  async function saveActiveTableProperties(status = "Saving…") {
    if (!activeTable) return null;
    setPropsBusy(true);
    setPropsStatus(status);
    const payload = currentPropertiesPayload();
    const response = await requestJson(`${tablePropertiesApi}/${encodeURIComponent(activeTable.table)}`, {
      method: "PUT",
      body: JSON.stringify(payload),
    });
    setPropsStatus("Saved");
    setReloadToken((v) => v + 1);
    return response;
  }

  async function handleUpdateTable(event) {
    event?.preventDefault?.();
    if (!activeTable) return;

    try {
      await saveActiveTableProperties("Saving…");
    } catch (error) {
      setPropsStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setPropsBusy(false);
    }
  }

  async function handleSyncSchemaToRepo() {
    setSchemaSyncBusy(true);
    setSchemaSyncStatus(activeTable ? "Saving table and syncing schema…" : "Syncing schema…");
    try {
      if (activeTable) {
        await saveActiveTableProperties("Saving before schema sync…");
      }
      const payload = await requestJson(sekejapSchemaSyncApi, { method: "POST" });
      const sync = payload?.sync || {};
      setSchemaSyncStatus(`Synced · ${Number(sync?.table_count || 0)} tables · ${Number(sync?.files_written?.length || 0)} files`);
      setReloadToken((v) => v + 1);
    } catch (error) {
      setSchemaSyncStatus(`Sync failed · ${String(error?.message || error)}`);
    } finally {
      setSchemaSyncBusy(false);
      setPropsBusy(false);
    }
  }

  function handleDownloadSchema() {
    const a = Object.assign(document.createElement("a"), {
      href: sekejapSchemaExportApi,
      download: `${input?.project || "project"}-sekejap-schema.json`,
    });
    a.click();
  }

  async function loadMaintenanceHealth({ silent = false } = {}) {
    if (!silent) {
      setMaintenanceBusy(true);
      setMaintenanceStatus("Checking store…");
    }
    try {
      const payload = await requestJson(`${sekejapMaintenanceApi}/health`);
      const health = payload?.health || null;
      setMaintenanceHealth(health);
      setMaintenanceStatus(health ? `Checked · ${Number(health?.duration_ms || 0)} ms` : "No health data returned");
      return health;
    } catch (error) {
      setMaintenanceStatus(`Error · ${String(error?.message || error)}`);
      return null;
    } finally {
      if (!silent) setMaintenanceBusy(false);
    }
  }

  async function runMaintenanceOperation(operation) {
    setMaintenanceBusy(true);
    setMaintenanceReport(null);
    setMaintenanceStatus(operation === "compact" ? "Compacting store…" : "Syncing WAL…");
    try {
      const payload = await requestJson(`${sekejapMaintenanceApi}/${operation}`, { method: "POST" });
      const report = payload?.[operation] || null;
      setMaintenanceReport(report);
      setMaintenanceHealth(report?.after || null);
      setMaintenanceStatus(`${operation === "compact" ? "Compacted" : "Synced"} · ${Number(report?.duration_ms || 0)} ms`);
    } catch (error) {
      setMaintenanceStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setMaintenanceBusy(false);
      setPendingMaintenanceAction("");
    }
  }

  async function handleDeleteTable() {
    if (!activeTable) return;
    setDeleteBusy(true);
    try {
      await requestJson(`${simpleTablesApi}/${encodeURIComponent(activeTable.table)}`, {
        method: "DELETE",
      });
      setDeleteConfirmOpen(false);
      setDeleteConfirmInput("");
      setSelectedTable("");
      setReloadToken((v) => v + 1);
    } catch (error) {
      setPropsStatus(`Delete failed · ${String(error?.message || error)}`);
    } finally {
      setDeleteBusy(false);
    }
  }

  const [totalRowCount, setTotalRowCount] = useState(null);
  const [countBusy, setCountBusy] = useState(false);
  const [pendingEdits, setPendingEdits] = useState({});
  const [editingCell, setEditingCell] = useState(null);
  const [mapPickerOpen, setMapPickerOpen] = useState(false);
  const [mapPickerTarget, setMapPickerTarget] = useState(null);
  const hasPendingEdits = Object.keys(pendingEdits).length > 0;

  async function handleRefreshData() {
    if (activeTable) {
      await loadPreviewData(activeTable.table);
      await loadTreeData(activeTable.table);
    } else {
      setReloadToken((v) => v + 1);
    }
    await loadMaintenanceHealth({ silent: true });
  }

  function handleCellEdit(rowKey, colName, newValue) {
    setPendingEdits((prev) => {
      const rowEdits = { ...(prev[rowKey] || {}), [colName]: newValue };
      return { ...prev, [rowKey]: rowEdits };
    });
  }

  function openMapPicker(target) {
    if (!target?.rowKey || !target?.colName) return;
    setMapPickerTarget(target);
    setMapPickerOpen(true);
    setEditingCell(null);
  }

  function handleMapPickerSave(geometry) {
    if (!mapPickerTarget?.rowKey || !mapPickerTarget?.colName) return;
    handleCellEdit(mapPickerTarget.rowKey, mapPickerTarget.colName, JSON.stringify(geometry));
  }

  function handleMapPickerClear() {
    if (!mapPickerTarget?.rowKey || !mapPickerTarget?.colName) return;
    handleCellEdit(mapPickerTarget.rowKey, mapPickerTarget.colName, "");
  }

  async function handleSaveEdits() {
    if (!activeTable || !dbApi.query || !hasPendingEdits) return;
    try {
      for (const edits of Object.values(pendingEdits)) {
        for (const [col, val] of Object.entries(edits || {})) {
          const warning = validateCellEditValue(activeTable, col, val);
          if (warning) {
            setValidationNotice(warning);
            return;
          }
        }
      }
      for (const [rowKey, edits] of Object.entries(pendingEdits)) {
        const setClauses = Object.entries(edits)
          .map(([col, val]) => {
            if (val === null || val === "") return `${col} = NULL`;
            const trimmed = typeof val === "string" ? val.trim() : String(val);
            if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
              try {
                const parsed = JSON.parse(trimmed);
                if (parsed && typeof parsed === "object" && parsed.type && (parsed.coordinates || parsed.geometries)) {
                  return `${col} = ST_GeomFromGeoJSON('${sqlStringLiteral(trimmed)}')`;
                }
              } catch (_) {}
            }
            const num = Number(val);
            if (!isNaN(num) && trimmed !== "") return `${col} = ${num}`;
            return `${col} = '${sqlStringLiteral(val)}'`;
          })
          .join(", ");
        await runDbQuery(
          `UPDATE ${activeTable.table} SET ${setClauses} WHERE ${rowIdentity} = '${sqlStringLiteral(rowKey)}'`,
          { readOnly: false, tableName: activeTable.table, limit: 0 },
        );
      }
      setPendingEdits({});
      setEditingCell(null);
      await loadPreviewData(activeTable.table);
    } catch (error) {
      setQueryStatus(`Save failed · ${String(error?.message || error)}`);
    }
  }

  function handleCancelEdits() {
    setPendingEdits({});
    setEditingCell(null);
  }

  async function handleAddRow() {
    if (!activeTable || !dbApi.query) return;
    try {
      // The insert statement differs by engine, so the driver writes it and
      // answers with the new row's identity.
      if (!simpleTablesApi) throw new Error("This engine cannot add rows");
      const created = await requestJson(
        `${simpleTablesApi}/${encodeURIComponent(selectedTable)}/rows`,
        { method: "POST" },
      );
      const uid = String(created?.identity ?? "");
      const { rows } = await loadPreviewData(activeTable.table);
      await loadTreeData(activeTable.table);
      if (rows.length) {
        const cols = mergedColumns.length ? mergedColumns : (activeTable.attributes || []).map((a) => a.name);
        const keyIdx = cols.indexOf(rowIdentity);
        const match = keyIdx >= 0 ? rows.find((r) => Array.isArray(r) && String(r[keyIdx]) === uid) : rows[rows.length - 1];
        const found = match || rows[rows.length - 1];
        const record = mapRowToObject(cols, Array.isArray(found) ? found : []);
        setSelectedPreviewRowKey(uid);
        setSelectedPreviewRowData(record);
      }
    } catch (error) {
      setQueryStatus(`Insert failed · ${String(error?.message || error)}`);
    }
  }

  async function handleDeleteSelectedRow() {
    if (!activeTable || !selectedPreviewRowData) return;
    const key = String(selectedPreviewRowData?.[rowIdentity] ?? "").trim();
    if (!key) {
      setQueryStatus(`Cannot delete · row has no ${rowIdentity}`);
      return;
    }
    try {
      await runDbQuery(`DELETE FROM ${activeTable.table} WHERE ${rowIdentity} = '${sqlStringLiteral(key)}'`, { readOnly: false, tableName: activeTable.table, limit: 0 });
      setSelectedPreviewRowKey("");
      setSelectedPreviewRowData(null);
      await loadPreviewData(activeTable.table);
      await loadTreeData(activeTable.table);
    } catch (error) {
      setQueryStatus(`Delete failed · ${String(error?.message || error)}`);
    }
  }

  async function handleCountRows() {
    if (!activeTable || !dbApi.query) return;
    setCountBusy(true);
    try {
      const res = await runDbQuery(`SELECT COUNT(*) AS cnt FROM ${activeTable.table}`, { readOnly: true, tableName: activeTable.table, limit: 1 });
      const cnt = Number(res.objects?.[0]?.cnt ?? res.rows?.[0]?.[0] ?? 0);
      setTotalRowCount(cnt);
    } catch {
      setTotalRowCount(null);
    } finally {
      setCountBusy(false);
    }
  }

  function handleExportCsv() {
    if (!mergedColumns.length || !mergedRows.length) return;
    const escCsv = (v) => {
      const s = String(v ?? "");
      return s.includes(",") || s.includes('"') || s.includes("\n") ? `"${s.replace(/"/g, '""')}"` : s;
    };
    const header = mergedColumns.map(escCsv).join(",");
    const body = mergedRows.map((row) => (Array.isArray(row) ? row : []).map((cell) => escCsv(typeof cell === "object" ? JSON.stringify(cell) : cell)).join(",")).join("\n");
    const blob = new Blob([header + "\n" + body], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = Object.assign(document.createElement("a"), { href: url, download: `${activeTable?.table || "export"}.csv` });
    a.click();
    URL.revokeObjectURL(url);
  }

  useEffect(() => { setTotalRowCount(null); }, [selectedTable]);

  const activeTableName = activeTable?.table || selectedTable.split(".").pop() || "";
  const indexCount = activeTable
    ? new Set([
        ...(activeTable.hashIndexed || []),
        ...(activeTable.rangeIndexed || []),
        ...(activeTable.fulltextFields || []),
        ...(activeTable.vectorFields || []),
        ...(activeTable.spatialFields || []),
      ]).size
    : 0;
  const hasInspectedValue = !!String(valueBody || "").trim();
  const selectedNodeSlug = relationNodeSlug(selectedPreviewRowData, activeTable?.table || "");
  const selectedNodeLabel = selectedPreviewRowData ? relationNodeLabel(selectedPreviewRowData, activeTable?.table || "") : "";
  const relatedSlugParts = relationSlugParts(relatedNodeSlug);
  const relatedSlugWarning = relatedNodeSlug.trim() && relatedSlugParts && !tables.some((table) => table.table === relatedSlugParts.collection)
    ? `Unknown collection '${relatedSlugParts.collection}'.`
    : "";

  async function searchRelationTargets(collection, query) {
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

  useEffect(() => {
    if (!activeTable?.table || !selectedPreviewRowData?._key) {
      setOutgoingRelations([]);
      setIncomingRelations([]);
      setRelationTypeOptions([]);
      setRelationsError("");
      return;
    }
    loadRelationsForNode(activeTable.table, selectedPreviewRowData);
  }, [activeTable?.table, selectedPreviewRowData?._key, reloadToken]);

  useEffect(() => {
    if (!activeTable?.table) {
      setOutgoingRelationStats([]);
      setIncomingRelationStats([]);
      setRelationStatsError("");
      return;
    }
    loadRelationSummaryForTable(activeTable.table);
  }, [activeTable?.table, reloadToken]);

  return (
    <>
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu={`Databases / ${connection.slug || "connection"}`}
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={navLinks.db_connections ?? "#"}>Connections</StudioTabLink>
          {suiteTabs.map((item, index) => (
              <StudioTabLink key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} active={item?.classes === "is-active"}>
                {item?.label}
              </StudioTabLink>
            ))}
          </StudioTabNav>

          <section
            className="db-suite-page flex min-h-0 flex-1 flex-col overflow-auto bg-bg"
            data-db-suite="true"
            data-owner={input.owner}
            data-project={input.project}
            data-db-kind={connection.kind ?? ""}
            data-connection-slug={connection.slug ?? ""}
            data-connection-id={connection.id ?? ""}
            data-api-describe={dbApi.describe ?? ""}
            data-api-schemas={dbApi.schemas ?? ""}
            data-api-tables={dbApi.tables ?? ""}
            data-api-functions={dbApi.functions ?? ""}
            data-api-preview={dbApi.preview ?? ""}
            data-api-query={dbApi.query ?? ""}
          >
            <header className="db-suite-header">
              <p className="db-suite-panel-title">{connection.name}</p>
              <span className="project-inline-chip">
                <i className={`zf-devicon ${connection.icon_class || ""}`} aria-hidden="true"></i>
                <span>kind: {connection.kind} | slug: {connection.slug}</span>
              </span>
            </header>
            <section className="db-suite-shell">
              <div className="db-suite-main">
                {tabFlags?.tables ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="db-suite-table-split">
                      <aside className="db-suite-table-list" data-db-suite-object-tree="true">
                        <div className="db-suite-side-actions">
                          <p className="db-suite-side-title">Schemas</p>
                          {canCreateTable ? (
                            <button type="button" className="project-inline-chip project-inline-chip-action" onClick={() => resetCreateForm(true)}>
                              Create Table
                            </button>
                          ) : null}
                        </div>

                        {treeError ? (
                          <div className="db-suite-empty">{treeError}</div>
                        ) : schemaNames.length === 0 ? (
                          <div className="db-suite-empty">No tables available yet.</div>
                        ) : (
                          schemaNames.map((schemaName, index) => {
                            const collapsed = !!collapsedSchemas[schemaName];
                            const items = (grouped.get(schemaName) || []).sort((a, b) => a.key.localeCompare(b.key));
                            return (
                              <section key={`${schemaName}-${index}`} className="db-suite-object-group">
                                <p className="db-suite-object-group-title">
                                  <button
                                    type="button"
                                    className="db-suite-schema-toggle"
                                    onClick={() =>
                                      setCollapsedSchemas((prev) => ({
                                        ...prev,
                                        [schemaName]: !prev[schemaName],
                                      }))
                                    }
                                  >
                                    <span className={cx("db-suite-schema-caret", collapsed ? "is-collapsed" : "")} aria-hidden="true">
                                      <svg viewBox="0 0 12 12" fill="none">
                                        <path d="M2.25 4.5L6 8.25L9.75 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"></path>
                                      </svg>
                                    </span>
                                    <i className="zf-devicon zf-icon-schema" aria-hidden="true"></i>
                                    <span>{schemaName}</span>
                                  </button>
                                </p>
                                <div className={cx("db-suite-object-items", collapsed ? "is-collapsed" : "")}>
                                  {items.map((item, itemIndex) => (
                                    <button
                                      key={`${item.key}-${itemIndex}`}
                                      type="button"
                                      className={cx("db-suite-object-item", item.key === selectedTable ? "is-active" : "")}
                                      onClick={() => {
                                        setSelectedTable(item.key);
                                        setValueMeta("Click a cell to inspect value");
                                        setValueBody("");
                                        setInspectedCellRaw(null);
                                      }}
                                    >
                                      <span className="db-suite-object-row">
                                        <i className="zf-devicon zf-icon-sjtable" aria-hidden="true"></i>
                                        <span>{item.table}</span>
                                      </span>
                                      <span>{item.rowCount || ""}</span>
                                    </button>
                                  ))}
                                </div>
                              </section>
                            );
                          })
                        )}
                      </aside>

                      <div className="db-suite-data-split">
                        <div className="flex min-h-0 flex-col">
                          <div className="flex min-h-0 flex-1 flex-col">
                            <div className="flex items-center justify-between gap-3 border-b border-ui-border/70 bg-ui-bg-muted/30 px-3 py-2">
                              <div className="min-w-0">
                                <p className="truncate text-sm font-medium text-ui-text">
                                  {activeTableName || "No table selected"}
                                </p>
                                <p className="text-xs text-ui-text-soft">
                                  {activeTable ? `${activeTable.schema} schema` : "Choose a table from the left or create a new one."}
                                </p>
                              </div>
                              <div className="flex flex-wrap items-center justify-end gap-2">
                                {activeTable ? (
                                  <div className="flex flex-wrap items-center justify-end gap-2 text-[11px] uppercase tracking-[0.14em] text-ui-text-soft">
                                    <span className="rounded-full border border-ui-border/80 px-2 py-1">{activeTable.rowCount || 0} rows</span>
                                    <span className="rounded-full border border-ui-border/80 px-2 py-1">{Math.max(schemaRows.length, activeTable.attributes.length)} fields</span>
                                    <span className="rounded-full border border-ui-border/80 px-2 py-1">{indexCount} indexed</span>
                                  </div>
                                ) : null}
                                <div className="flex items-center gap-2">
                                  <button
                                    type="button"
                                    className="rounded border border-ui-border px-2 py-1 text-xs font-medium text-ui-text-soft hover:text-ui-text disabled:opacity-50"
                                    disabled={schemaSyncBusy || propsBusy}
                                    onClick={handleSyncSchemaToRepo}
                                  >
                                    Sync schema to repo
                                  </button>
                                  <button
                                    type="button"
                                    className="rounded border border-ui-border px-2 py-1 text-xs font-medium text-ui-text-soft hover:text-ui-text"
                                    onClick={handleDownloadSchema}
                                  >
                                    Download schema
                                  </button>
                                </div>
                              </div>
                            </div>
                            {schemaSyncStatus ? (
                              <div className="border-b border-ui-border/70 px-3 py-1 text-xs text-ui-text-soft">
                                {schemaSyncStatus}
                              </div>
                            ) : null}

                            <div className="flex items-center gap-1 border-b border-ui-border/70 px-3">
                                <button
                                  type="button"
                                  className={cx(
                                    "px-3 py-1.5 text-xs font-medium transition-colors",
                                    contentTab === "data"
                                      ? "border-b-2 border-ui-text text-ui-text"
                                      : "text-ui-text-soft hover:text-ui-text"
                                  )}
                                  onClick={() => setContentTab("data")}
                                >
                                  Data
                                </button>
                                {hasGraphRelations ? (
                                <button
                                  type="button"
                                  disabled={!activeTable}
                                  className={cx(
                                    "px-3 py-1.5 text-xs font-medium transition-colors",
                                    contentTab === "relations"
                                      ? "border-b-2 border-ui-text text-ui-text"
                                      : "text-ui-text-soft hover:text-ui-text",
                                    !activeTable ? "cursor-not-allowed opacity-40" : ""
                                  )}
                                  onClick={() => setContentTab("relations")}
                                >
                                  Relations
                                </button>
                                ) : null}
                                {canEditProperties ? (
                                <button
                                  type="button"
                                  disabled={!activeTable}
                                  className={cx(
                                    "px-3 py-1.5 text-xs font-medium transition-colors",
                                    contentTab === "properties"
                                      ? "border-b-2 border-ui-text text-ui-text"
                                      : "text-ui-text-soft hover:text-ui-text",
                                    !activeTable ? "cursor-not-allowed opacity-40" : ""
                                  )}
                                  onClick={() => setContentTab("properties")}
                                >
                                  Properties
                                </button>
                                ) : null}
                              </div>

                            {contentTab === "relations" && activeTable && hasGraphRelations ? (
                              <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-4">
                                <div className="flex flex-wrap items-start justify-between gap-3">
                                  <div>
                                    <p className="text-sm font-semibold text-ui-text">Relations</p>
                                    <p className="mt-1 text-xs text-ui-text-soft">
                                      Collection-level relation patterns for {activeTable.table}.
                                    </p>
                                  </div>
                                </div>

                                {relationStatsBusy ? <p className="text-xs text-ui-text-soft">Loading relation statistics…</p> : null}
                                {relationStatsError ? <p className="text-xs text-danger">Failed to load relation statistics: {relationStatsError}</p> : null}

                                <div className="grid gap-3 xl:grid-cols-2">
                                  <RelationStatsList
                                    title="Outbound By Type"
                                    items={outgoingRelationStats}
                                    emptyText="No outbound relation types are declared for this collection."
                                    peerKey="to"
                                  />
                                  <RelationStatsList
                                    title="Inbound By Type"
                                    items={incomingRelationStats}
                                    emptyText="No inbound relation types are declared for this collection."
                                    peerKey="from"
                                  />
                                </div>
                              </div>
                            ) : contentTab === "properties" && activeTable && canEditProperties ? (
                              <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-4">
                                <form onSubmit={handleUpdateTable} className="flex flex-col gap-5">
                                  <div className="flex flex-col gap-3">
                                    <div className="flex items-center justify-between">
                                      <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Attributes</p>
                                      <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() =>
                                          setPropsAttributes((prev) => [
                                            ...prev,
                                            { ...DEFAULT_ATTRIBUTE },
                                          ])
                                        }
                                      >
                                        Add Attribute
                                      </Button>
                                    </div>
                                    {propsAttributes.length === 0 ? (
                                      <p className="text-xs text-ui-text-soft">No attributes defined yet.</p>
                                    ) : (
                                      propsAttributes.map((attr, idx) => (
                                        <AttributeEditorRow
                                          key={idx}
                                          item={attr}
                                          onChange={(next) =>
                                            setPropsAttributes((prev) =>
                                              prev.map((a, i) => (i === idx ? next : a))
                                            )
                                          }
                                          onRemove={() =>
                                            setPropsAttributes((prev) =>
                                              prev.filter((_, i) => i !== idx)
                                            )
                                          }
                                        />
                                      ))
                                    )}
                                  </div>

                                  <div className="flex items-center gap-3">
                                    <Button type="submit" size="sm" disabled={propsBusy}>
                                      {propsBusy ? "Saving…" : "Save Changes"}
                                    </Button>
                                    {propsStatus ? (
                                      <span className="text-xs text-ui-text-soft">{propsStatus}</span>
                                    ) : null}
                                  </div>
                                </form>

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
                                        setDeleteConfirmInput("");
                                        setDeleteConfirmOpen(true);
                                      }}
                                    >
                                      Delete Table
                                    </Button>
                                  ) : null}
                                </div>

                                {deleteConfirmOpen ? (
                                  <Dialog open onOpenChange={(v) => { if (!v) setDeleteConfirmOpen(false); }}>
                                    <DialogContent onKeyDown={(e) => e.stopPropagation()}>
                                      <DialogHeader>
                                        <DialogTitle>Delete Table</DialogTitle>
                                      </DialogHeader>
                                      <div className="flex flex-col gap-3 py-2">
                                        <p className="text-sm text-ui-text-soft">
                                          This will permanently delete <strong>{activeTable.table}</strong> and all its rows. Type the table name to confirm.
                                        </p>
                                        <Input
                                          value={deleteConfirmInput}
                                          onInput={(e) => setDeleteConfirmInput(e.currentTarget.value)}
                                          placeholder={activeTable.table}
                                        />
                                      </div>
                                      <DialogFooter>
                                        <Button
                                          type="button"
                                          variant="outline"
                                          size="sm"
                                          onClick={() => setDeleteConfirmOpen(false)}
                                        >
                                          Cancel
                                        </Button>
                                        <Button
                                          type="button"
                                          size="sm"
                                          disabled={deleteConfirmInput !== activeTable.table || deleteBusy}
                                          className="bg-red-600 text-white hover:bg-red-700 disabled:opacity-40"
                                          onClick={handleDeleteTable}
                                        >
                                          {deleteBusy ? "Deleting…" : "Delete"}
                                        </Button>
                                      </DialogFooter>
                                    </DialogContent>
                                  </Dialog>
                                ) : null}
                              </div>
                            ) : (
                            <div className="db-suite-grid-wrap db-suite-grid-editor-wrap">
                              {activeTable ? (
                                <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-ui-border/70 bg-ui-bg-muted/30 px-2 py-1.5">
                                  <button type="button" title="Save changes" className={`flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium disabled:opacity-30 ${hasPendingEdits ? "bg-blue-600 text-white hover:bg-blue-700" : "text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text"}`} disabled={!hasPendingEdits} onClick={handleSaveEdits}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3 w-3"><path d="M13 14H3a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1h7.586a1 1 0 0 1 .707.293l2.414 2.414a1 1 0 0 1 .293.707V13a1 1 0 0 1-1 1Z"/><path d="M5 14V9h6v5M5 2v3h4"/></svg>
                                    Save
                                  </button>
                                  <button type="button" title="Cancel changes" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text disabled:opacity-30" disabled={!hasPendingEdits} onClick={handleCancelEdits}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3 w-3"><path d="m4 4 8 8M12 4l-8 8"/></svg>
                                    Cancel
                                  </button>
                                  <span className="mx-0.5 h-4 w-px bg-ui-border/60" />
                                  <button type="button" title="Add row" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text hover:bg-ui-bg-muted" onClick={handleAddRow}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M8 3v10M3 8h10"/></svg>
                                    Row
                                  </button>
                                  <button type="button" title="Delete selected row" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-red-500 disabled:opacity-30" disabled={!selectedPreviewRowData} onClick={handleDeleteSelectedRow}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M3 8h10"/></svg>
                                    Delete
                                  </button>
                                  <span className="mx-0.5 h-4 w-px bg-ui-border/60" />
                                  <button type="button" title="Export CSV" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text disabled:opacity-30" disabled={!mergedRows.length} onClick={handleExportCsv}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M8 2v8M4 7l4 4 4-4M2 13h12"/></svg>
                                    CSV
                                  </button>
                                  <button type="button" title="Calculate total row count" className={cx("flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium hover:bg-ui-bg-muted", countBusy ? "animate-pulse text-ui-text" : "text-ui-text-soft hover:text-ui-text")} onClick={handleCountRows}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M13 3H3v10h10V3ZM6 6h4M6 8h4M6 10h2"/></svg>
                                    Count
                                  </button>
                                  <button type="button" title="Refresh" className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text" onClick={handleRefreshData}>
                                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M13.5 8A5.5 5.5 0 1 1 8 2.5M13.5 2.5v3h-3"/></svg>
                                    Refresh
                                  </button>
                                  <span className="ml-auto text-[10px] tabular-nums text-ui-text-soft">
                                    {totalRowCount !== null ? `${totalRowCount} rows` : `${mergedRows.length} loaded`}
                                  </span>
                                </div>
                              ) : null}
                              <div className="db-suite-grid-scroll">
                              {!activeTable ? (
                                <div className="flex h-full min-h-[14rem] items-center justify-center text-sm text-ui-text-soft">
                                  Select a table to inspect its data and structure.
                                </div>
                              ) : mergedRows.length ? (
                                <ResizableDataGrid
                                  columns={mergedColumns}
                                  rows={mergedRows}
                                  selectedRowKey={selectedPreviewRowKey}
                                  onRowSelect={(key, record) => {
                                    setSelectedPreviewRowKey(key);
                                    setSelectedPreviewRowData(record);
                                  }}
                                  onCellInspect={onCellInspect}
                                  mapRowToObject={mapRowToObject}
                                  editingCell={canInlineEdit ? editingCell : null}
                                  pendingEdits={canInlineEdit ? pendingEdits : null}
                                  onEditingCellChange={canInlineEdit ? setEditingCell : undefined}
                                  onCellEdit={canInlineEdit ? handleCellEdit : undefined}
                                  vectorFields={activeTable?.vectorFields}
                                  geoFields={hasGeo ? activeTable?.spatialFields : []}
                                  onGeoPick={hasGeo ? openMapPicker : undefined}
                                />
                              ) : (
                                <div className="flex min-h-full flex-col">
                                  <div className="border-b border-ui-border/70 px-3 py-4">
                                    <p className="text-sm font-medium text-ui-text">
                                      {previewError ? "Preview unavailable" : "No rows yet"}
                                    </p>
                                    <p className="mt-1 text-sm text-ui-text-soft">
                                      {previewError
                                        ? `Failed to load preview: ${previewError}`
                                        : "This table exists, but it does not have stored rows yet. The declared structure is still available below."}
                                    </p>
                                    {!previewError ? (
                                      <button type="button" className="mt-3 inline-flex items-center gap-1 rounded border border-ui-border bg-ui-bg px-2.5 py-1.5 text-xs font-medium text-ui-text hover:bg-ui-bg-muted" onClick={handleAddRow}>
                                        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-3.5 w-3.5"><path d="M8 3v10M3 8h10"/></svg>
                                        Add First Row
                                      </button>
                                    ) : null}
                                  </div>
                                  <div className="min-h-0 flex-1 px-3 pt-4">
                                    <p className="mb-3 text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">
                                      Structure
                                    </p>
                                    <StructureTable
                                      activeTable={activeTable}
                                      schemaColumns={schemaColumns}
                                      schemaRows={schemaRows}
                                      schemaError={schemaError}
                                    />
                                  </div>
                                </div>
                              )}
                              </div>
                            </div>
                            )}
                          </div>

                          {contentTab === "data" && activeTable && mergedRows.length ? (
                            <div className="border-t border-ui-border/70 bg-ui-bg-muted/15 px-3 py-3">
                              <div className="mb-3 flex items-center justify-between gap-3">
                                <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">
                                  Structure
                                </p>
                                <a
                                  href={suiteTabs.find((item) => item?.label === "Schema")?.href ?? "#"}
                                  className="text-xs text-ui-text-soft underline-offset-4 hover:text-ui-text hover:underline"
                                >
                                  Open full schema
                                </a>
                              </div>
                              <div className="max-h-56 overflow-auto">
                                <StructureTable
                                  activeTable={activeTable}
                                  schemaColumns={schemaColumns}
                                  schemaRows={schemaRows}
                                  schemaError={schemaError}
                                />
                              </div>
                            </div>
                          ) : null}
                        </div>

                        <aside className="db-suite-value-panel">
                          <div className="db-suite-value-head">{selectedPreviewRowData ? "Node" : hasInspectedValue ? "Value" : "Overview"}</div>
                          <div className="db-suite-value-meta">
                            {selectedPreviewRowData ? selectedNodeSlug || valueMeta : hasInspectedValue ? valueMeta : activeTable ? `${activeTable.schema}.${activeTable.table}` : "Select a table"}
                          </div>
                          {selectedPreviewRowData ? (
                            <div className="flex min-h-0 flex-col gap-4 overflow-y-auto overflow-x-hidden px-3 py-3 text-sm text-ui-text" style={{ wordBreak: "break-word" }}>
                              <div className="flex items-start justify-between gap-3">
                                <div className="min-w-0">
                                  <p className="truncate text-sm font-medium text-ui-text">{selectedNodeLabel || selectedNodeSlug}</p>
                                  <p className="truncate text-xs text-ui-text-soft">{selectedNodeSlug}</p>
                                </div>
                                {hasGraphRelations ? (
                                  <Button type="button" variant="outline" size="sm" onClick={openRelationDialog}>
                                    New Relation
                                  </Button>
                                ) : null}
                              </div>

                              {hasInspectedValue ? (
                                <div className="space-y-2">
                                  <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Selected Value</p>
                                  <p className="text-xs text-ui-text-soft">{valueMeta}</p>
                                  {hasGeo && isGeoJsonGeometry(inspectedCellRaw) ? (
                                    <GeoPreviewMap geometry={inspectedCellRaw} />
                                  ) : null}
                                  <pre className="max-h-32 overflow-y-auto overflow-x-hidden rounded-md border border-ui-border/70 bg-ui-bg-muted/20 p-2 text-xs text-ui-text" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
                                    {valueBody}
                                  </pre>
                                </div>
                              ) : null}

                              {hasGraphRelations ? (
                              <div className="space-y-3">
                                <div className="flex flex-wrap items-center justify-between gap-2">
                                  <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Relations</p>
                                  {relationsBusy ? <span className="text-[11px] text-ui-text-soft">Loading…</span> : null}
                                </div>
                                {relationsError ? <p className="text-xs text-danger">Failed to load relations: {relationsError}</p> : null}
                                <RowRelationList
                                  title="Outgoing"
                                  items={outgoingRelations}
                                  emptyText="No outgoing relations."
                                  onDelete={setPendingRelationDelete}
                                />
                                <RowRelationList
                                  title="Incoming"
                                  items={incomingRelations}
                                  emptyText="No incoming relations."
                                  onDelete={setPendingRelationDelete}
                                />
                              </div>
                              ) : null}

                              {activeTable ? (
                              <div className="grid grid-cols-2 gap-2 text-xs uppercase tracking-[0.12em] text-ui-text-soft">
                                <span>Rows</span>
                                <span className="text-right">{activeTable.rowCount || 0}</span>
                                <span>Fields</span>
                                <span className="text-right">{Math.max(schemaRows.length, activeTable.attributes.length)}</span>
                                <span>Indexes</span>
                                <span className="text-right">{indexCount}</span>
                                <span>Slug</span>
                                <span className="truncate text-right normal-case tracking-normal text-ui-text">{activeTable.table}</span>
                              </div>
                              ) : null}

                              <div className="space-y-2">
                                <p className="text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Fields</p>
                                {schemaRows.length || activeTable?.attributes.length ? (
                                  <div className="flex flex-wrap gap-2">
                                    {(schemaRows.length
                                      ? schemaRows.map((row, index) => ({
                                          name: String(Array.isArray(row) ? row[0] ?? `field_${index + 1}` : `field_${index + 1}`),
                                        }))
                                      : (activeTable?.attributes || []).map((attr) => ({ name: String(attr?.name || "") }))
                                    )
                                      .filter((item) => item.name && !item.name.startsWith("_"))
                                      .map((item) => (
                                        <span key={item.name} className="inline-flex rounded-full border border-ui-border/80 px-2 py-1 text-xs text-ui-text-soft">
                                          {item.name}
                                        </span>
                                      ))}
                                  </div>
                                ) : (
                                  <p className="text-sm text-ui-text-soft">No field metadata available yet.</p>
                                )}
                              </div>

                              {hasGraphRelations ? (
                              <p className="text-xs text-ui-text-soft">
                                Open the Relations tab for collection-level relation statistics.
                              </p>
                              ) : null}
                            </div>
                          ) : hasInspectedValue ? (
                            <div className="flex min-h-0 flex-col gap-3 overflow-y-auto overflow-x-hidden px-3 py-3">
                              {hasGeo && isGeoJsonGeometry(inspectedCellRaw) ? (
                                <GeoPreviewMap geometry={inspectedCellRaw} />
                              ) : null}
                              <pre className="db-suite-value-body" style={{ margin: 0 }}>{valueBody}</pre>
                            </div>
                          ) : (
                            <pre className="db-suite-value-body">Choose a table from the left to inspect its data and structure.</pre>
                          )}
                        </aside>
                      </div>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.query ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="db-suite-query-split">
                      <div className="db-suite-query-top">
                        <div className="db-suite-query-toolbar">
                          <button type="button" className="project-inline-chip project-inline-chip-action" onClick={runQuery}>
                            Run Query
                          </button>
                          <p className="db-suite-query-status">{queryStatus}</p>
                        </div>
                        <Textarea
                          className="db-suite-query-editor-host"
                          value={querySql}
                          onInput={(event) => setQuerySql(event?.target?.value || "")}
                          rows={10}
                        />
                      </div>

                      <div className="db-suite-query-bottom">
                        <div className="db-suite-grid-wrap">
                          <StudioTable variant="dbGrid">
                            <StudioThead>
                              <tr>
                                {queryColumns.map((col, index) => (
                                  <StudioTh key={`qcol-${col}-${index}`}>{col}</StudioTh>
                                ))}
                              </tr>
                            </StudioThead>
                            <tbody>
                              {queryRows.map((row, rowIndex) => (
                                <tr key={`qrow-${rowIndex}`}>
                                  {(Array.isArray(row) ? row : []).map((cell, cellIndex) => {
                                    const colName = queryColumns[cellIndex] || `column_${cellIndex + 1}`;
                                    return (
                                      <StudioTd key={`qcell-${rowIndex}-${cellIndex}`} onClick={() => onCellInspect(colName, rowIndex, cell)}>
                                        {displayCellText(cell, colName, activeTable?.vectorFields)}
                                      </StudioTd>
                                    );
                                  })}
                                </tr>
                              ))}
                              {!queryRows.length ? (
                                <tr>
                                  <StudioTd colSpan={Math.max(queryColumns.length, 1)}>No rows available</StudioTd>
                                </tr>
                              ) : null}
                            </tbody>
                          </StudioTable>
                        </div>
                      </div>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.graph ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-6 py-10 text-center">
                      <p className="text-base font-semibold text-ui-text">Interactive Graph Inserter</p>
                      <p className="max-w-2xl text-sm text-ui-text-soft">
                        This workspace will let users draft multiple Sekejap nodes and native edges on a canvas, validate the graph, then commit it as one transaction.
                      </p>
                      <Button type="button" variant="outline" size="sm" disabled>
                        Insert Graph
                      </Button>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.schema ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="flex h-full flex-col gap-4 p-6">
                      {!activeTable ? (
                        <div className="db-suite-empty">Select a table to inspect its schema.</div>
                      ) : (
                        <>
                          <div className="flex items-center justify-between gap-3">
                            <div>
                              <p className="text-lg font-semibold text-ui-text">{activeTable.table}</p>
                              <p className="text-sm text-ui-text-soft">{activeTable.rowCount || 0} rows</p>
                            </div>
                            {canCreateTable ? (
                              <Button type="button" variant="outline" size="sm" onClick={() => resetCreateForm(true)}>
                                Create Table
                              </Button>
                            ) : null}
                          </div>

                          <StructureTable
                            activeTable={activeTable}
                            schemaColumns={schemaColumns}
                            schemaRows={schemaRows}
                            schemaError={schemaError}
                          />
                        </>
                      )}
                    </div>
                  </section>
                ) : null}

                {tabFlags?.mart ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <div className="db-suite-mart-full">
                      <StudioTable>
                        <StudioThead>
                          <tr>
                            <StudioTh>Name</StudioTh>
                            <StudioTh>Description</StudioTh>
                            <StudioTh>Status</StudioTh>
                          </tr>
                        </StudioThead>
                        <tbody>
                          <tr>
                            <StudioTd>mart_sales_daily</StudioTd>
                            <StudioTd>Daily aggregated sales mart</StudioTd>
                            <StudioTd>draft</StudioTd>
                          </tr>
                          <tr>
                            <StudioTd>mart_retention_cohort</StudioTd>
                            <StudioTd>User retention cohort mart</StudioTd>
                            <StudioTd>draft</StudioTd>
                          </tr>
                        </tbody>
                      </StudioTable>
                    </div>
                  </section>
                ) : null}

                {tabFlags?.maintenance && hasMaintenance ? (
                  <section className="db-suite-panel db-suite-panel-fill">
                    <MaintenancePanel
                      health={maintenanceHealth}
                      report={maintenanceReport}
                      busy={maintenanceBusy}
                      status={maintenanceStatus}
                      onRefresh={() => loadMaintenanceHealth()}
                      onSync={() => runMaintenanceOperation("sync")}
                      onCompact={() => setPendingMaintenanceAction("compact")}
                    />
                  </section>
                ) : null}
              </div>
            </section>
          </section>
        </div>
        {canCreateTable ? (
        <CreateTableDialog
          open={createOpen}
          onOpenChange={setCreateOpen}
          tableSlug={createTableSlug}
          setTableSlug={setCreateTableSlug}
          attributes={createAttributes}
          setAttributes={setCreateAttributes}
          status={createStatus}
          busy={createBusy}
          onSubmit={handleCreateTable}
        />
        ) : null}
        {hasGraphRelations ? (
        <RelationDialog
          open={relationCreateOpen}
          onOpenChange={setRelationCreateOpen}
          busy={relationCreateBusy}
          status={relationCreateStatus}
          direction={relationDirection}
          setDirection={setRelationDirection}
          relationType={relationType}
          setRelationType={setRelationType}
          relatedNodeSlug={relatedNodeSlug}
          setRelatedNodeSlug={setRelatedNodeSlug}
          currentNodeSlug={selectedNodeSlug}
          relationTypeOptions={relationTypeOptions}
          relatedSlugWarning={relatedSlugWarning}
          onOpenTargetSearch={() => setRelationTargetSearchOpen(true)}
          onSubmit={handleCreateRelation}
        />
        ) : null}
        {hasGraphRelations ? (
        <RelationTargetSearchDialog
          open={relationTargetSearchOpen}
          onOpenChange={setRelationTargetSearchOpen}
          tables={tables}
          onSearch={searchRelationTargets}
          onSelect={setRelatedNodeSlug}
        />
        ) : null}
        <MapPicker
          open={mapPickerOpen}
          onOpenChange={setMapPickerOpen}
          value={mapPickerTarget?.value}
          title={mapPickerTarget?.colName ? `Pick Geometry · ${mapPickerTarget.colName}` : "Pick Geometry"}
          onSave={handleMapPickerSave}
          onClear={handleMapPickerClear}
        />
        <DataWarningDialog
          notice={validationNotice}
          onClose={() => setValidationNotice(null)}
        />
        <ConfirmDialog
          open={!!pendingRelationDelete}
          onClose={() => setPendingRelationDelete(null)}
          onConfirm={() => {
            if (pendingRelationDelete) {
              handleDeleteRelation(pendingRelationDelete);
            }
          }}
          title="Delete Relation"
          message={
            pendingRelationDelete
              ? `Delete relation ${pendingRelationDelete.type} between ${selectedNodeSlug} and ${pendingRelationDelete.otherSlug}?`
              : ""
          }
          confirmLabel="Delete"
          variant="destructive"
        />
        <ConfirmDialog
          open={pendingMaintenanceAction === "compact"}
          onClose={() => setPendingMaintenanceAction("")}
          onConfirm={() => runMaintenanceOperation("compact")}
          title="Compact Sekejap Store"
          message="Compact the project-local Sekejap store now? This checkpoints the snapshot and truncates WAL replay data. Run it during low-traffic windows for large stores."
          confirmLabel="Compact"
        />
      </ProjectStudioShell>
    </>
  );
}
