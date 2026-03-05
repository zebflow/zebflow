import ProjectStudioShell from "@/components/layout/project-studio-shell";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-suite.css" },
      { rel: "stylesheet", href: "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css" },
    ],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "h-screen overflow-hidden bg-slate-950 text-slate-100 font-sans",
  },
  navigation: "history",
};

export const app = {};

function cx(...parts) {
  return parts.filter(Boolean).join(" ");
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
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const message = payload?.error?.message || payload?.message || `${response.status} ${response.statusText}`;
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
      };
    })
    .filter((item) => item.schema && item.table && !item.schema.startsWith("_"))
    .sort((a, b) => a.key.localeCompare(b.key));
}

function stringifyCell(cell) {
  if (cell === null || typeof cell === "undefined") return "";
  if (typeof cell === "string") return cell;
  if (typeof cell === "number" || typeof cell === "boolean") return String(cell);
  try {
    return JSON.stringify(cell);
  } catch (_) {
    return String(cell);
  }
}

function prettyValue(raw) {
  const text = String(raw || "").trim();
  if (!text) return "";
  if (!text.startsWith("{") && !text.startsWith("[")) return text;
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch (_) {
    return text;
  }
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

export default function Page(input) {
  const navLinks = input?.nav?.links ?? {};
  const suiteTabs = Array.isArray(input?.suite_tabs) ? input.suite_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const preview = input?.preview ?? { columns: [], rows: [], empty: true };
  const connection = input?.connection ?? {};
  const dbApi = input?.db_runtime_api ?? {};
  const initialTable = typeof window !== "undefined" ? new URLSearchParams(window.location.search).get("table") || "" : "";
  const [schemas, setSchemas] = useState([]);
  const [tables, setTables] = useState([]);
  const [selectedTable, setSelectedTable] = useState(initialTable);
  const [collapsedSchemas, setCollapsedSchemas] = useState({});
  const [treeError, setTreeError] = useState("");
  const [previewColumns, setPreviewColumns] = useState(Array.isArray(preview?.columns) ? preview.columns : []);
  const [previewRows, setPreviewRows] = useState(Array.isArray(preview?.rows) ? preview.rows : []);
  const [previewError, setPreviewError] = useState("");
  const [querySql, setQuerySql] = useState(String(input?.query_example || "-- Write SQL and click Run Query."));
  const [queryStatus, setQueryStatus] = useState("Ready");
  const [queryColumns, setQueryColumns] = useState(Array.isArray(preview?.columns) ? preview.columns : []);
  const [queryRows, setQueryRows] = useState(Array.isArray(preview?.rows) ? preview.rows : []);
  const [valueMeta, setValueMeta] = useState("Click a cell to inspect value");
  const [valueBody, setValueBody] = useState("");

  useEffect(() => {
    if (!dbApi.schemas || !dbApi.tables) return;
    let active = true;
    Promise.all([requestJson(dbApi.schemas), requestJson(dbApi.tables)])
      .then(([schemasPayload, tablesPayload]) => {
        if (!active) return;
        const nextSchemas = normalizeSchemaNodes(schemasPayload?.result?.nodes);
        const nextTables = normalizeTableNodes(tablesPayload?.result?.nodes);
        setSchemas(nextSchemas);
        setTables(nextTables);
        setTreeError("");
        const requested = initialTable.trim();
        const first = nextTables[0]?.key || "";
        const target = nextTables.some((item) => item.key === requested) ? requested : first;
        if (target) {
          setSelectedTable(target);
        }
      })
      .catch((error) => {
        if (!active) return;
        setSchemas([]);
        setTables([]);
        setTreeError(`Failed to load tables: ${String(error?.message || error)}`);
      });
    return () => {
      active = false;
    };
  }, [dbApi.schemas, dbApi.tables]);

  useEffect(() => {
    if (!tabFlags?.tables) return;
    if (!dbApi.preview || !selectedTable) return;
    let active = true;
    const url = `${dbApi.preview}?table=${encodeURIComponent(selectedTable)}&limit=120`;
    requestJson(url)
      .then((payload) => {
        if (!active) return;
        const result = payload?.result || {};
        setPreviewColumns(Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : []);
        setPreviewRows(Array.isArray(result?.rows) ? result.rows : []);
        setPreviewError("");
      })
      .catch((error) => {
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
  }, [dbApi.preview, selectedTable, tabFlags?.tables]);

  function onCellInspect(columnName, rowIndex, cellValue) {
    setValueMeta(`${columnName} · row ${rowIndex + 1}`);
    setValueBody(prettyValue(stringifyCell(cellValue)));
  }

  async function runQuery() {
    if (!dbApi.query) return;
    const sql = String(querySql || "").trim();
    if (!sql) {
      setQueryStatus("Error · Query is empty");
      return;
    }
    setQueryStatus("Running...");
    try {
      const payload = await requestJson(dbApi.query, {
        method: "POST",
        body: JSON.stringify({ sql, read_only: true, limit: 1000 }),
      });
      const result = payload?.result || {};
      setQueryColumns(Array.isArray(result?.columns) ? result.columns.map((item) => String(item?.name || "")) : []);
      setQueryRows(Array.isArray(result?.rows) ? result.rows : []);
      setQueryStatus(`OK · rows ${Number(result?.row_count || 0)} · ${Number(result?.duration_ms || 0)} ms`);
    } catch (error) {
      setQueryColumns([]);
      setQueryRows([]);
      setQueryStatus(`Error · ${String(error?.message || error)}`);
    }
  }

  const grouped = groupTablesBySchema(tables);
  const schemaNames = (schemas.length ? schemas : Array.from(grouped.keys())).sort((a, b) => a.localeCompare(b));

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
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href={navLinks.db_connections ?? "#"} className="project-tab-link">Connections</a>
          {suiteTabs.map((item, index) => (
            <a key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} className={cx("project-tab-link", item?.classes)}>{item?.label}</a>
          ))}
        </nav>
        <section className="project-workspace-body db-suite-page" data-db-suite="true"
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
                      <p className="db-suite-side-title">Schemas</p>
                      {treeError ? (
                        <div className="db-suite-empty">{treeError}</div>
                      ) : schemaNames.length === 0 ? (
                        <div className="db-suite-empty">No tables available.</div>
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
                                    onClick={() => setSelectedTable(item.key)}
                                  >
                                    <span className="db-suite-object-row">
                                      <i className="zf-devicon zf-icon-table" aria-hidden="true"></i>
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
                      <div className="db-suite-grid-wrap">
                        <table className="project-table" data-db-suite-table-preview-table="true">
                          <thead>
                            <tr data-db-suite-table-preview-head="true">
                              {previewColumns.map((col, index) => <th key={`${col}-${index}`}>{col}</th>)}
                            </tr>
                          </thead>
                          <tbody data-db-suite-table-preview-body="true">
                            {previewRows.map((row, rowIndex) => (
                              <tr key={`row-${rowIndex}`}>
                                {(Array.isArray(row) ? row : []).map((cell, cellIndex) => {
                                  const colName = previewColumns[cellIndex] || `column_${cellIndex + 1}`;
                                  return (
                                    <td key={`cell-${rowIndex}-${cellIndex}`} onClick={() => onCellInspect(colName, rowIndex, cell)}>
                                      {stringifyCell(cell)}
                                    </td>
                                  );
                                })}
                              </tr>
                            ))}
                            {!previewRows.length ? (
                              <tr>
                                <td colSpan={Math.max(previewColumns.length, 1)}>
                                  {previewError ? `Failed to load table preview: ${previewError}` : "No rows available"}
                                </td>
                              </tr>
                            ) : null}
                          </tbody>
                        </table>
                      </div>
                      <aside className="db-suite-value-panel">
                        <div className="db-suite-value-head">Value</div>
                        <div className="db-suite-value-meta" data-db-suite-value-meta="true">{valueMeta}</div>
                        <pre className="db-suite-value-body" data-db-suite-value-body="true">{valueBody}</pre>
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
                        <button type="button" className="project-inline-chip project-inline-chip-action" data-db-suite-query-run="true" onClick={runQuery}>Run Query</button>
                        <p className="db-suite-query-status" data-db-suite-query-status="true">{queryStatus}</p>
                      </div>
                      <textarea
                        className="db-suite-query-editor-host"
                        data-db-suite-query-editor="true"
                        value={querySql}
                        onInput={(event) => setQuerySql(event?.target?.value || "")}
                      ></textarea>
                    </div>
                    <div className="db-suite-query-bottom">
                      <div className="db-suite-grid-wrap">
                        <table className="project-table">
                          <thead>
                            <tr data-db-suite-query-head="true">
                              {queryColumns.map((col, index) => <th key={`qcol-${col}-${index}`}>{col}</th>)}
                            </tr>
                          </thead>
                          <tbody data-db-suite-query-body="true">
                            {queryRows.map((row, rowIndex) => (
                              <tr key={`qrow-${rowIndex}`}>
                                {(Array.isArray(row) ? row : []).map((cell, cellIndex) => {
                                  const colName = queryColumns[cellIndex] || `column_${cellIndex + 1}`;
                                  return (
                                    <td key={`qcell-${rowIndex}-${cellIndex}`} onClick={() => onCellInspect(colName, rowIndex, cell)}>
                                      {stringifyCell(cell)}
                                    </td>
                                  );
                                })}
                              </tr>
                            ))}
                            {!queryRows.length ? (
                              <tr>
                                <td colSpan={Math.max(queryColumns.length, 1)}>No rows available</td>
                              </tr>
                            ) : null}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.schema ? (
                <section className="db-suite-panel db-suite-panel-fill">
                  <div className="db-suite-empty"></div>
                </section>
              ) : null}

              {tabFlags?.mart ? (
                <section className="db-suite-panel db-suite-panel-fill">
                  <div className="db-suite-mart-full">
                    <table className="project-table">
                      <thead>
                        <tr>
                          <th>Name</th>
                          <th>Description</th>
                          <th>Status</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <td>mart_sales_daily</td>
                          <td>Daily aggregated sales mart</td>
                          <td>draft</td>
                        </tr>
                        <tr>
                          <td>mart_retention_cohort</td>
                          <td>User retention cohort mart</td>
                          <td>draft</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </section>
              ) : null}
            </div>
          </section>
        </section>
      </div>
    </ProjectStudioShell>
  </>
  );
}
