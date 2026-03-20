function requestJson(url, options = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (response) => {
    if (response.status === 401) { window.location.href = "/login"; return null; }
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

function formatTs(ts) {
  if (!Number.isFinite(Number(ts))) {
    return "-";
  }
  const dt = new Date(Number(ts) * 1000);
  if (Number.isNaN(dt.getTime())) {
    return "-";
  }
  return dt.toISOString().slice(0, 19).replace("T", " ");
}

function slugify(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

function normalizeCredentialId(value) {
  return slugify(value);
}

function dbKindIconClass(kind) {
  const value = String(kind || "").trim().toLowerCase();
  if (value === "postgresql" || value === "postgres" || value === "pg") {
    return "devicon-postgresql-plain colored";
  }
  if (value === "mysql") {
    return "devicon-mysql-plain colored";
  }
  if (value === "sqlite") {
    return "devicon-sqlite-plain colored";
  }
  if (value === "redis") {
    return "devicon-redis-plain colored";
  }
  if (value === "mongodb") {
    return "devicon-mongodb-plain colored";
  }
  if (value === "qdrant") {
    return "devicon-vectorlogozone-plain";
  }
  if (value === "sekejap") {
    return "zf-icon-sjtable";
  }
  return "zf-icon-default-db";
}

function generateCredentialSlug(kind, existingIds = []) {
  const used = new Set(
    (Array.isArray(existingIds) ? existingIds : [])
      .map((value) => slugify(value))
      .filter(Boolean)
  );
  const base = slugify(`${kind || "credential"}-main`) || "credential";
  if (!used.has(base)) {
    return base;
  }
  let index = 1;
  while (used.has(`${base}-${index}`)) {
    index += 1;
  }
  return `${base}-${index}`;
}

function credentialKindForDatabaseKind(databaseKind) {
  const kind = String(databaseKind || "").trim().toLowerCase();
  if (kind === "postgresql") {
    return "postgres";
  }
  if (kind === "mysql") {
    return "mysql";
  }
  return "";
}

function isCredentialRequired(databaseKind) {
  return credentialKindForDatabaseKind(databaseKind) !== "";
}

function setStatus(state, message, tone = "info") {
  state.statusEl.textContent = message || "";
  state.statusEl.setAttribute("data-tone", tone);
}

function setBusy(state, isBusy) {
  state.busy = !!isBusy;
  state.form.querySelectorAll("input, select, textarea, button").forEach((el) => {
    if (el === state.cancelBtn) {
      el.disabled = false;
      return;
    }
    el.disabled = state.busy;
  });
}

function setCredentialBusy(state, isBusy) {
  state.credentialBusy = !!isBusy;
  if (!state.credentialForm) {
    return;
  }
  state.credentialForm.querySelectorAll("input, select, textarea, button").forEach((el) => {
    if (el === state.credentialCancelBtn) {
      el.disabled = false;
      return;
    }
    el.disabled = state.credentialBusy;
  });
}

function parseConfig(raw) {
  const trimmed = String(raw || "").trim();
  if (!trimmed) {
    return {};
  }
  return JSON.parse(trimmed);
}

function parseSecretJson(raw) {
  const trimmed = String(raw || "").trim();
  if (!trimmed) {
    return {};
  }
  const parsed = JSON.parse(trimmed);
  if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
    return parsed;
  }
  return { value: parsed };
}

function defaultSecretTemplate(kind) {
  if (kind === "postgres") {
    return {
      host: "localhost",
      port: "5432",
      database: "postgres",
      user: "postgres",
      password: "",
      sslmode: "prefer",
    };
  }
  if (kind === "mysql") {
    return {
      host: "localhost",
      port: "3306",
      database: "mysql",
      user: "root",
      password: "",
    };
  }
  return {};
}

function filteredCredentialsForKind(credentials, databaseKind) {
  const requiredKind = credentialKindForDatabaseKind(databaseKind);
  if (!requiredKind) {
    return credentials.slice();
  }
  return credentials.filter((item) => String(item.kind || "").toLowerCase() === requiredKind);
}

function renderCredentialSelect(state, preferredCredentialId = "") {
  const dbKind = String(state.kindField.value || "sekejap").trim().toLowerCase();
  const required = isCredentialRequired(dbKind);
  const compatible = filteredCredentialsForKind(state.credentials, dbKind);
  const previous = normalizeCredentialId(preferredCredentialId || state.credentialField.value || "");

  state.credentialField.innerHTML = "";

  if (!required) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "None";
    state.credentialField.appendChild(option);
  }

  compatible.forEach((item) => {
    const credentialId = String(item.credential_id || "").trim();
    if (!credentialId) {
      return;
    }
    const option = document.createElement("option");
    option.value = credentialId;
    const label = String(item.title || "").trim();
    option.textContent = label
      ? `${label} (${item.kind || "custom"})`
      : `Credential (${item.kind || "custom"})`;
    state.credentialField.appendChild(option);
  });

  const hasPreferred = !!previous && compatible.some((item) => String(item.credential_id || "") === previous);
  if (hasPreferred) {
    state.credentialField.value = previous;
  } else if (!required) {
    state.credentialField.value = "";
  } else if (compatible.length > 0) {
    state.credentialField.value = String(compatible[0].credential_id || "");
  } else {
    state.credentialField.value = "";
  }

  const disableCredential = dbKind === "sekejap";
  state.credentialField.disabled = disableCredential;

  if (disableCredential) {
    state.credentialHelp.textContent = "SekejapDB does not use credentials.";
    state.inlineCreateCredentialBtn.disabled = true;
    state.inlineRefreshCredentialBtn.disabled = true;
    return;
  }

  state.inlineRefreshCredentialBtn.disabled = false;
  state.inlineCreateCredentialBtn.disabled = false;

  if (required && compatible.length === 0) {
    state.credentialHelp.textContent = `No compatible credential found for ${dbKind}. Create one, then test.`;
    return;
  }

  if (required) {
    state.credentialHelp.textContent = `Select one ${credentialKindForDatabaseKind(dbKind)} credential.`;
    return;
  }

  state.credentialHelp.textContent = "Select an optional credential for this connection.";
}

async function initDbConnections(root) {
  const runtimeNode = root.querySelector("#project-db-connections-runtime");
  let runtimeConfig = null;
  try {
    runtimeConfig = runtimeNode ? JSON.parse(runtimeNode.textContent || "{}") : null;
  } catch (_err) {
    runtimeConfig = null;
  }
  const owner = String(runtimeConfig?.owner || "");
  const project = String(runtimeConfig?.project || "");
  const apiList = String(runtimeConfig?.api?.list || "");
  const apiItemBase = String(runtimeConfig?.api?.item_base || "");
  const apiTest = String(runtimeConfig?.api?.test || "");
  const apiCredentials = String(runtimeConfig?.api?.credentials_list || "");

  const rows = root.querySelector("[data-db-connection-rows]");
  const dialog = root.querySelector("[data-db-connection-dialog]");
  const form = root.querySelector("[data-db-connection-form]");
  const titleEl = root.querySelector("[data-db-connection-title]");
  const statusEl = root.querySelector("[data-db-connection-status]");
  const createBtn = document.querySelector("[data-db-connection-create]");
  const cancelBtn = root.querySelector("[data-db-connection-cancel]");
  const deleteBtn = root.querySelector("[data-db-connection-delete]");
  const saveBtn = root.querySelector("[data-db-connection-save]");
  const testBtn = root.querySelector("[data-db-connection-test]");
  const slugField = root.querySelector("[data-db-connection-slug]");
  const kindField = root.querySelector("[data-db-connection-kind]");
  const credentialField = root.querySelector("[data-db-connection-credential-id]");
  const credentialHelp = root.querySelector("[data-db-connection-credential-help]");
  const inlineCreateCredentialBtn = root.querySelector("[data-db-credential-create-inline]");
  const inlineRefreshCredentialBtn = root.querySelector("[data-db-credential-refresh-inline]");
  const configJsonField = root.querySelector("[data-db-connection-config-json]");

  const credentialDialog = root.querySelector("[data-db-credential-dialog]");
  const credentialForm = root.querySelector("[data-db-credential-form]");
  const credentialStatusEl = root.querySelector("[data-db-credential-status]");
  const credentialIdField = root.querySelector("[data-db-credential-id]");
  const credentialTitleField = root.querySelector("[data-db-credential-title]");
  const credentialKindField = root.querySelector("[data-db-credential-kind]");
  const credentialSecretJsonField = root.querySelector("[data-db-credential-secret-json]");
  const credentialCancelBtn = root.querySelector("[data-db-credential-cancel]");

  if (
    !rows ||
    !dialog ||
    !form ||
    !titleEl ||
    !statusEl ||
    !cancelBtn ||
    !deleteBtn ||
    !saveBtn ||
    !testBtn ||
    !slugField ||
    !kindField ||
    !credentialField ||
    !credentialHelp ||
    !inlineCreateCredentialBtn ||
    !inlineRefreshCredentialBtn ||
    !configJsonField ||
    !apiList ||
    !apiItemBase ||
    !apiTest ||
    !apiCredentials ||
    !credentialDialog ||
    !credentialForm ||
    !credentialStatusEl ||
    !credentialIdField ||
    !credentialTitleField ||
    !credentialKindField ||
    !credentialSecretJsonField ||
    !credentialCancelBtn
  ) {
    return;
  }

  const state = {
    owner,
    project,
    apiList,
    apiItemBase,
    apiTest,
    apiCredentials,
    rows,
    dialog,
    form,
    titleEl,
    statusEl,
    cancelBtn,
    deleteBtn,
    saveBtn,
    testBtn,
    slugField,
    kindField,
    credentialField,
    credentialHelp,
    inlineCreateCredentialBtn,
    inlineRefreshCredentialBtn,
    configJsonField,
    credentialDialog,
    credentialForm,
    credentialStatusEl,
    credentialIdField,
    credentialTitleField,
    credentialKindField,
    credentialSecretJsonField,
    credentialCancelBtn,
    busy: false,
    credentialBusy: false,
    mode: "create",
    currentSlug: "",
    items: [],
    credentials: [],
    credentialById: new Map(),
    reopenDbDialogAfterCredential: false,
  };

  function setCredentialStatus(message, tone = "info") {
    state.credentialStatusEl.textContent = message || "";
    state.credentialStatusEl.setAttribute("data-tone", tone);
  }

  async function loadCredentials(preferredCredentialId = "") {
    try {
      const payload = await requestJson(state.apiCredentials);
      state.credentials = Array.isArray(payload?.items) ? payload.items : [];
      state.credentialById = new Map(
        state.credentials.map((item) => [String(item.credential_id || ""), item])
      );
    } catch (_err) {
      state.credentials = [];
      state.credentialById = new Map();
    }
    renderCredentialSelect(state, preferredCredentialId);
  }

  async function loadList() {
    const payload = await requestJson(state.apiList);
    state.items = Array.isArray(payload?.items) ? payload.items : [];
    state.rows.innerHTML = "";
    if (state.items.length === 0) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 6;
      td.textContent = "No database connections yet";
      tr.appendChild(td);
      state.rows.appendChild(tr);
      return;
    }

    state.items.forEach((item) => {
      const slug = String(item.connection_slug || "");
      const kind = String(item.database_kind || "sekejap");
      const iconClass = dbKindIconClass(kind);
      const openPath = `/projects/${encodeURIComponent(owner)}/${encodeURIComponent(project)}/db/${encodeURIComponent(kind)}/${encodeURIComponent(slug)}/tables`;
      const tr = document.createElement("tr");
      const credential = state.credentialById.get(String(item.credential_id || ""));
      const credentialLabel = credential
        ? String(credential.title || "").trim() || String(credential.credential_id || "")
        : item.credential_id || "-";
      tr.innerHTML = `
        <td>
          <span class="db-connection-name">
            <i class="zf-devicon ${iconClass}" aria-hidden="true"></i>
            <span>${slug}</span>
          </span>
        </td>
        <td>${item.connection_label || ""}</td>
        <td>
          <span class="db-connection-kind">
            <i class="zf-devicon ${iconClass}" aria-hidden="true"></i>
            <span>${kind}</span>
          </span>
        </td>
        <td>${credentialLabel || "-"}</td>
        <td>${formatTs(item.updated_at)}</td>
        <td>
          <a href="${openPath}" class="project-inline-chip">Open</a>
          <button type="button" class="project-inline-chip" data-edit-slug="${slug}">Edit</button>
        </td>
      `;
      state.rows.appendChild(tr);
    });

    state.rows.querySelectorAll("[data-edit-slug]").forEach((button) => {
      button.addEventListener("click", () => {
        const slug = button.getAttribute("data-edit-slug") || "";
        openEdit(slug).catch((err) => {
          setStatus(state, `Failed to load connection: ${err?.message || String(err)}`, "error");
        });
      });
    });
  }

  async function openCreate() {
    state.mode = "create";
    state.currentSlug = "";
    state.form.reset();
    state.slugField.disabled = false;
    state.kindField.value = "sekejap";
    state.configJsonField.value = "{}";
    state.titleEl.textContent = "Create Database Connection";
    state.deleteBtn.style.display = "none";
    setBusy(state, false);
    await loadCredentials();
    setStatus(state, "Fill fields and save.", "info");
    state.dialog.showModal();
  }

  async function openEdit(slug) {
    const normalized = slugify(slug);
    if (!normalized) {
      return;
    }
    state.mode = "edit";
    state.currentSlug = normalized;
    state.form.reset();
    state.slugField.value = normalized;
    state.slugField.disabled = true;
    state.deleteBtn.style.display = normalized === "default" ? "none" : "inline-flex";
    state.titleEl.textContent = `Edit DB Connection | ${normalized}`;
    setStatus(state, "Loading connection details...", "info");
    setBusy(state, true);
    state.dialog.showModal();

    try {
      const payload = await requestJson(`${state.apiItemBase}/${encodeURIComponent(normalized)}`);
      const item = payload?.connection;
      if (!item) {
        throw new Error("Connection payload missing");
      }
      state.currentSlug = item.connection_slug || normalized;
      state.slugField.value = state.currentSlug;
      state.form.elements.namedItem("connection_label").value = item.connection_label || "";
      state.kindField.value = item.database_kind || "sekejap";
      state.configJsonField.value = JSON.stringify(item.config || {}, null, 2);
      await loadCredentials(item.credential_id || "");
      setStatus(state, "Loaded. Update fields and save.", "ok");
    } finally {
      setBusy(state, false);
    }
  }

  function openCreateCredentialDialog() {
    const dbKind = String(state.kindField.value || "sekejap").trim().toLowerCase();
    const credentialKind = credentialKindForDatabaseKind(dbKind) || "custom";

    const generatedId = generateCredentialSlug(
      credentialKind,
      state.credentials.map((item) => item?.credential_id || "")
    );
    const suggestedTitle = credentialKind === "custom"
      ? "Custom Credential"
      : `${credentialKind.toUpperCase()} Credential`;

    const dbDialogWasOpen = state.dialog.open;
    state.reopenDbDialogAfterCredential = dbDialogWasOpen;
    if (dbDialogWasOpen) {
      try {
        state.dialog.close("open-credential-dialog");
      } catch (_err) {
      }
    }

    state.credentialForm.reset();
    state.credentialKindField.value = credentialKind;
    state.credentialIdField.value = generatedId;
    state.credentialTitleField.value = suggestedTitle;
    state.credentialSecretJsonField.value = JSON.stringify(defaultSecretTemplate(credentialKind), null, 2);
    setCredentialStatus("Create credential then continue with connection test.", "info");
    setCredentialBusy(state, false);
    state.credentialDialog.showModal();
  }

  state.kindField.addEventListener("change", () => {
    renderCredentialSelect(state);
  });

  state.inlineRefreshCredentialBtn.addEventListener("click", async () => {
    await loadCredentials();
    setStatus(state, "Credentials refreshed.", "info");
  });

  state.inlineCreateCredentialBtn.addEventListener("click", () => {
    openCreateCredentialDialog();
  });

  state.credentialKindField.addEventListener("change", () => {
    const selected = String(state.credentialKindField.value || "custom");
    state.credentialSecretJsonField.value = JSON.stringify(defaultSecretTemplate(selected), null, 2);
  });

  state.credentialForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (state.credentialBusy) {
      return;
    }

    const formData = new FormData(state.credentialForm);
    const credentialId = normalizeCredentialId(formData.get("credential_id"));
    const title = String(formData.get("title") || "").trim();
    const kind = String(formData.get("kind") || "custom").trim();

    if (!credentialId) {
      setCredentialStatus("Credential ID is required.", "error");
      return;
    }
    if (!title) {
      setCredentialStatus("Credential title is required.", "error");
      return;
    }

    let secret;
    try {
      secret = parseSecretJson(formData.get("secret_json"));
    } catch (err) {
      setCredentialStatus(`Invalid secret JSON: ${err?.message || String(err)}`, "error");
      return;
    }

    const payload = {
      credential_id: credentialId,
      title,
      kind,
      notes: "Created from DB connection dialog",
      secret,
    };

    setCredentialBusy(state, true);
    setCredentialStatus("Creating credential...", "info");
    try {
      await requestJson(state.apiCredentials, {
        method: "POST",
        body: JSON.stringify(payload),
      });
      await loadCredentials(credentialId);
      setStatus(state, "Credential created and selected.", "ok");
      state.credentialDialog.close();
      if (state.reopenDbDialogAfterCredential) {
        state.reopenDbDialogAfterCredential = false;
        state.dialog.showModal();
      }
    } catch (err) {
      setCredentialStatus(`Create failed: ${err?.message || String(err)}`, "error");
    } finally {
      setCredentialBusy(state, false);
    }
  });

  state.credentialCancelBtn.addEventListener("click", () => {
    state.credentialDialog.close();
    if (state.reopenDbDialogAfterCredential) {
      state.reopenDbDialogAfterCredential = false;
      state.dialog.showModal();
    }
  });

  const labelField = state.form.elements.namedItem("connection_label");
  if (labelField) {
    labelField.addEventListener("input", () => {
      if (state.mode === "create" && !state.slugField.value.trim()) {
        state.slugField.value = slugify(labelField.value);
      }
    });
  }

  state.form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (state.busy) {
      return;
    }
    const formData = new FormData(state.form);
    const slug = state.mode === "edit" ? state.currentSlug : slugify(formData.get("connection_slug"));
    if (!slug) {
      setStatus(state, "Connection slug is required.", "error");
      return;
    }

    let config;
    try {
      config = parseConfig(formData.get("config_json"));
    } catch (err) {
      setStatus(state, `Invalid config JSON: ${err?.message || String(err)}`, "error");
      return;
    }

    const databaseKind = String(formData.get("database_kind") || "sekejap").trim();
    const credentialId = normalizeCredentialId(formData.get("credential_id"));

    if (isCredentialRequired(databaseKind) && !credentialId) {
      setStatus(state, `Credential is required for ${databaseKind}.`, "error");
      return;
    }

    const payload = {
      connection_slug: slug,
      connection_label: String(formData.get("connection_label") || "").trim(),
      database_kind: databaseKind,
      credential_id: credentialId || null,
      config,
    };

    if (!payload.connection_label) {
      setStatus(state, "Connection label is required.", "error");
      return;
    }

    setBusy(state, true);
    setStatus(state, "Saving connection...", "info");
    try {
      if (state.mode === "edit") {
        await requestJson(`${state.apiItemBase}/${encodeURIComponent(slug)}`, {
          method: "PUT",
          body: JSON.stringify(payload),
        });
      } else {
        await requestJson(state.apiList, {
          method: "POST",
          body: JSON.stringify(payload),
        });
      }
      state.dialog.close();
      await loadList();
    } catch (err) {
      setStatus(state, `Save failed: ${err?.message || String(err)}`, "error");
    } finally {
      setBusy(state, false);
    }
  });

  state.testBtn.addEventListener("click", async () => {
    if (state.busy) {
      return;
    }
    const formData = new FormData(state.form);
    const slug = state.mode === "edit" ? state.currentSlug : slugify(formData.get("connection_slug"));

    let config = {};
    try {
      config = parseConfig(formData.get("config_json"));
    } catch (err) {
      setStatus(state, `Invalid config JSON: ${err?.message || String(err)}`, "error");
      return;
    }

    const databaseKind = String(formData.get("database_kind") || "sekejap").trim();
    const credentialId = normalizeCredentialId(formData.get("credential_id"));
    if (isCredentialRequired(databaseKind) && !credentialId) {
      setStatus(state, `Credential is required for ${databaseKind}.`, "error");
      return;
    }

    const payload = state.mode === "edit" && slug
      ? { connection_slug: slug }
      : {
        database_kind: databaseKind,
        credential_id: credentialId || null,
        config,
      };

    setBusy(state, true);
    setStatus(state, "Testing connection...", "info");
    try {
      const response = await requestJson(state.apiTest, {
        method: "POST",
        body: JSON.stringify(payload),
      });
      const message = response?.result?.message || "Connection test passed";
      setStatus(state, message, "ok");
    } catch (err) {
      setStatus(state, `Test failed: ${err?.message || String(err)}`, "error");
    } finally {
      setBusy(state, false);
    }
  });

  state.deleteBtn.addEventListener("click", async () => {
    if (state.mode !== "edit" || !state.currentSlug || state.busy) {
      return;
    }
    setBusy(state, true);
    setStatus(state, "Deleting connection...", "info");
    try {
      await requestJson(`${state.apiItemBase}/${encodeURIComponent(state.currentSlug)}`, {
        method: "DELETE",
      });
      state.dialog.close();
      await loadList();
    } catch (err) {
      setStatus(state, `Delete failed: ${err?.message || String(err)}`, "error");
    } finally {
      setBusy(state, false);
    }
  });

  state.cancelBtn.addEventListener("click", () => state.dialog.close());

  if (createBtn) {
    createBtn.addEventListener("click", () => {
      openCreate().catch((err) => {
        setStatus(state, `Cannot open dialog: ${err?.message || String(err)}`, "error");
      });
    });
  }

  await loadCredentials();
  await loadList();
}

const initializedRoots = new WeakSet();
let scanScheduled = false;

function scanDbConnectionRoots() {
  document.querySelectorAll("[data-project-db-connections='true']").forEach((root) => {
    if (initializedRoots.has(root)) {
      return;
    }
    initializedRoots.add(root);
    initDbConnections(root).catch((err) => {
      console.error("db connections ui failed", err);
    });
  });
}

export function initProjectDbConnectionsBehavior() {
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
    scanDbConnectionRoots();
  };
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(run);
  } else {
    setTimeout(run, 0);
  }
}
