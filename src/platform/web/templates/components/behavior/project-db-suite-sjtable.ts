let editorViewCtor: (new (...args: any[]) => any) | null = null;
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
    const attributes = Array.isArray(node?.meta?.attributes) ? node.meta.attributes : [];
    const hashIndexed = Array.isArray(node?.meta?.hash_indexed_fields) ? node.meta.hash_indexed_fields : [];
    const rangeIndexed = Array.isArray(node?.meta?.range_indexed_fields) ? node.meta.range_indexed_fields : [];
    const fulltextFields = Array.isArray(node?.meta?.fulltext_fields) ? node.meta.fulltext_fields : [];
    const vectorFields = Array.isArray(node?.meta?.vector_fields) ? node.meta.vector_fields : [];
    const spatialFields = Array.isArray(node?.meta?.spatial_fields) ? node.meta.spatial_fields : [];
    items.push({
      schema,
      table,
      key,
      rowCount: Number(node?.meta?.row_count || 0),
      iconClass: "zf-icon-table",
      attributes,
      hashIndexed,
      rangeIndexed,
      fulltextFields,
      vectorFields,
      spatialFields,
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

function setCreateStatus(state, message, kind = "neutral") {
  if (!state.createStatusEl) {
    return;
  }
  state.createStatusEl.textContent = message;
  if (!kind || kind === "neutral") {
    state.createStatusEl.removeAttribute("data-state");
  } else {
    state.createStatusEl.setAttribute("data-state", kind);
  }
}

function openCreateDialog(state) {
  if (!state.createModalEl) {
    return;
  }
  state.createModalEl.classList.remove("is-hidden");
  setCreateStatus(state, "");
  const tableInput = state.createFormEl?.elements?.table;
  if (tableInput && typeof tableInput.focus === "function") {
    tableInput.focus();
  }
}

function closeCreateDialog(state) {
  if (!state.createModalEl) {
    return;
  }
  state.createModalEl.classList.add("is-hidden");
  state.createFormEl?.reset();
  setCreateStatus(state, "");
}

// Index options available per data kind
const KIND_INDEX_OPTIONS = {
  string:  [{ id: "hash", label: "Exact" }, { id: "range", label: "Range" }, { id: "fulltext", label: "Fulltext" }],
  number:  [{ id: "hash", label: "Exact" }, { id: "range", label: "Range" }],
  boolean: [{ id: "hash", label: "Exact" }],
  text:    [{ id: "fulltext", label: "Fulltext" }],
  json:    [],
  vector:  [{ id: "vector", label: "Index" }],
  geo:     [{ id: "spatial", label: "Index" }],
};

function rebuildIndexOptions(row, kind) {
  const container = row.querySelector("[data-db-attr-indexes]");
  if (!container) return;
  const options = KIND_INDEX_OPTIONS[kind] || [];
  // preserve currently checked ids
  const checked = new Set();
  container.querySelectorAll("input[type=checkbox]").forEach(function(cb) {
    if (cb.checked) checked.add(cb.getAttribute("data-db-attr-idx"));
  });
  if (options.length === 0) {
    container.innerHTML = '<span class="db-suite-attr-no-idx">no index</span>';
    return;
  }
  container.innerHTML = options.map(function(opt) {
    const isChecked = checked.has(opt.id) ? " checked" : "";
    return '<label class="db-suite-attr-idx-label" title="' + opt.id + '">' +
      '<input type="checkbox" data-db-attr-idx="' + opt.id + '"' + isChecked + ' />' +
      '<span>' + opt.label + '</span></label>';
  }).join("");
}

function addAttributeRow(container) {
  const row = document.createElement("div");
  row.className = "db-suite-attr-row";
  row.innerHTML =
    '<input class="db-suite-attr-name" data-db-attr-name placeholder="field_name" />' +
    '<select class="db-suite-attr-kind" data-db-attr-kind>' +
    '<option value="string">string</option>' +
    '<option value="number">number</option>' +
    '<option value="boolean">boolean</option>' +
    '<option value="text">text</option>' +
    '<option value="json">json</option>' +
    '<option value="vector">vector</option>' +
    '<option value="geo">geo</option>' +
    '</select>' +
    '<div class="db-suite-attr-indexes" data-db-attr-indexes></div>' +
    '<button type="button" class="db-suite-attr-remove" data-db-attr-remove aria-label="Remove">x</button>';
  const kindEl = row.querySelector("[data-db-attr-kind]");
  const removeBtn = row.querySelector("[data-db-attr-remove]");
  if (removeBtn) {
    removeBtn.addEventListener("click", function() { row.remove(); });
  }
  if (kindEl) {
    kindEl.addEventListener("change", function() {
      rebuildIndexOptions(row, kindEl.value);
    });
  }
  container.appendChild(row);
  rebuildIndexOptions(row, "string");
  const nameInput = row.querySelector("[data-db-attr-name]");
  if (nameInput && typeof (nameInput as any).focus === "function") {
    (nameInput as any).focus();
  }
}

function collectAttributes(container) {
  const attributes = [];
  if (!container) {
    return { attributes };
  }
  container.querySelectorAll(".db-suite-attr-row").forEach(function(row) {
    const nameEl = row.querySelector("[data-db-attr-name]") as any;
    const kindEl = row.querySelector("[data-db-attr-kind]") as any;
    const name = String(nameEl?.value || "").trim();
    if (!name) return;
    const kind = String(kindEl?.value || "string");
    const index_types: string[] = [];
    row.querySelectorAll("input[data-db-attr-idx]").forEach(function(cb: any) {
      if (cb.checked) index_types.push(cb.getAttribute("data-db-attr-idx"));
    });
    attributes.push({ name, kind, index_types });
  });
  return { attributes };
}

async function handleCreateTable(state) {
  if (!state.apiSimpleTables || !state.createFormEl) {
    return;
  }
  const form = state.createFormEl;
  const table = String(form.elements.table?.value || "").trim();
  const title = String(form.elements.title?.value || "").trim();
  const attrsContainer = form.querySelector("[data-db-suite-attrs]");
  const { attributes } = collectAttributes(attrsContainer);

  if (!table) {
    setCreateStatus(state, "Table slug is required.", "error");
    return;
  }

  try {
    setCreateStatus(state, "Creating table...", "busy");
    const payload = await requestJson(state.apiSimpleTables, {
      method: "POST",
      body: JSON.stringify({ table, title: title || null, attributes }),
    });
    const createdTable = String(payload?.table?.table || table).trim();
    await loadTreeData(state, createdTable);
    closeCreateDialog(state);
    setQueryStatus(state, `Created table '${createdTable}'.`, "ok");
  } catch (err) {
    setCreateStatus(state, `Error · ${String(err?.message || err)}`, "error");
  }
}

async function runQuery(state) {
  if (!state.apiQuery || !state.queryEditor) {
    return;
  }
  const raw = state.queryEditor.state.doc.toString().trim();
  const isSjtable = state.dbKind === "sjtable";

  let payloadBody = {
    sql: raw,
    read_only: true,
    limit: 1000,
  };

  if (isSjtable) {
    if (!state.selectedTable) {
      setQueryStatus(state, "Select a table first.", "warn");
      return;
    }
    // Allow direct Sekejap JSON query; default to selected-table query if editor is empty/comment.
    const isJson = raw.startsWith("{") || raw.startsWith("[");
    if (isJson) {
      payloadBody = {
        sql: raw,
        table: state.selectedTable.split(".").pop() || state.selectedTable,
        read_only: true,
        limit: 1000,
      };
    } else {
      const defaultSql = `SELECT * FROM ${state.selectedTable} LIMIT 200`;
      const sql = !raw || raw.startsWith("--") ? defaultSql : raw;
      payloadBody = {
        sql,
        table: state.selectedTable.split(".").pop() || state.selectedTable,
        read_only: true,
        limit: 1000,
      };
    }
  } else if (!raw) {
    setQueryStatus(state, "Query is empty", "warn");
    return;
  }

  try {
    setQueryStatus(state, "Running...", "busy");
    const payload = await requestJson(state.apiQuery, {
      method: "POST",
      body: JSON.stringify(payloadBody),
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

  state.treeEl.innerHTML = `<div class="db-suite-side-actions">
      <p class="db-suite-side-title">Schemas</p>
      <button type="button" class="project-inline-chip project-inline-chip-action" data-db-suite-create-open="true">+ Create Table</button>
    </div>${sections}`;

  state.treeEl.querySelectorAll("[data-db-suite-create-open]").forEach((button) => {
    button.addEventListener("click", () => openCreateDialog(state));
  });

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

function renderSchemaPanel(state) {
  if (!state.schemaPanelEl) {
    return;
  }
  if (!state.selectedTable) {
    state.schemaPanelEl.innerHTML = '<div class="db-suite-schema-hint">Select a table from the tree to view its schema.</div>';
    return;
  }
  const tableItem = state.tables.find(function(t) { return t.key === state.selectedTable; });
  if (!tableItem) {
    state.schemaPanelEl.innerHTML = '<div class="db-suite-schema-hint">Table schema not available.</div>';
    return;
  }
  const attrs = tableItem.attributes || [];
  const hashSet = new Set(tableItem.hashIndexed || []);
  const rangeSet = new Set(tableItem.rangeIndexed || []);
  let html = '<div class="db-suite-schema-head">';
  html += '<span class="db-suite-schema-table-name">' + escapeHtml(tableItem.table) + '</span>';
  html += '<span class="db-suite-schema-row-count">' + (tableItem.rowCount || 0) + ' rows</span>';
  html += '</div>';
  const fulltextSet = new Set(tableItem.fulltextFields || []);
  const vectorSet = new Set(tableItem.vectorFields || []);
  const spatialSet = new Set(tableItem.spatialFields || []);
  if (attrs.length === 0) {
    html += '<div class="db-suite-schema-hint">No attribute schema defined. Dynamic fields accepted.</div>';
  } else {
    html += '<table class="db-suite-schema-table"><thead><tr><th>Name</th><th>Kind</th><th>Indexes</th></tr></thead><tbody>';
    attrs.forEach(function(attr) {
      const idxBadges: string[] = [];
      if (hashSet.has(attr.name)) idxBadges.push('<span class="db-suite-schema-idx">exact</span>');
      if (rangeSet.has(attr.name)) idxBadges.push('<span class="db-suite-schema-idx">range</span>');
      if (fulltextSet.has(attr.name)) idxBadges.push('<span class="db-suite-schema-idx db-suite-schema-idx-ft">fulltext</span>');
      if (vectorSet.has(attr.name)) idxBadges.push('<span class="db-suite-schema-idx db-suite-schema-idx-vec">vector</span>');
      if (spatialSet.has(attr.name)) idxBadges.push('<span class="db-suite-schema-idx db-suite-schema-idx-geo">geo</span>');
      html += '<tr>';
      html += '<td>' + escapeHtml(attr.name) + '</td>';
      html += '<td><span class="db-suite-schema-kind">' + escapeHtml(attr.kind || 'string') + '</span></td>';
      html += '<td>' + (idxBadges.length ? idxBadges.join(' ') : '<span class="db-suite-schema-no-idx">—</span>') + '</td>';
      html += '</tr>';
    });
    html += '</tbody></table>';
  }
  state.schemaPanelEl.innerHTML = html;
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
  renderSchemaPanel(state);
  await loadPreview(state);
}

async function loadTreeData(state, preferredTable: string | null = null) {
  const [schemasPayload, tablesPayload] = await Promise.all([
    requestJson(state.apiSchemas),
    requestJson(state.apiTables),
  ]);

  state.schemas = Array.isArray(schemasPayload?.result?.nodes)
    ? schemasPayload.result.nodes
    : [];
  state.tables = normalizedTableList(tablesPayload?.result?.nodes || []);

  const params = new URLSearchParams(window.location.search);
  const requested = String(preferredTable || params.get("table") || "").trim();
  const first = state.tables[0]?.key || "";
  const target = state.tables.some((item) => item.key === requested) ? requested : first;
  await setSelectedTable(state, target, false);
}

async function initQueryEditor(state) {
  if (!state.queryEditorHost) {
    return;
  }
  await ensureCodeMirrorRuntime();
  const initial = (state.queryEditorHost.textContent || "").trim() || `{
  "pipeline": [
    { "op": "collection", "name": "sjtable__your_table" },
    { "op": "take", "n": 200 }
  ]
}`;
  state.queryEditorHost.textContent = "";
  if (!editorViewCtor) return;
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

function initCreateTableDialog(state) {
  if (!state.createModalEl || !state.createFormEl) {
    return;
  }

  state.createOpenEls.forEach((button) => {
    button.addEventListener("click", () => openCreateDialog(state));
  });

  state.createCancelEls.forEach((button) => {
    button.addEventListener("click", () => closeCreateDialog(state));
  });

  state.createModalEl.addEventListener("click", (event) => {
    if (event.target === state.createModalEl) {
      closeCreateDialog(state);
    }
  });

  const attrAddBtn = state.createFormEl.querySelector("[data-db-suite-attr-add]");
  const attrsContainer = state.createFormEl.querySelector("[data-db-suite-attrs]");
  if (attrAddBtn && attrsContainer) {
    attrAddBtn.addEventListener("click", function() {
      addAttributeRow(attrsContainer);
    });
  }

  state.createFormEl.addEventListener("submit", (event) => {
    event.preventDefault();
    handleCreateTable(state).catch((err) => {
      setCreateStatus(state, `Error · ${String(err?.message || err)}`, "error");
    });
  });
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
    dbKind,
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
    createOpenEls: Array.from(root.querySelectorAll("[data-db-suite-create-open]")),
    createModalEl: root.querySelector("[data-db-suite-create-modal]"),
    createFormEl: root.querySelector("[data-db-suite-create-form]"),
    createCancelEls: Array.from(root.querySelectorAll("[data-db-suite-create-cancel]")),
    createStatusEl: root.querySelector("[data-db-suite-create-status]"),
    valueMetaEl: root.querySelector("[data-db-suite-value-meta]"),
    valueBodyEl: root.querySelector("[data-db-suite-value-body]"),
    schemaPanelEl: root.querySelector("[data-db-suite-schema-panel]"),
    queryEditor: null,
    schemas: [],
    tables: [],
    selectedTable: "",
    collapsedSchemas: new Set(),
    apiSimpleTables: `/api/projects/${encodeURIComponent(owner)}/${encodeURIComponent(
      project
    )}/tables`,
  };

  try {
    initCreateTableDialog(state);
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
      console.error("sjtable db suite runtime init failed", err);
    });
  });
}

export function initProjectDbSuiteSjtableBehavior() {
  if (typeof (globalThis as any).Deno !== "undefined") {
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
