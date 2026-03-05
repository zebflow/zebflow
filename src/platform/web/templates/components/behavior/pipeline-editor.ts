let graphUiRuntime = null;
let codeMirrorRuntime = null;
let runtimeLoadPromise = null;

async function ensurePipelineEditorRuntime() {
  if (graphUiRuntime && codeMirrorRuntime) {
    return;
  }
  if (typeof window === "undefined" || typeof window.location === "undefined") {
    throw new Error("window location is not available");
  }
  if (!runtimeLoadPromise) {
    const graphUiUrl = new URL(
      "/assets/libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs",
      window.location.origin,
    ).toString();
    const codeMirrorUrl = new URL(
      "/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
      window.location.origin,
    ).toString();
    runtimeLoadPromise = Promise.all([
      import(graphUiUrl),
      import(codeMirrorUrl),
    ]).then(([graphUi, codeMirror]) => {
      graphUiRuntime = graphUi;
      codeMirrorRuntime = codeMirror;
    });
  }
  await runtimeLoadPromise;
}

function requireGraphUiRuntime() {
  if (!graphUiRuntime) {
    throw new Error("pipeline editor runtime is not loaded");
  }
  return graphUiRuntime;
}

function requireCodeMirrorRuntime() {
  if (!codeMirrorRuntime) {
    throw new Error("codemirror runtime is not loaded");
  }
  return codeMirrorRuntime;
}

const NODE_CATEGORIES = {
  trigger: ["n.trigger.webhook", "n.trigger.schedule", "n.trigger.manual"],
  data: ["n.sjtable.query", "n.pg.query", "n.http.request"],
  logic: ["n.script", "n.ai.zebtune"],
  render: ["n.web.render"],
};

function canonicalNodeKind(kind) {
  const raw = String(kind || "").trim();
  if (raw.startsWith("x.n.")) {
    return `n.${raw.slice("x.n.".length)}`;
  }
  return raw;
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
      const message =
        payload?.error?.message ||
        payload?.message ||
        payload?.error ||
        `${response.status} ${response.statusText}`;
      throw new Error(message);
    }
    return payload;
  });
}

function sanitizeSegment(raw) {
  return String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "") || "pipeline";
}

function nodeSlugBase(kind) {
  const parts = String(kind || "node")
    .split(".")
    .map((part) => String(part || "").trim())
    .filter((part) => part.length > 0);
  const normalized = parts[0] === "n" ? parts.slice(1) : parts;
  return sanitizeSegment(normalized.join("-") || "node");
}

function generateNodeSlug(kind, existingNodes = []) {
  const base = nodeSlugBase(kind);
  const targetKind = canonicalNodeKind(kind);
  const sameKindCount = (Array.isArray(existingNodes) ? existingNodes : []).filter((node) =>
    canonicalNodeKind(node?.zfKind) === targetKind
  ).length;
  if (sameKindCount <= 0) {
    return base;
  }
  return `${base}-${sameKindCount}`;
}

function deriveTemplateIdFromPath(rawPath) {
  return String(rawPath || "")
    .trim()
    .replace(/^pages\//, "")
    .replace(/\.(tsx|jsx|ts|js)$/i, "")
    .replace(/[\\/]+/g, ".")
    .replace(/[^a-zA-Z0-9._-]+/g, "_");
}

function ensureUniqueNodeSlug(state, node, wantedRaw) {
  const wantedBase = sanitizeSegment(wantedRaw || "");
  if (!wantedBase) {
    return node?.zfPipelineNodeId || "node";
  }
  const used = new Set(
    (state?.graphApp?.graph?.nodes || [])
      .filter((item) => item !== node)
      .map((item) => sanitizeSegment(item?.zfPipelineNodeId || ""))
      .filter((item) => item.length > 0),
  );
  let candidate = wantedBase;
  let seq = 1;
  while (used.has(candidate)) {
    candidate = `${wantedBase}-${seq}`;
    seq += 1;
  }
  return candidate;
}

function normalizeVirtualPath(raw) {
  const trimmed = String(raw || "/").trim();
  if (!trimmed || trimmed === "/") {
    return "/";
  }
  return `/${trimmed.replace(/^\/+|\/+$/g, "")}`;
}

function normalizeWebhookPath(raw) {
  const trimmed = String(raw || "/").trim();
  if (!trimmed || trimmed === "/") {
    return "/";
  }
  return `/${trimmed.replace(/^\/+|\/+$/g, "")}`;
}

function webhookPublicUrlFor(state, webhookPath) {
  const owner = String(state?.owner || "").trim();
  const project = String(state?.project || "").trim();
  if (!owner || !project || typeof window === "undefined") {
    return "";
  }
  const base = `${window.location.origin}/wh/${owner}/${project}`;
  const normalized = normalizeWebhookPath(webhookPath);
  if (normalized === "/") {
    return base;
  }
  return `${base}${normalized}`;
}

function isTemplateToken(value) {
  const text = String(value || "");
  return text.includes("{") || text.includes("}");
}

function nodeColor(kind) {
  const kindColors = requireGraphUiRuntime().DEFAULT_NODE_KIND_COLORS || {};
  return kindColors[kind] || "#334155";
}

function normalizeNodePins(kind, pinRole, rawPins, fallback = []) {
  const canonicalKind = canonicalNodeKind(kind);
  if (pinRole === "output" && canonicalKind === "n.web.render") {
    return [];
  }
  if (
    pinRole === "input" &&
    (
      canonicalKind === "n.trigger.webhook" ||
      canonicalKind === "n.trigger.schedule" ||
      canonicalKind === "n.trigger.manual"
    )
  ) {
    return [];
  }
  const pins = Array.isArray(rawPins)
    ? rawPins
      .map((pin) => String(pin || "").trim())
      .filter((pin) => pin.length > 0)
    : [];
  if (pins.length > 0) {
    return pins;
  }
  return fallback.slice();
}

function normalizeGraphForEditor(graph) {
  const source = graph && typeof graph === "object" ? graph : {};
  const nodes = Array.isArray(source.nodes) ? source.nodes : [];
  return {
    ...source,
    nodes: nodes.map((node) => {
      const kind = canonicalNodeKind(node?.kind);
      return {
        ...node,
        kind,
        input_pins: normalizeNodePins(kind, "input", node?.input_pins, ["in"]),
        output_pins: normalizeNodePins(kind, "output", node?.output_pins, ["out"]),
      };
    }),
  };
}

function emptyPipelineGraph(name, triggerKind) {
  const id = sanitizeSegment(name);
  if (triggerKind === "schedule") {
    return {
      kind: "zebflow.pipeline",
      version: "0.1",
      id,
      entry_nodes: ["trigger_schedule"],
      nodes: [
        {
          id: "trigger_schedule",
          kind: "n.trigger.schedule",
          input_pins: [],
          output_pins: ["out"],
          config: { cron: "*/5 * * * *", timezone: "UTC" },
        },
      ],
      edges: [],
    };
  }
  if (triggerKind === "function") {
    return {
      kind: "zebflow.pipeline",
      version: "0.1",
      id,
      entry_nodes: ["script_entry"],
      nodes: [
        {
          id: "script_entry",
          kind: "n.script",
          input_pins: ["in"],
          output_pins: ["out"],
          config: { source: "return input;" },
        },
      ],
      edges: [],
    };
  }
  if (triggerKind === "manual") {
    return {
      kind: "zebflow.pipeline",
      version: "0.1",
      id,
      entry_nodes: ["trigger_manual"],
      nodes: [
        {
          id: "trigger_manual",
          kind: "n.trigger.manual",
          input_pins: [],
          output_pins: ["out"],
          config: {},
        },
      ],
      edges: [],
    };
  }
  return {
    kind: "zebflow.pipeline",
    version: "0.1",
    id,
    entry_nodes: ["trigger_webhook"],
    nodes: [
        {
          id: "trigger_webhook",
          kind: "n.trigger.webhook",
          input_pins: [],
          output_pins: ["out"],
          config: { path: `/${id}`, method: "POST" },
        },
    ],
    edges: [],
  };
}

function createNodeCatalog(items) {
  const map = new Map();
  (Array.isArray(items) ? items : []).forEach((item) => {
    if (!item || !item.kind) {
      return;
    }
    map.set(item.kind, item);
  });
  return map;
}

function toInputType(field) {
  if (field.type === "textarea") {
    return "textarea";
  }
  if (field.type === "select") {
    return "select";
  }
  if (field.type === "datalist") {
    return "datalist";
  }
  if (field.type === "copy_url") {
    return "copy_url";
  }
  if (field.type === "section") {
    return "section";
  }
  if (field.type === "checkbox") {
    return "checkbox";
  }
  return "input";
}

function normalizeCredentialOption(item) {
  const credentialId = String(item?.credential_id || "").trim();
  if (!credentialId) {
    return null;
  }
  const title = String(item?.title || "").trim();
  const kind = String(item?.kind || "").trim();
  const label = title ? `${title} (${credentialId})` : credentialId;
  return {
    value: credentialId,
    label: kind ? `${label} | ${kind}` : label,
  };
}

function normalizeTemplateOption(item) {
  const relPath = String(item?.rel_path || "").trim();
  if (!relPath) {
    return null;
  }
  const name = String(item?.name || "").trim();
  return {
    value: relPath,
    label: name ? `${name} | ${relPath}` : relPath,
  };
}

function parseListLines(raw) {
  return String(raw || "")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function buildNodeFields(kind, config, state) {
  const canonicalKind = canonicalNodeKind(kind);
  const base = [
    {
      name: "title",
      label: "Title",
      type: "text",
      value: config?.title || "",
      help: "Override display title for this node.",
    },
  ];
  if (canonicalKind === "n.trigger.webhook") {
    return base.concat([
      {
        name: "path",
        label: "Path",
        type: "text",
        value: config?.path || "/",
        help: "Webhook relative path under /wh/{owner}/{project}. Supports dynamic segments.",
      },
      {
        name: "method",
        label: "Method",
        type: "select",
        value: config?.method || "POST",
        options: ["GET", "POST", "PUT", "PATCH", "DELETE"],
        help: "HTTP method accepted by webhook trigger.",
      },
      {
        name: "__webhook_public_url",
        label: "Public URL",
        type: "copy_url",
        value: webhookPublicUrlFor(state, config?.path || "/"),
        help: "Copy-ready URL for this trigger.",
      },
    ]);
  }
  if (canonicalKind === "n.trigger.schedule") {
    return base.concat([
      {
        name: "cron",
        label: "Cron",
        type: "text",
        value: config?.cron || "*/5 * * * *",
        help: "Cron expression for schedule trigger.",
      },
      {
        name: "timezone",
        label: "Timezone",
        type: "text",
        value: config?.timezone || "UTC",
        help: "IANA timezone, for example UTC or Asia/Jakarta.",
      },
    ]);
  }
  if (canonicalKind === "n.trigger.manual") {
    return base.concat([
      {
        name: "__manual_note",
        label: "Manual Trigger",
        type: "text",
        value: "Runs only when pipeline execute trigger=manual.",
        readonly: true,
      },
    ]);
  }
  if (canonicalKind === "n.script") {
    return base.concat([
      {
        name: "source",
        label: "Source",
        type: "textarea",
        rows: 16,
        value: config?.source || "return input;",
        help: "Deno JavaScript expression/body. Must return next payload.",
      },
    ]);
  }
  if (canonicalKind === "n.http.request") {
    return base.concat([
      {
        name: "url",
        label: "URL",
        type: "text",
        value: config?.url || "https://example.com",
        help: "Fallback URL when url_expr is empty.",
      },
      {
        name: "method",
        label: "Method",
        type: "select",
        value: config?.method || "GET",
        options: ["GET", "POST", "PUT", "PATCH", "DELETE"],
        help: "Fallback HTTP method when method_expr is empty.",
      },
      { name: "timeout_ms", label: "Timeout (ms)", type: "text", value: config?.timeout_ms == null ? "" : String(config.timeout_ms), help: "Request timeout in milliseconds." },
      { name: "url_expr", label: "URL Expr", type: "textarea", rows: 3, value: config?.url_expr || "", help: "Optional JS expression returning string URL." },
      { name: "method_expr", label: "Method Expr", type: "textarea", rows: 3, value: config?.method_expr || "", help: "Optional JS expression returning string method." },
      { name: "body_path", label: "Body Path", type: "text", value: config?.body_path || "", help: "Payload path used as request body when body_expr is empty." },
      {
        name: "headers_expr",
        label: "Headers Expr",
        type: "textarea",
        rows: 4,
        value: config?.headers_expr || "",
        help: "JS expression returning header object.",
      },
      {
        name: "body_expr",
        label: "Body Expr",
        type: "textarea",
        rows: 4,
        value: config?.body_expr || "",
        help: "JS expression returning request body value.",
      },
    ]);
  }
  if (canonicalKind === "n.sjtable.query") {
    return base.concat([
      { name: "table", label: "Table", type: "text", value: config?.table || "posts", help: "Simple table slug to query/upsert." },
      {
        name: "operation",
        label: "Operation",
        type: "select",
        value: config?.operation || "query",
        options: ["query", "upsert"],
        help: "query returns rows, upsert writes one row.",
      },
      { name: "table_expr", label: "Table Expr", type: "textarea", rows: 3, value: config?.table_expr || "", help: "Optional JS expression returning table slug." },
      { name: "where_field", label: "Where Field", type: "text", value: config?.where_field || "", help: "Field name for equality filter in query mode." },
      { name: "where_field_expr", label: "Where Field Expr", type: "textarea", rows: 3, value: config?.where_field_expr || "", help: "Optional JS expression returning where field." },
      { name: "where_value_path", label: "Where Value Path", type: "text", value: config?.where_value_path || "", help: "Payload path for where value in query mode." },
      { name: "where_value_expr", label: "Where Value Expr", type: "textarea", rows: 3, value: config?.where_value_expr || "", help: "Optional JS expression returning where value." },
      { name: "limit", label: "Limit", type: "text", value: config?.limit == null ? "" : String(config.limit), help: "Max rows for query mode." },
      { name: "limit_expr", label: "Limit Expr", type: "textarea", rows: 3, value: config?.limit_expr || "", help: "Optional JS expression returning integer limit." },
      { name: "row_id_path", label: "Row ID Path", type: "text", value: config?.row_id_path || "row_id", help: "Payload path resolving row id for upsert mode." },
      { name: "row_id_expr", label: "Row ID Expr", type: "textarea", rows: 3, value: config?.row_id_expr || "", help: "Optional JS expression returning row id." },
      { name: "data_path", label: "Data Path", type: "text", value: config?.data_path || "data", help: "Payload path resolving data object for upsert mode." },
      { name: "data_expr", label: "Data Expr", type: "textarea", rows: 4, value: config?.data_expr || "", help: "Optional JS expression returning upsert data payload." },
    ]);
  }
  if (canonicalKind === "n.pg.query") {
    const credentialOptions = (Array.isArray(state?.pgCredentials) ? state.pgCredentials : [])
      .map(normalizeCredentialOption)
      .filter(Boolean);
    const credentialOptionValues = credentialOptions.map((option) => option.value);
    const selectedCredentialId = String(config?.credential_id || "");
    if (
      selectedCredentialId &&
      !credentialOptionValues.includes(selectedCredentialId)
    ) {
      credentialOptions.unshift({
        value: selectedCredentialId,
        label: `${selectedCredentialId} (not listed)`,
      });
    }
    if (credentialOptions.length === 0) {
      credentialOptions.push({
        value: "",
        label: "No postgres credential available",
      });
    }
    return base.concat([
      {
        name: "credential_id",
        label: "Credential",
        type: "select",
        value: config?.credential_id || "",
        options: credentialOptions,
        help: "Loaded from project credentials filtered by kind=postgres.",
      },
      { name: "credential_id_expr", label: "Credential ID Expr", type: "textarea", rows: 3, value: config?.credential_id_expr || "", help: "Optional JS expression returning credential id." },
      {
        name: "query",
        label: "Query",
        type: "textarea",
        rows: 8,
        value: config?.query || "SELECT 1;",
        help: "SQL query string. SELECT/WITH returns rows, others return affected_rows.",
      },
      { name: "query_expr", label: "Query Expr", type: "textarea", rows: 4, value: config?.query_expr || "", help: "Optional JS expression returning SQL string." },
      { name: "params_path", label: "Params Path", type: "text", value: config?.params_path || "", help: "Payload path to array of SQL params." },
      { name: "params_expr", label: "Params Expr", type: "textarea", rows: 4, value: config?.params_expr || "", help: "Optional JS expression returning params array/value." },
    ]);
  }
  if (canonicalKind === "n.web.render") {
    const templateOptions = (Array.isArray(state?.pageTemplates) ? state.pageTemplates : [])
      .map(normalizeTemplateOption)
      .filter(Boolean);
    const selectedTemplate = String(
      config?.template_path || config?.template_rel_path || "",
    );
    if (selectedTemplate && !templateOptions.some((option) => option.value === selectedTemplate)) {
      templateOptions.unshift({
        value: selectedTemplate,
        label: `${selectedTemplate} (not listed)`,
      });
    }
    return base.concat([
      {
        name: "__render_basic",
        label: "Render Target",
        type: "section",
      },
      {
        name: "template_path_select",
        label: "Template",
        type: "datalist",
        value: selectedTemplate,
        options: templateOptions,
        placeholder: "pages/blog/list.tsx",
        help: "Select template file from workspace. Main input for n.web.render.",
      },
    ]);
  }
  if (canonicalKind === "n.ai.zebtune") {
    return base.concat([
      { name: "step_budget", label: "Step Budget", type: "text", value: config?.step_budget == null ? "" : String(config.step_budget), help: "Maximum LLM + tool iterations (default 10)." },
      {
        name: "output_mode",
        label: "Output Mode",
        type: "select",
        value: config?.output_mode || "full",
        options: ["full", "final_only"],
        help: "full includes chain details, final_only returns final answer only.",
      },
      {
        name: "system_prompt",
        label: "System Prompt",
        type: "textarea",
        rows: 10,
        value: config?.system_prompt || "",
        help: "Optional override for default zebtune system prompt.",
      },
    ]);
  }
  return base.concat([
    {
      name: "config_json",
      label: "Config JSON",
      type: "textarea",
      rows: 10,
      value: JSON.stringify(config || {}, null, 2),
    },
  ]);
}

function extractNodeConfig(kind, fieldsContainer) {
  const values = {};
  fieldsContainer.querySelectorAll("[data-node-field]").forEach((field) => {
    const key = field.getAttribute("data-node-field");
    if (!key) {
      return;
    }
    if (field.type === "checkbox") {
      values[key] = field.checked;
      return;
    }
    values[key] = field.value;
  });

  if (values.config_json && !kind.startsWith("n.")) {
    try {
      return JSON.parse(values.config_json);
    } catch (_err) {
      return {};
    }
  }

  const next = {};
  Object.entries(values).forEach(([key, value]) => {
    if (key.startsWith("__")) {
      return;
    }
    if (key === "config_json") {
      try {
        Object.assign(next, JSON.parse(value || "{}"));
      } catch (_err) {
        next[key] = value;
      }
      return;
    }
    if (key === "template_path_select") {
      const selected = String(value || "").trim();
      if (!selected) {
        return;
      }
      next.template_path = selected;
      next.template_rel_path = selected;
      next.template_id = deriveTemplateIdFromPath(selected);
      return;
    }
    if ((key === "limit" || key === "timeout_ms" || key === "step_budget") && value !== "") {
      const asNum = Number(value);
      next[key] = Number.isFinite(asNum) ? asNum : value;
      return;
    }
    if (value === "") {
      return;
    }
    next[key] = value;
  });
  return next;
}

function updateSelectedItemClass(state) {
  state.root.querySelectorAll(".pipeline-editor-item").forEach((item) => {
    item.classList.toggle(
      "is-selected",
      item.getAttribute("data-editor-pipeline-id") === state.selectedId,
    );
  });
}

function setFootHits(state, hits) {
  const success = state.root.querySelector("[data-editor-hit-success]");
  const failed = state.root.querySelector("[data-editor-hit-failed]");
  const latest = state.root.querySelector("[data-editor-hit-error]");

  const okCount = Number(hits?.success_count || 0);
  const failCount = Number(hits?.failed_count || 0);
  const latestErr = Array.isArray(hits?.latest_errors) && hits.latest_errors.length > 0
    ? `${hits.latest_errors[0].code}: ${hits.latest_errors[0].message}`
    : "-";

  if (success) {
    success.textContent = `Success: ${okCount}`;
  }
  if (failed) {
    failed.textContent = `Failed: ${failCount}`;
  }
  if (latest) {
    latest.textContent = `Latest error: ${latestErr}`;
    latest.title = latestErr;
  }
}

function setDraftState(state) {
  const draft = state.root.querySelector("[data-editor-draft-state]");
  if (!draft || !state.currentMeta) {
    return;
  }
  const isActive = state.currentMeta.active_hash && state.currentMeta.active_hash === state.currentMeta.hash;
  const hasDraft = state.currentMeta.active_hash && state.currentMeta.active_hash !== state.currentMeta.hash;
  const value = isActive ? "active" : hasDraft ? "draft changed" : "inactive";
  draft.textContent = value;
  draft.setAttribute("data-tone", isActive ? "ok" : hasDraft ? "warning" : "neutral");
}

function setHeaderInfo(state) {
  const name = state.root.querySelector("[data-editor-selected-name]");
  const meta = state.root.querySelector("[data-editor-selected-meta]");
  const trigger = state.root.querySelector("[data-editor-trigger-kind]");
  const lockState = state.root.querySelector("[data-editor-lock-state]");
  if (!state.currentMeta) {
    if (name) {
      name.textContent = "No pipeline selected";
    }
    if (meta) {
      meta.textContent = "Select a pipeline to edit graph + node config.";
    }
    if (trigger) {
      trigger.textContent = "trigger: -";
      trigger.setAttribute("data-trigger-kind", "none");
    }
    if (lockState) {
      lockState.textContent = "editable";
      lockState.setAttribute("data-tone", "ok");
    }
    const draft = state.root.querySelector("[data-editor-draft-state]");
    if (draft) {
      draft.setAttribute("data-tone", "neutral");
    }
    return;
  }
  if (name) {
    name.textContent = state.currentMeta.title || state.currentMeta.name;
  }
  if (meta) {
    meta.textContent = `${state.currentMeta.virtual_path} | ${state.currentMeta.trigger_kind} | ${state.currentMeta.file_rel_path}`;
  }
  if (trigger) {
    const triggerKind = String(state.currentMeta.trigger_kind || "-").toUpperCase();
    trigger.textContent = `trigger: ${triggerKind}`;
    trigger.setAttribute("data-trigger-kind", triggerKind.toLowerCase());
  }
  if (lockState) {
    lockState.textContent = state.currentLocked ? "locked" : "editable";
    lockState.setAttribute("data-tone", state.currentLocked ? "danger" : "ok");
  }
}

function setLockedState(state) {
  const saveBtn = state.root.querySelector("[data-editor-save]");
  const activateBtn = state.root.querySelector("[data-editor-activate]");
  const deactivateBtn = state.root.querySelector("[data-editor-deactivate]");
  const catButtons = state.root.querySelectorAll("[data-editor-cat]");
  const isLocked = !!state.currentLocked;

  [saveBtn, activateBtn, deactivateBtn].forEach((btn) => {
    if (!btn) {
      return;
    }
    btn.disabled = isLocked;
    btn.setAttribute("aria-disabled", isLocked ? "true" : "false");
  });

  catButtons.forEach((btn) => {
    btn.disabled = isLocked;
    btn.setAttribute("aria-disabled", isLocked ? "true" : "false");
  });

  if (isLocked) {
    closeCategoryMenu(state);
  }
}

function collectGraphAsPipeline(state) {
  const nodes = [];
  const edges = [];

  const used = new Set();
  const graphNodes = state.graphApp.graph.nodes;
  graphNodes.forEach((node) => {
    const kind = node.zfKind || "n.script";
    let nodeId = sanitizeSegment(node.zfPipelineNodeId || kind.split(".").pop() || "node");
    if (!nodeId) {
      nodeId = "node";
    }
    let candidate = nodeId;
    let seq = 2;
    while (used.has(candidate)) {
      candidate = `${nodeId}_${seq}`;
      seq += 1;
    }
    nodeId = candidate;
    used.add(nodeId);

    node.zfPipelineNodeId = nodeId;
    node.zfKind = kind;
    node.zfConfig = {
      ...(node.zfConfig || {}),
      ui: {
        x: Math.round(node.x),
        y: Math.round(node.y),
      },
    };

    nodes.push({
      id: nodeId,
      kind,
      input_pins: normalizeNodePins(kind, "input", node.inputs.map((pin) => pin.name), ["in"]),
      output_pins: normalizeNodePins(
        kind,
        "output",
        node.outputs.map((pin) => pin.name),
        ["out"],
      ),
      config: node.zfConfig,
    });
  });

  const byGraphId = new Map(graphNodes.map((node) => [node.id, node]));
  state.graphApp.graph.links.forEach((link) => {
    const from = byGraphId.get(link.fromNode);
    const to = byGraphId.get(link.toNode);
    if (!from || !to) {
      return;
    }
    const fromPin = from.outputs[link.fromSlot]?.name || from.outputs[0]?.name || "out";
    const toPin = to.inputs[link.toSlot]?.name || to.inputs[0]?.name || "in";
    edges.push({
      from_node: from.zfPipelineNodeId,
      from_pin: fromPin,
      to_node: to.zfPipelineNodeId,
      to_pin: toPin,
    });
  });

  const entryNodes = nodes
    .filter((node) => String(node.kind || "").startsWith("n.trigger."))
    .map((node) => node.id);
  const fallbackEntry = nodes[0] ? [nodes[0].id] : [];

  return {
    kind: "zebflow.pipeline",
    version: "0.1",
    id: state.currentGraph?.id || sanitizeSegment(state.currentMeta?.name || "pipeline"),
    metadata: {
      ...(state.currentGraph?.metadata || {}),
      virtual_path: state.currentMeta?.virtual_path || state.scopePath || "/",
      locked: !!state.currentLocked,
    },
    entry_nodes: entryNodes.length > 0 ? entryNodes : fallbackEntry,
    nodes,
    edges,
  };
}

function attachNodeEditButtons(state) {
  const root = state.root.querySelector("[data-pipeline-graph-root]");
  if (!root) {
    return;
  }
  if (state.currentLocked) {
    root.querySelectorAll(".zf-node-edit").forEach((btn) => btn.remove());
    return;
  }
  const nodeMap = new Map(state.graphApp.graph.nodes.map((node) => [String(node.id), node]));
  root.querySelectorAll(".zgu-node").forEach((nodeEl) => {
    const graphNodeId = nodeEl.getAttribute("data-id");
    if (!graphNodeId) {
      return;
    }
    const nodeData = nodeMap.get(graphNodeId);
    if (!nodeData) {
      return;
    }
    if (nodeEl.querySelector(".zf-node-edit")) {
      return;
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "zf-node-edit";
    btn.setAttribute("data-zgu-nodrag", "true");
    btn.textContent = "E";
    btn.title = "Edit Node";
    btn.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      openNodeDialog(state, nodeData);
    });
    nodeEl.appendChild(btn);

    const kind = String(nodeData?.zfKind || "");
    const slug = String(nodeData?.zfPipelineNodeId || "");
    let badge = nodeEl.querySelector(".zf-node-slug");
    if (!badge) {
      badge = document.createElement("div");
      badge.className = "zf-node-slug";
      nodeEl.appendChild(badge);
    }
    badge.textContent = `${slug || "node"}${kind ? ` | ${kind}` : ""}`;
  });
}

function scheduleAttachNodeEditButtons(state) {
  attachNodeEditButtons(state);
  window.setTimeout(() => attachNodeEditButtons(state), 0);
  window.setTimeout(() => attachNodeEditButtons(state), 120);
}

function ensureNodeEditObserver(state) {
  const root = state.root.querySelector("[data-pipeline-graph-root]");
  if (!root) {
    return;
  }
  if (state.nodeEditObserver) {
    state.nodeEditObserver.disconnect();
  }
  const observer = new MutationObserver(() => {
    attachNodeEditButtons(state);
  });
  observer.observe(root, { childList: true, subtree: true });
  state.nodeEditObserver = observer;
}

function resolveInitialPipelineId(root, selectedId) {
  const selected = String(selectedId || "").trim();
  if (selected && !isTemplateToken(selected)) {
    return selected;
  }
  const items = Array.from(root.querySelectorAll("[data-editor-pipeline-id]"));
  for (const item of items) {
    if (item.hasAttribute("hidden")) {
      continue;
    }
    const id = String(item.getAttribute("data-editor-pipeline-id") || "").trim();
    if (!id || isTemplateToken(id)) {
      continue;
    }
    return id;
  }
  return "";
}

function renderNodeFormFields(fieldsContainer, fields) {
  fieldsContainer.innerHTML = "";
  fields.forEach((field) => {
    const row = document.createElement("label");
    row.className = "pipeline-editor-field";

    const label = document.createElement("span");
    label.textContent = field.label;
    row.appendChild(label);

    const type = toInputType(field);
    let input;
    if (type === "copy_url") {
      const wrap = document.createElement("div");
      wrap.className = "pipeline-editor-copy-row";

      const readonlyInput = document.createElement("input");
      readonlyInput.type = "text";
      readonlyInput.value = String(field.value || "");
      readonlyInput.readOnly = true;
      readonlyInput.setAttribute("data-node-field", field.name);
      wrap.appendChild(readonlyInput);

      const copyBtn = document.createElement("button");
      copyBtn.type = "button";
      copyBtn.className = "pipeline-editor-copy-btn";
      copyBtn.textContent = "Copy";
      copyBtn.addEventListener("click", async () => {
        const value = String(readonlyInput.value || "");
        if (!value) {
          return;
        }
        try {
          await navigator.clipboard.writeText(value);
          copyBtn.textContent = "Copied";
          window.setTimeout(() => {
            copyBtn.textContent = "Copy";
          }, 900);
        } catch (_err) {
          readonlyInput.select();
        }
      });
      wrap.appendChild(copyBtn);
      row.appendChild(wrap);
      input = readonlyInput;
    } else if (type === "textarea") {
      input = document.createElement("textarea");
      input.rows = Number(field.rows || 5);
      input.value = String(field.value || "");
    } else if (type === "select") {
      input = document.createElement("select");
      (field.options || []).forEach((optionValue) => {
        const optionObj =
          optionValue && typeof optionValue === "object"
            ? optionValue
            : { value: optionValue, label: optionValue };
        const option = document.createElement("option");
        option.value = String(optionObj.value ?? "");
        option.textContent = String(optionObj.label ?? optionObj.value ?? "");
        if (String(option.value) === String(field.value || "")) {
          option.selected = true;
        }
        input.appendChild(option);
      });
    } else if (type === "datalist") {
      const inputWrap = document.createElement("div");
      inputWrap.className = "pipeline-editor-datalist-wrap";
      input = document.createElement("input");
      input.type = "text";
      input.value = String(field.value || "");
      const listId = `pipeline-editor-list-${Math.random().toString(36).slice(2)}`;
      input.setAttribute("list", listId);
      const datalist = document.createElement("datalist");
      datalist.id = listId;
      (field.options || []).forEach((optionValue) => {
        const optionObj =
          optionValue && typeof optionValue === "object"
            ? optionValue
            : { value: optionValue, label: optionValue };
        const option = document.createElement("option");
        option.value = String(optionObj.value ?? "");
        option.label = String(optionObj.label ?? optionObj.value ?? "");
        datalist.appendChild(option);
      });
      inputWrap.appendChild(input);
      inputWrap.appendChild(datalist);
      row.appendChild(inputWrap);
      input.setAttribute("data-node-field", field.name);
      if (field.placeholder && typeof field.placeholder === "string") {
        input.placeholder = field.placeholder;
      }
      if (field.readonly) {
        input.readOnly = true;
        input.disabled = true;
      }
      if (field.help) {
        const hint = document.createElement("small");
        hint.className = "pipeline-editor-field-help";
        hint.textContent = field.help;
        row.appendChild(hint);
      }
      fieldsContainer.appendChild(row);
      return;
    } else if (type === "section") {
      row.className = "pipeline-editor-field pipeline-editor-field-section";
      row.innerHTML = `<span>${field.label}</span>`;
      fieldsContainer.appendChild(row);
      return;
    } else if (type === "checkbox") {
      input = document.createElement("input");
      input.type = "checkbox";
      input.checked = !!field.value;
    } else {
      input = document.createElement("input");
      input.type = "text";
      input.value = String(field.value || "");
    }
    if (type !== "copy_url") {
      input.setAttribute("data-node-field", field.name);
      if (field.placeholder && typeof field.placeholder === "string") {
        input.placeholder = field.placeholder;
      }
      if (field.readonly) {
        input.readOnly = true;
        input.disabled = true;
      }
      row.appendChild(input);
    }

    if (field.help) {
      const hint = document.createElement("small");
      hint.className = "pipeline-editor-field-help";
      hint.textContent = field.help;
      row.appendChild(hint);
    }

    fieldsContainer.appendChild(row);
  });
}

async function openWebRenderNodeDialog(state, node, refs) {
  const { dialog, form, titleEl, copyEl, fieldsContainer } = refs;
  const config = node.zfConfig || {};
  const selectedTemplate = String(config?.template_path || config?.template_rel_path || "");

  dialog.classList.add("is-fullscreen");
  if (titleEl) {
    titleEl.textContent = "Edit Node | n.web.render";
  }
  if (copyEl) {
    copyEl.textContent = "Set slug/title/template, then edit the selected template directly.";
  }

  const templates = Array.isArray(state.pageTemplates) ? state.pageTemplates : [];
  fieldsContainer.classList.add("is-web-render");
  fieldsContainer.innerHTML = `
    <div class="pipeline-render-top">
      <label class="pipeline-editor-field">
        <span>Node Slug</span>
        <input type="text" data-node-field="__node_slug" value="${String(node.zfPipelineNodeId || "")}" />
      </label>
      <label class="pipeline-editor-field">
        <span>Title</span>
        <input type="text" data-node-field="title" value="${String(config?.title || "")}" />
      </label>
    </div>
    <div class="pipeline-render-workspace">
      <aside class="pipeline-render-sidebar">
        <div class="pipeline-render-sidebar-head">
          <div class="pipeline-render-sidebar-title">Select template</div>
          <input type="text" data-render-template-search="true" placeholder="Search template path..." />
        </div>
        <div class="pipeline-render-sidebar-list" data-render-template-sidebar="true"></div>
      </aside>
      <section class="pipeline-render-editor-shell">
        <div class="pipeline-render-editor-head">
          <span data-render-current-template="true">No template selected</span>
          <div class="pipeline-render-editor-actions">
            <span data-render-template-status="true" class="pipeline-render-status">Idle</span>
            <button type="button" class="project-inline-chip" data-render-template-save="true">Save Template</button>
          </div>
        </div>
        <div class="pipeline-render-editor-host" data-render-editor-host="true"></div>
      </section>
    </div>
  `;

  const slugInput = fieldsContainer.querySelector('[data-node-field="__node_slug"]');
  const titleInput = fieldsContainer.querySelector('[data-node-field="title"]');
  const searchInput = fieldsContainer.querySelector('[data-render-template-search="true"]');
  const sidebar = fieldsContainer.querySelector('[data-render-template-sidebar="true"]');
  const editorHost = fieldsContainer.querySelector('[data-render-editor-host="true"]');
  const currentTemplateEl = fieldsContainer.querySelector('[data-render-current-template="true"]');
  const saveTemplateBtn = fieldsContainer.querySelector('[data-render-template-save="true"]');
  const templateStatusEl = fieldsContainer.querySelector('[data-render-template-status="true"]');

  const { EditorView, basicSetup, javascript, oneDark } = requireCodeMirrorRuntime();
  const editorView = new EditorView({
    doc: "",
    parent: editorHost || fieldsContainer,
    extensions: [
      basicSetup,
      oneDark,
      javascript(),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) {
          return;
        }
        renderState.isDirty = true;
        if (templateStatusEl) {
          templateStatusEl.textContent = "Unsaved";
          templateStatusEl.setAttribute("data-state", "warning");
        }
      }),
    ],
  });

  const renderState = {
    currentPath: "",
    query: "",
    isDirty: false,
  };

  function setTemplateStatus(message, stateKey = "neutral") {
    if (!templateStatusEl) {
      return;
    }
    templateStatusEl.textContent = message;
    templateStatusEl.setAttribute("data-state", stateKey);
  }

  async function loadTemplate(relPath) {
    const path = String(relPath || "").trim();
    if (!path) {
      renderState.currentPath = "";
      renderState.isDirty = false;
      editorView.dispatch({
        changes: { from: 0, to: editorView.state.doc.length, insert: "" },
      });
      if (currentTemplateEl) {
        currentTemplateEl.textContent = "No template selected";
      }
      setTemplateStatus("Idle");
      return;
    }
    if (!state.api.templateFile) {
      setTemplateStatus("Template API missing", "error");
      return;
    }
    setTemplateStatus("Loading...", "neutral");
    const payload = await requestJson(`${state.api.templateFile}?path=${encodeURIComponent(path)}`);
    const content = String(payload?.content || "");
    renderState.currentPath = path;
    renderState.isDirty = false;
    editorView.dispatch({
      changes: { from: 0, to: editorView.state.doc.length, insert: content },
    });
    if (currentTemplateEl) {
      currentTemplateEl.textContent = path;
    }
    setTemplateStatus("Loaded", "ok");
  }

  async function saveTemplate() {
    const path = String(renderState.currentPath || "").trim();
    if (!path || !state.api.templateSave) {
      return;
    }
    setTemplateStatus("Saving...", "neutral");
    const content = editorView.state.doc.toString();
    await requestJson(state.api.templateSave, {
      method: "POST",
      body: JSON.stringify({
        rel_path: path,
        content,
      }),
    });
    renderState.isDirty = false;
    setTemplateStatus("Saved", "ok");
  }

  async function selectTemplate(relPath) {
    const nextPath = String(relPath || "").trim();
    if (!nextPath) {
      return;
    }
    if (renderState.isDirty) {
      await saveTemplate().catch(() => {});
    }
    await loadTemplate(nextPath).catch((err) => {
      setTemplateStatus(err?.message || String(err), "error");
    });
    renderSidebar();
  }

  function renderSidebar() {
    if (!sidebar) {
      return;
    }
    const query = String(renderState.query || "").trim().toLowerCase();
    const visibleTemplates = templates.filter((item) => {
      const relPath = String(item?.rel_path || "");
      if (!query) {
        return true;
      }
      return relPath.toLowerCase().includes(query);
    });
    sidebar.innerHTML = visibleTemplates
      .map((item) => {
        const relPath = String(item?.rel_path || "");
        const selected = relPath === String(renderState.currentPath || "").trim();
        const cls = selected ? "pipeline-render-template-item is-selected" : "pipeline-render-template-item";
        return `<button type="button" class="${cls}" data-render-template-item="${relPath}">${relPath}</button>`;
      })
      .join("") || `<div class="pipeline-render-template-empty">No template matched.</div>`;
    sidebar.querySelectorAll("[data-render-template-item]").forEach((button) => {
      button.addEventListener("click", async () => {
        const relPath = button.getAttribute("data-render-template-item") || "";
        await selectTemplate(relPath);
      });
    });
  }

  if (searchInput) {
    searchInput.addEventListener("input", () => {
      renderState.query = String(searchInput.value || "");
      renderSidebar();
    });
  }
  if (saveTemplateBtn) {
    saveTemplateBtn.addEventListener("click", async () => {
      await saveTemplate().catch((err) => {
        setTemplateStatus(err?.message || String(err), "error");
      });
    });
  }

  renderSidebar();
  if (selectedTemplate) {
    await selectTemplate(selectedTemplate).catch((err) => {
      setTemplateStatus(err?.message || String(err), "error");
    });
  } else {
    setTemplateStatus("Choose template", "neutral");
  }

  let cleanedUp = false;
  const onClose = () => {
    if (cleanedUp) {
      return;
    }
    cleanedUp = true;
    fieldsContainer.classList.remove("is-web-render");
    try {
      editorView.destroy();
    } catch (_err) {
      // ignore teardown errors
    }
    form.removeEventListener("submit", onSubmit);
  };

  const onSubmit = async (event) => {
    event.preventDefault();
    const wantedSlug = String(slugInput?.value || "").trim();
    node.zfPipelineNodeId = ensureUniqueNodeSlug(state, node, wantedSlug);
    const nextConfig = {
      ...(node.zfConfig || {}),
      route: (node.zfConfig && node.zfConfig.route) ? node.zfConfig.route : "/",
    };
    const title = String(titleInput?.value || "").trim();
    if (title) {
      nextConfig.title = title;
      node.title = title;
    } else {
      delete nextConfig.title;
    }
    const selected = String(renderState.currentPath || "").trim();
    if (selected) {
      nextConfig.template_path = selected;
      nextConfig.template_rel_path = selected;
      nextConfig.template_id = deriveTemplateIdFromPath(selected);
    }
    if (renderState.isDirty) {
      await saveTemplate().catch(() => {});
    }
    node.zfConfig = nextConfig;

    dialog.close();
    dialog.classList.remove("is-fullscreen");
    attachNodeEditButtons(state);
  };

  form.addEventListener("submit", onSubmit);
  dialog.addEventListener("close", onClose, { once: true });
  dialog.showModal();
}

function openNodeDialog(state, node) {
  const dialog = state.root.querySelector("[data-editor-node-dialog]");
  const form = state.root.querySelector("[data-editor-node-form]");
  const titleEl = state.root.querySelector("[data-editor-node-title]");
  const copyEl = state.root.querySelector("[data-editor-node-copy]");
  const fieldsContainer = state.root.querySelector("[data-editor-node-fields]");
  if (!dialog || !form || !fieldsContainer) {
    return;
  }

  const kind = node.zfKind || "n.script";
  if (kind === "n.web.render") {
    openWebRenderNodeDialog(state, node, {
      dialog,
      form,
      titleEl,
      copyEl,
      fieldsContainer,
    }).catch((err) => {
      console.error("open web render dialog failed", err);
    });
    return;
  }
  const config = node.zfConfig || {};
  const fields = [
    {
      name: "__node_slug",
      label: "Node Slug",
      type: "text",
      value: node.zfPipelineNodeId || "",
      help: "Unique key for this node in pipeline graph edges.",
    },
  ].concat(buildNodeFields(kind, config, state));
  dialog.classList.toggle("is-fullscreen", kind === "n.web.render" || kind === "n.script");
  if (titleEl) {
    titleEl.textContent = `Edit Node | ${kind}`;
  }
  if (copyEl) {
    const def = state.nodeCatalog.get(kind);
    copyEl.textContent = def?.description || "Configure node fields based on node contract.";
  }
  renderNodeFormFields(fieldsContainer, fields);
  if (canonicalNodeKind(kind) === "n.trigger.webhook") {
    const pathInput = fieldsContainer.querySelector('[data-node-field="path"]');
    const urlInput = fieldsContainer.querySelector('[data-node-field="__webhook_public_url"]');
    if (pathInput && urlInput) {
      const syncPublicUrl = () => {
        const nextPath = String(pathInput.value || "/");
        urlInput.value = webhookPublicUrlFor(state, nextPath);
      };
      pathInput.addEventListener("input", syncPublicUrl);
      pathInput.addEventListener("change", syncPublicUrl);
      syncPublicUrl();
    }
  }

  const onSubmit = (event) => {
    event.preventDefault();
    const slugInput = fieldsContainer.querySelector('[data-node-field="__node_slug"]');
    if (slugInput) {
      node.zfPipelineNodeId = ensureUniqueNodeSlug(state, node, slugInput.value || "");
    }
    const nextConfig = extractNodeConfig(kind, fieldsContainer);
    node.zfConfig = nextConfig;
    if (nextConfig.title) {
      node.title = nextConfig.title;
      const header = node.el?.querySelector(".zgu-node-header");
      if (header) {
        header.textContent = nextConfig.title;
      }
    }
    dialog.close();
    dialog.classList.remove("is-fullscreen");
    attachNodeEditButtons(state);
    form.removeEventListener("submit", onSubmit);
  };

  form.addEventListener("submit", onSubmit);
  dialog.showModal();
}

function buildCategoryMenu(state, category) {
  const menu = state.root.querySelector("[data-editor-cat-menu]");
  if (!menu) {
    return;
  }
  const kinds = NODE_CATEGORIES[category] || [];
  const items = kinds
    .map((kind) => state.nodeCatalog.get(kind))
    .filter(Boolean);

  menu.innerHTML = "";
  items.forEach((item) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "pipeline-editor-node-insert";
    btn.textContent = item.title || item.kind;
    btn.title = item.description || item.kind;
    btn.addEventListener("click", () => addNodeFromCatalog(state, item.kind));
    menu.appendChild(btn);
  });
}

function closeCategoryMenu(state) {
  const menu = state.root.querySelector("[data-editor-cat-menu]");
  if (!menu) {
    return;
  }
  menu.classList.remove("is-open");
}

function addNodeFromCatalog(state, kind) {
  if (!state.graphApp || state.currentLocked) {
    return;
  }
  const def = state.nodeCatalog.get(kind);
  if (!def) {
    return;
  }

  const ui = state.graphApp.ui;
  const x = (-ui.transform.x + ui.workspaceEl.clientWidth / 2) / ui.transform.k - 90;
  const y = (-ui.transform.y + ui.workspaceEl.clientHeight / 2) / ui.transform.k - 50;
  const node = state.graphApp.factory.custom(x, y, {
    title: def.title || def.kind,
    color: nodeColor(kind),
    inputs: normalizeNodePins(kind, "input", def.input_pins, ["in"]),
    outputs: normalizeNodePins(kind, "output", def.output_pins, ["out"]),
  });
  node.zfKind = kind;
  node.zfConfig = {};
  node.zfPipelineNodeId = generateNodeSlug(kind, state.graphApp.graph.nodes);
  state.graphApp.addNode(node);
  scheduleAttachNodeEditButtons(state);
  closeCategoryMenu(state);
}

async function loadPipeline(state, id) {
  const encoded = encodeURIComponent(id);
  const payload = await requestJson(`${state.api.byId}?id=${encoded}&include_source=true`);
  const source = payload?.source || "{}";
  let graph;
  try {
    graph = JSON.parse(source);
  } catch (_err) {
    graph = emptyPipelineGraph(payload?.meta?.name || "pipeline", payload?.meta?.trigger_kind || "webhook");
  }
  graph = normalizeGraphForEditor(graph);

  state.selectedId = id;
  state.currentMeta = payload.meta || null;
  state.currentLocked = !!payload.locked;
  state.scopePath = state.currentMeta?.virtual_path || state.scopePath || "/";
  state.currentGraph = graph;
  state.currentSource = source;

  if (state.graphApp) {
    state.graphApp.destroy();
    state.graphApp = null;
  }

  const graphRoot = state.root.querySelector("[data-pipeline-graph-root]");
  const graphUi = requireGraphUiRuntime();
  const scene = graphUi.createPipelineScene(graph, {
    label: payload?.meta?.name || graph.id || "pipeline",
    kindColors: graphUi.DEFAULT_NODE_KIND_COLORS,
  });

  state.graphApp = graphUi.createGraphUI(graphRoot, {
    showHeader: false,
    showToolbox: false,
    readOnly: state.currentLocked,
    snapToGrid: state.snapToGrid,
    gridSize: 30,
    scenes: { pipeline: scene },
    initialScene: "pipeline",
  });
  scheduleAttachNodeEditButtons(state);
  ensureNodeEditObserver(state);

  updateSelectedItemClass(state);
  setLockedState(state);
  setHeaderInfo(state);
  setDraftState(state);
  setFootHits(state, payload.hits || null);
}

function bindCategoryButtons(state) {
  const buttons = state.root.querySelectorAll("[data-editor-cat]");
  const menu = state.root.querySelector("[data-editor-cat-menu]");
  let activeCategory = "";

  buttons.forEach((btn) => {
    btn.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (state.currentLocked) {
        return;
      }

      const category = btn.getAttribute("data-editor-cat") || "trigger";
      const isSame = activeCategory === category && menu?.classList.contains("is-open");
      buttons.forEach((node) => node.classList.remove("is-active"));
      if (isSame) {
        activeCategory = "";
        closeCategoryMenu(state);
        return;
      }
      activeCategory = category;
      btn.classList.add("is-active");
      buildCategoryMenu(state, category);
      if (menu) {
        menu.classList.add("is-open");
      }
    });
  });

  document.addEventListener("click", (event) => {
    if (!state.root.contains(event.target)) {
      closeCategoryMenu(state);
      return;
    }
    const clickedCat = event.target.closest?.("[data-editor-cat]");
    const clickedMenu = event.target.closest?.("[data-editor-cat-menu]");
    if (!clickedCat && !clickedMenu) {
      closeCategoryMenu(state);
    }
  });
}

function bindDialogs(state) {
  const newOpen = state.root.querySelector("[data-editor-new-open]");
  const newDialog = state.root.querySelector("[data-editor-new-dialog]");
  const newCancel = state.root.querySelector("[data-editor-new-cancel]");
  const newForm = state.root.querySelector("[data-editor-new-form]");

  if (newOpen && newDialog) {
    newOpen.addEventListener("click", () => {
      const folderField = newDialog.querySelector('input[name="virtual_path"]');
      if (folderField) {
        folderField.value = state.scopePath || "/";
      }
      newDialog.showModal();
    });
  }
  if (newCancel && newDialog) {
    newCancel.addEventListener("click", () => newDialog.close());
  }
  if (newForm && newDialog) {
    newForm.addEventListener("submit", async (event) => {
      event.preventDefault();
      const formData = new FormData(newForm);
      const triggerKind = String(formData.get("trigger_kind") || "webhook");
      const name = sanitizeSegment(formData.get("name"));
      const virtualPath = normalizeVirtualPath(formData.get("virtual_path"));
      const title = String(formData.get("title") || "");
      const description = String(formData.get("description") || "");
      const source = JSON.stringify(emptyPipelineGraph(name, triggerKind), null, 2);

      const payload = await requestJson(state.api.definition, {
        method: "POST",
        body: JSON.stringify({
          virtual_path: virtualPath,
          name,
          title,
          description,
          trigger_kind: triggerKind,
          source,
        }),
      });
      const id = payload?.meta?.file_rel_path;
      if (id) {
        const path = payload?.meta?.virtual_path || virtualPath;
        window.location.href = `/projects/${state.owner}/${state.project}/pipelines/editor?path=${encodeURIComponent(path)}&id=${encodeURIComponent(id)}`;
      }
      newDialog.close();
    });
  }

  const nodeDialog = state.root.querySelector("[data-editor-node-dialog]");
  const nodeCancel = state.root.querySelector("[data-editor-node-cancel]");
  if (nodeDialog && nodeCancel) {
    nodeCancel.addEventListener("click", () => {
      nodeDialog.classList.remove("is-fullscreen");
      nodeDialog.close();
    });
    nodeDialog.addEventListener("close", () => {
      nodeDialog.classList.remove("is-fullscreen");
    });
  }
}

function bindActions(state) {
  const saveBtn = state.root.querySelector("[data-editor-save]");
  const activateBtn = state.root.querySelector("[data-editor-activate]");
  const deactivateBtn = state.root.querySelector("[data-editor-deactivate]");

  if (saveBtn) {
    saveBtn.addEventListener("click", async () => {
      if (!state.currentMeta || !state.graphApp || state.currentLocked) {
        return;
      }
      const graph = collectGraphAsPipeline(state);
      const source = JSON.stringify(graph, null, 2);
      const payload = {
        virtual_path: state.currentMeta.virtual_path,
        name: state.currentMeta.name,
        title: state.currentMeta.title,
        description: state.currentMeta.description,
        trigger_kind: state.currentMeta.trigger_kind,
        source,
      };
      const result = await requestJson(state.api.definition, {
        method: "POST",
        body: JSON.stringify(payload),
      });
      const id = result?.meta?.file_rel_path || state.selectedId;
      const path = result?.meta?.virtual_path || state.currentMeta.virtual_path || state.scopePath || "/";
      window.location.href = `/projects/${state.owner}/${state.project}/pipelines/editor?path=${encodeURIComponent(path)}&id=${encodeURIComponent(id)}`;
    });
  }

  if (activateBtn) {
    activateBtn.addEventListener("click", async () => {
      if (!state.currentMeta || state.currentLocked) {
        return;
      }
      await requestJson(state.api.activate, {
        method: "POST",
        body: JSON.stringify({
          virtual_path: state.currentMeta.virtual_path,
          name: state.currentMeta.name,
        }),
      });
      window.location.reload();
    });
  }

  if (deactivateBtn) {
    deactivateBtn.addEventListener("click", async () => {
      if (!state.currentMeta || state.currentLocked) {
        return;
      }
      await requestJson(state.api.deactivate, {
        method: "POST",
        body: JSON.stringify({
          virtual_path: state.currentMeta.virtual_path,
          name: state.currentMeta.name,
        }),
      });
      window.location.reload();
    });
  }
}

async function initPipelineEditor(root) {
  await ensurePipelineEditorRuntime();
  const state = {
    root,
    owner: root.getAttribute("data-editor-owner") || "",
    project: root.getAttribute("data-editor-project") || "",
    scopePath: root.getAttribute("data-editor-scope-path") || "/",
    selectedId: root.getAttribute("data-editor-selected-id") || "",
    currentMeta: null,
    currentLocked: false,
    currentGraph: null,
    currentSource: "",
    graphApp: null,
    snapToGrid: root.getAttribute("data-editor-snap-grid") !== "false",
    nodeCatalog: new Map(),
    api: {
      byId: root.getAttribute("data-editor-api-by-id") || "",
      definition: root.getAttribute("data-editor-api-definition") || "",
      activate: root.getAttribute("data-editor-api-activate") || "",
      deactivate: root.getAttribute("data-editor-api-deactivate") || "",
      hits: root.getAttribute("data-editor-api-hits") || "",
      nodes: root.getAttribute("data-editor-api-nodes") || "",
      credentials: root.getAttribute("data-editor-api-credentials") || "",
      templatesWorkspace: root.getAttribute("data-editor-api-templates-workspace") || "",
      templateFile: root.getAttribute("data-editor-api-template-file") || "",
      templateSave: root.getAttribute("data-editor-api-template-save") || "",
    },
    pgCredentials: [],
    pageTemplates: [],
    nodeEditObserver: null,
  };

  bindDialogs(state);
  bindActions(state);
  bindCategoryButtons(state);

  try {
    const nodesPayload = await requestJson(state.api.nodes);
    state.nodeCatalog = createNodeCatalog(nodesPayload?.items || []);
  } catch (_err) {
    state.nodeCatalog = new Map();
  }

  if (state.api.credentials) {
    try {
      const credentialsPayload = await requestJson(state.api.credentials);
      const items = Array.isArray(credentialsPayload?.items)
        ? credentialsPayload.items
        : [];
      state.pgCredentials = items.filter((item) =>
        String(item?.kind || "").toLowerCase() === "postgres"
      );
    } catch (_err) {
      state.pgCredentials = [];
    }
  }

  if (state.api.templatesWorkspace) {
    try {
      const templateWorkspace = await requestJson(state.api.templatesWorkspace);
      const items = Array.isArray(templateWorkspace?.items)
        ? templateWorkspace.items
        : [];
      state.pageTemplates = items.filter((item) =>
        String(item?.kind || "").toLowerCase() === "file" &&
        String(item?.file_kind || "").toLowerCase() === "page"
      );
    } catch (_err) {
      state.pageTemplates = [];
    }
  }

  const firstId = resolveInitialPipelineId(root, state.selectedId);
  if (!firstId) {
    setLockedState(state);
    setHeaderInfo(state);
    setFootHits(state, null);
    return;
  }

  try {
    await loadPipeline(state, firstId);
  } catch (err) {
    console.error("pipeline editor init failed", err);
    const title = root.querySelector("[data-editor-selected-name]");
    const meta = root.querySelector("[data-editor-selected-meta]");
    if (title) {
      title.textContent = "Failed to load pipeline";
    }
    if (meta) {
      meta.textContent = err?.message || String(err);
    }
  }
}

const initializedRoots = new WeakSet();
let scanScheduled = false;

function scanPipelineEditors() {
  document.querySelectorAll('[data-pipeline-editor="true"]').forEach((root) => {
    if (initializedRoots.has(root)) {
      return;
    }
    initializedRoots.add(root);
    initPipelineEditor(root);
  });
}

export function initPipelineEditorBehavior() {
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
    scanPipelineEditors();
  };
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(run);
  } else {
    setTimeout(run, 0);
  }
}
