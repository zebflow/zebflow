let editorViewCtor = null;
let basicSetupExt = null;
let oneDarkExt = null;
let codeMirrorRuntimePromise = null;

async function ensureCodeMirrorRuntime() {
  if (editorViewCtor && basicSetupExt && oneDarkExt) {
    return;
  }
  if (codeMirrorRuntimePromise) {
    return codeMirrorRuntimePromise;
  }
  codeMirrorRuntimePromise = (async () => {
    if (typeof window === "undefined") {
      throw new Error("codemirror runtime requires browser window");
    }
    const runtimeUrl = new URL(
      "/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
      window.location.origin
    );
    const runtime = await import(runtimeUrl.href);
    editorViewCtor = runtime.EditorView;
    basicSetupExt = runtime.basicSetup;
    oneDarkExt = runtime.oneDark;
  })();
  return codeMirrorRuntimePromise;
}

function escapeHtml(raw) {
  return String(raw || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

async function requestJson(url, options = {}) {
  const response = await fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  });

  if (response.status === 401) { window.location.href = "/login"; return null; }
  const payload = await response.json().catch(() => null);
  if (!response.ok) {
    const message =
      payload?.error?.message || payload?.message || `${response.status} ${response.statusText}`;
    throw new Error(message);
  }
  return payload;
}

function normalizedTableList(tableNodes) {
  const items = [];
  (tableNodes || []).forEach((node) => {
    if (String(node?.kind || "") !== "table") {
      return;
    }
    const schema = String(node?.schema || "default");
    if (schema.startsWith("_")) {
      return;
    }
    const table = String(node?.name || "");
    if (!table) {
      return;
    }
    const key = schema === "default" ? table : `${schema}.${table}`;
    items.push({
      schema,
      table,
      key,
      rowCount: Number(node?.meta?.row_count || 0),
      iconClass: "zf-icon-table",
    });
  });
  items.sort((a, b) => a.key.localeCompare(b.key));
  return items;
}

function normalizeCellValue(cell) {
  if (cell === null || typeof cell === "undefined") {
    return { display: "", raw: "" };
  }
  if (typeof cell === "string") {
    return { display: cell, raw: cell };
  }
  if (typeof cell === "number" || typeof cell === "boolean") {
    const text = String(cell);
    return { display: text, raw: text };
  }
  try {
    const pretty = JSON.stringify(cell, null, 2);
    return { display: JSON.stringify(cell), raw: pretty || "" };
  } catch (_) {
    const fallback = String(cell);
    return { display: fallback, raw: fallback };
  }
}

function tryPrettyJson(raw) {
  const text = String(raw || "").trim();
  if (!text) {
    return "";
  }
  if (!text.startsWith("{") && !text.startsWith("[")) {
    return text;
  }
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch (_) {
    return text;
  }
}

function bindCellInspectors(state) {
  const metaTarget = state.valueMetaEl;
  const bodyTarget = state.valueBodyEl;
  const root = state.root;
  if (!metaTarget || !bodyTarget) {
    return;
  }

  root.querySelectorAll(".project-table td.is-selected").forEach((cell) => {
    cell.classList.remove("is-selected");
  });

  root.querySelectorAll("[data-db-cell='true']").forEach((cell) => {
    cell.addEventListener("click", () => {
      root.querySelectorAll(".project-table td.is-selected").forEach((node) => {
        node.classList.remove("is-selected");
      });
      cell.classList.add("is-selected");
      const columnName = cell.getAttribute("data-db-col") || "value";
      const rowIndex = Number(cell.getAttribute("data-db-row") || "0");
      const raw = cell.getAttribute("data-db-raw") || "";
      metaTarget.textContent = `${columnName} · row ${rowIndex}`;
      bodyTarget.textContent = tryPrettyJson(raw);
    });
  });
}

function renderRows(state, heads, bodies, result) {
  const columns = Array.isArray(result?.columns)
    ? result.columns.map((column) => String(column?.name || ""))
    : [];
  const rows = Array.isArray(result?.rows) ? result.rows : [];

  heads.forEach((head) => {
    if (!head) {
      return;
    }
    head.innerHTML = columns.length
      ? columns.map((column) => `<th>${escapeHtml(column)}</th>`).join("")
      : "<th>No Columns</th>";
  });

  bodies.forEach((body) => {
    if (!body) {
      return;
    }
    if (!rows.length) {
      body.innerHTML = `<tr><td colspan="${Math.max(columns.length, 1)}">No rows available</td></tr>`;
      return;
    }

    body.innerHTML = rows
      .map((row, rowIndex) => {
        const cells = Array.isArray(row) ? row : [];
        return `<tr>${cells
          .map((cell, colIndex) => {
            const normalized = normalizeCellValue(cell);
            const columnName = columns[colIndex] || `column_${colIndex + 1}`;
            return `<td data-db-cell="true" data-db-row="${rowIndex + 1}" data-db-col="${escapeHtml(
              columnName
            )}" data-db-raw="${escapeHtml(normalized.raw)}">${escapeHtml(
              normalized.display
            )}</td>`;
          })
          .join("")}</tr>`;
      })
      .join("");
  });

  bindCellInspectors(state);
}

function setQueryStatus(state, message, kind = "neutral") {
  if (!state.queryStatusEl) {
    return;
  }
  state.queryStatusEl.textContent = message;
  state.queryStatusEl.setAttribute("data-state", kind);
}

function setTreeError(state, message) {
  if (!state.treeEl) {
    return;
  }
  state.treeEl.innerHTML = `<p class="db-suite-side-title">Schemas</p><div class="db-suite-empty">${escapeHtml(
    message || "Failed to load schema metadata."
  )}</div>`;
}

async function runQuery(state) {
  if (!state.apiQuery || !state.queryEditor) {
    return;
  }
  const sql = state.queryEditor.state.doc.toString().trim();
  if (!sql) {
    setQueryStatus(state, "Query is empty", "warn");
    return;
  }

  try {
    setQueryStatus(state, "Running...", "busy");
    const payload = await requestJson(state.apiQuery, {
      method: "POST",
      body: JSON.stringify({
        sql,
        read_only: true,
        limit: 1000,
      }),
    });
    const result = payload?.result || {};
    renderRows(state, state.queryHeads, state.queryBodies, result);
    setQueryStatus(
      state,
      `OK · rows ${Number(result?.row_count || 0)} · ${Number(result?.duration_ms || 0)} ms`,
      "ok"
    );
  } catch (err) {
    setQueryStatus(state, `Error · ${String(err?.message || err)}`, "error");
  }
}

function renderTree(state) {
  if (!state.treeEl) {
    return;
  }

  const bySchema = new Map();
  state.tables.forEach((item) => {
    const key = item.schema || "default";
    if (!bySchema.has(key)) {
      bySchema.set(key, []);
    }
    bySchema.get(key).push(item);
  });

  const schemaNames = (state.schemas.length
    ? state.schemas
        .map((node) => String(node?.name || "default"))
        .filter((name) => !name.startsWith("_"))
    : Array.from(bySchema.keys())
  ).sort((a, b) => a.localeCompare(b));

  const sections = schemaNames
    .map((schemaName) => {
      const collapsed = state.collapsedSchemas.has(schemaName);
      const tables = (bySchema.get(schemaName) || []).sort((a, b) => a.key.localeCompare(b.key));
      const tableLinks = tables
        .map((item) => {
          const active = item.key === state.selectedTable ? "is-active" : "";
          const href = `${state.basePath}/tables?table=${encodeURIComponent(item.key)}`;
          return `<a href="${href}" class="db-suite-object-item ${active}" data-db-suite-table="${escapeHtml(
            item.key
          )}"><span class="db-suite-object-row"><i class="zf-devicon ${escapeHtml(
            item.iconClass
          )}" aria-hidden="true"></i><span class="db-suite-object-icon-fallback" aria-hidden="true">[]</span><span>${escapeHtml(
            item.table
          )}</span></span><span>${item.rowCount || ""}</span></a>`;
        })
        .join("");

      return `<section class="db-suite-object-group">
        <p class="db-suite-object-group-title">
          <button type="button" class="db-suite-schema-toggle" data-db-suite-schema="${escapeHtml(
            schemaName
          )}">
            <span class="db-suite-schema-caret ${collapsed ? "is-collapsed" : ""}">v</span>
            <i class="zf-devicon zf-icon-schema" aria-hidden="true"></i>
            <span>${escapeHtml(schemaName)}</span>
          </button>
        </p>
        <div class="db-suite-object-items ${collapsed ? "is-collapsed" : ""}">
          ${tableLinks}
        </div>
      </section>`;
    })
    .join("");

  state.treeEl.innerHTML = `<p class="db-suite-side-title">Schemas</p>${sections}`;

  state.treeEl.querySelectorAll("[data-db-suite-schema]").forEach((button) => {
    button.addEventListener("click", () => {
      const schemaName = button.getAttribute("data-db-suite-schema") || "";
      if (!schemaName) {
        return;
      }
      if (state.collapsedSchemas.has(schemaName)) {
        state.collapsedSchemas.delete(schemaName);
      } else {
        state.collapsedSchemas.add(schemaName);
      }
      renderTree(state);
    });
  });

  state.treeEl.querySelectorAll("[data-db-suite-table]").forEach((link) => {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      const table = link.getAttribute("data-db-suite-table") || "";
      if (!table) {
        return;
      }
      setSelectedTable(state, table, true).catch((err) => {
        console.error("db suite table click failed", err);
      });
    });
  });
}

async function loadPreview(state) {
  if (!state.selectedTable) {
    renderRows(state, state.tableHeads, state.tableBodies, { columns: [], rows: [] });
    return;
  }
  const url = `${state.apiPreview}?table=${encodeURIComponent(state.selectedTable)}&limit=120`;
  const payload = await requestJson(url);
  renderRows(state, state.tableHeads, state.tableBodies, payload?.result || {});
}

async function setSelectedTable(state, table, pushHistory) {
  state.selectedTable = String(table || "");
  if (pushHistory) {
    const next = new URL(window.location.href);
    next.searchParams.set("table", state.selectedTable);
    window.history.replaceState({}, "", next.toString());
  }
  renderTree(state);
  await loadPreview(state);
}

async function loadTreeData(state) {
  const [schemasPayload, tablesPayload] = await Promise.all([
    requestJson(state.apiSchemas),
    requestJson(state.apiTables),
  ]);

  state.schemas = Array.isArray(schemasPayload?.result?.nodes)
    ? schemasPayload.result.nodes
    : [];
  state.tables = normalizedTableList(tablesPayload?.result?.nodes || []);

  const params = new URLSearchParams(window.location.search);
  const requested = String(params.get("table") || "").trim();
  const first = state.tables[0]?.key || "";
  const target = state.tables.some((item) => item.key === requested) ? requested : first;
  await setSelectedTable(state, target, false);
}

async function initQueryEditor(state) {
  if (!state.queryEditorHost) {
    return;
  }
  await ensureCodeMirrorRuntime();
  const initial = (state.queryEditorHost.textContent || "").trim() || "-- Write SQL and click Run Query.";
  state.queryEditorHost.textContent = "";
  state.queryEditor = new editorViewCtor({
    doc: initial,
    extensions: [basicSetupExt, oneDarkExt],
    parent: state.queryEditorHost,
  });
  setQueryStatus(state, "Ready", "neutral");

  if (state.queryRunEl) {
    state.queryRunEl.addEventListener("click", () => {
      runQuery(state).catch((err) => {
        setQueryStatus(state, `Error · ${String(err?.message || err)}`, "error");
      });
    });
  }
}

async function initDbSuite(root) {
  const apiSchemas = root.getAttribute("data-api-schemas") || "";
  const apiTables = root.getAttribute("data-api-tables") || "";
  const apiPreview = root.getAttribute("data-api-preview") || "";
  const apiQuery = root.getAttribute("data-api-query") || "";
  const owner = root.getAttribute("data-owner") || "";
  const project = root.getAttribute("data-project") || "";
  const dbKind = root.getAttribute("data-db-kind") || "";
  const connectionSlug = root.getAttribute("data-connection-slug") || "";

  if (!apiSchemas || !apiTables || !apiPreview) {
    return;
  }

  const state = {
    apiSchemas,
    apiTables,
    apiPreview,
    apiQuery,
    basePath: `/projects/${encodeURIComponent(owner)}/${encodeURIComponent(
      project
    )}/db/${encodeURIComponent(dbKind)}/${encodeURIComponent(connectionSlug)}`,
    root,
    treeEl: root.querySelector("[data-db-suite-object-tree]"),
    tableHeads: Array.from(root.querySelectorAll("[data-db-suite-table-preview-head]")),
    tableBodies: Array.from(root.querySelectorAll("[data-db-suite-table-preview-body]")),
    queryHeads: Array.from(root.querySelectorAll("[data-db-suite-query-head]")),
    queryBodies: Array.from(root.querySelectorAll("[data-db-suite-query-body]")),
    queryEditorHost: root.querySelector("[data-db-suite-query-editor]"),
    queryRunEl: root.querySelector("[data-db-suite-query-run]"),
    queryStatusEl: root.querySelector("[data-db-suite-query-status]"),
    valueMetaEl: root.querySelector("[data-db-suite-value-meta]"),
    valueBodyEl: root.querySelector("[data-db-suite-value-body]"),
    queryEditor: null,
    schemas: [],
    tables: [],
    selectedTable: "",
    collapsedSchemas: new Set(),
  };

  try {
    await initQueryEditor(state);
    await loadTreeData(state);
  } catch (err) {
    console.error("db suite runtime load failed", err);
    setTreeError(state, `Failed to load tables: ${String(err?.message || err)}`);
    setQueryStatus(state, `Error · ${String(err?.message || err)}`, "error");
  }
}

const initializedRoots = new WeakSet();
let scanScheduled = false;

function scanDbSuiteRoots() {
  document.querySelectorAll("[data-db-suite='true']").forEach((root) => {
    if (initializedRoots.has(root)) {
      return;
    }
    initializedRoots.add(root);
    initDbSuite(root).catch((err) => {
      console.error("postgres db suite runtime init failed", err);
    });
  });
}

export function initProjectDbSuitePostgresqlBehavior() {
  if (typeof Deno !== "undefined") {
    return;
  }
  if (typeof document === "undefined") {
    return;
  }
  if (scanScheduled) {
    return;
  }
  scanScheduled = true;
  const run = () => {
    scanScheduled = false;
    scanDbSuiteRoots();
  };
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(run);
  } else {
    setTimeout(run, 0);
  }
}
