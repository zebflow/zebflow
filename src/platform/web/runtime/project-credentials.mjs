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

const SECRET_SCHEMAS = {
  postgres: [
    { key: "host", label: "Host", help: "Hostname or IP of PostgreSQL server." },
    { key: "port", label: "Port", placeholder: "5432", help: "TCP port for PostgreSQL." },
    { key: "database", label: "Database", help: "Database name." },
    { key: "user", label: "User", help: "Login username." },
    { key: "password", label: "Password", type: "password", help: "Login password." },
    { key: "sslmode", label: "SSL Mode", placeholder: "prefer", help: "disable, prefer, require, verify-ca, verify-full." },
  ],
  mysql: [
    { key: "host", label: "Host", help: "Hostname or IP of MySQL server." },
    { key: "port", label: "Port", placeholder: "3306", help: "TCP port for MySQL." },
    { key: "database", label: "Database", help: "Database name." },
    { key: "user", label: "User", help: "Login username." },
    { key: "password", label: "Password", type: "password", help: "Login password." },
  ],
  openai: [
    { key: "api_key", label: "API Key", type: "password", help: "Provider API token." },
    { key: "base_url", label: "Base URL", placeholder: "https://api.openai.com/v1", help: "Custom endpoint if needed." },
    { key: "model", label: "Default Model", help: "Fallback model id for requests." },
  ],
  http: [
    { key: "base_url", label: "Base URL", help: "Service root URL." },
    { key: "token", label: "Token", type: "password", help: "Bearer token or API key." },
  ],
  custom: [
    {
      key: "json",
      label: "Secret JSON",
      type: "textarea",
      rows: 10,
      placeholder: "{\n  \"key\": \"value\"\n}",
      help: "Stored as raw JSON object for custom nodes.",
    },
  ],
};

function formatTs(ts) {
  if (!Number.isFinite(Number(ts))) {
    return "-";
  }
  const value = Number(ts) * 1000;
  const dt = new Date(value);
  if (Number.isNaN(dt.getTime())) {
    return "-";
  }
  return dt.toISOString().slice(0, 19).replace("T", " ");
}

function toSecretRecord(secret) {
  if (secret && typeof secret === "object" && !Array.isArray(secret)) {
    return { ...secret };
  }
  return {};
}

function sanitizeCredentialId(raw) {
  return String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

function setStatus(state, message, tone = "info") {
  if (!state.statusEl) {
    return;
  }
  state.statusEl.textContent = message || "";
  state.statusEl.setAttribute("data-tone", tone);
}

function setBusy(state, isBusy) {
  state.busy = !!isBusy;
  state.form.querySelectorAll("input, textarea, select, button").forEach((el) => {
    if (el === state.cancelBtn) {
      el.disabled = false;
      return;
    }
    el.disabled = state.busy;
  });
}

function renderSecretFields(container, kind, secret = {}) {
  const schema = SECRET_SCHEMAS[kind] || SECRET_SCHEMAS.custom;
  const payload = toSecretRecord(secret);
  container.innerHTML = "";

  schema.forEach((field) => {
    const row = document.createElement("label");
    row.className = "pipeline-editor-field";

    const label = document.createElement("span");
    label.textContent = field.label;
    row.appendChild(label);

    let input;
    if (field.type === "textarea") {
      input = document.createElement("textarea");
      input.rows = Number(field.rows || 6);
      input.value = typeof payload[field.key] === "string" ? payload[field.key] : "";
    } else {
      input = document.createElement("input");
      input.type = field.type || "text";
      input.value = typeof payload[field.key] === "string" ? payload[field.key] : "";
    }
    input.setAttribute("data-secret-key", field.key);
    if (field.placeholder) {
      input.placeholder = field.placeholder;
    }
    row.appendChild(input);

    if (field.help) {
      const hint = document.createElement("small");
      hint.className = "pipeline-editor-field-help";
      hint.textContent = field.help;
      row.appendChild(hint);
    }

    container.appendChild(row);
  });
}

function collectSecret(container, kind) {
  if (kind === "custom") {
    const field = container.querySelector('[data-secret-key="json"]');
    const raw = String(field?.value || "").trim();
    if (!raw) {
      return {};
    }
    try {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        return parsed;
      }
      return { value: parsed };
    } catch (_err) {
      return { raw };
    }
  }

  const out = {};
  container.querySelectorAll("[data-secret-key]").forEach((field) => {
    const key = field.getAttribute("data-secret-key");
    if (!key) {
      return;
    }
    const value = String(field.value || "").trim();
    if (value) {
      out[key] = value;
    }
  });
  return out;
}

async function initCredentials(root) {
  const apiList = root.getAttribute("data-api-list") || "";
  const apiItemBase = root.getAttribute("data-api-item") || "";

  const rows = root.querySelector("[data-credential-rows]");
  const dialog = root.querySelector("[data-credential-dialog]");
  const form = root.querySelector("[data-credential-form]");
  const titleEl = root.querySelector("[data-credential-title]");
  const statusEl = root.querySelector("[data-credential-status]");
  const createBtn = document.querySelector("[data-credential-create]");
  const cancelBtn = root.querySelector("[data-credential-cancel]");
  const deleteBtn = root.querySelector("[data-credential-delete]");
  const saveBtn = root.querySelector("[data-credential-save]");
  const kindField = root.querySelector("[data-credential-kind]");
  const idField = root.querySelector("[data-credential-id]");
  const fieldsWrap = root.querySelector("[data-credential-secret-fields]");

  if (
    !rows || !dialog || !form || !kindField || !idField || !fieldsWrap || !apiList || !apiItemBase ||
    !titleEl || !statusEl || !cancelBtn || !deleteBtn || !saveBtn
  ) {
    return;
  }

  const state = {
    items: [],
    mode: "create",
    currentId: "",
    secret: {},
    busy: false,
    rows,
    dialog,
    form,
    titleEl,
    statusEl,
    cancelBtn,
    deleteBtn,
    saveBtn,
    kindField,
    idField,
    fieldsWrap,
    apiList,
    apiItemBase,
  };

  async function loadList() {
    const payload = await requestJson(state.apiList);
    state.items = Array.isArray(payload?.items) ? payload.items : [];
    state.rows.innerHTML = "";

    if (state.items.length === 0) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 6;
      td.textContent = "No credentials yet";
      tr.appendChild(td);
      state.rows.appendChild(tr);
      return;
    }

    state.items.forEach((item) => {
      const tr = document.createElement("tr");
      tr.innerHTML = `
        <td>${item.credential_id || ""}</td>
        <td>${item.title || ""}</td>
        <td>${item.kind || ""}</td>
        <td>${item.has_secret ? "yes" : "no"}</td>
        <td>${formatTs(item.updated_at)}</td>
        <td><button type="button" class="project-inline-chip" data-edit-id="${item.credential_id || ""}">Edit</button></td>
      `;
      state.rows.appendChild(tr);
    });

    state.rows.querySelectorAll("[data-edit-id]").forEach((button) => {
      button.addEventListener("click", () => {
        const id = button.getAttribute("data-edit-id") || "";
        openEdit(id).catch((err) => {
          setStatus(state, `Failed to load credential: ${err?.message || String(err)}`, "error");
        });
      });
    });
  }

  function openCreate() {
    state.mode = "create";
    state.currentId = "";
    state.secret = {};
    state.form.reset();
    state.idField.disabled = false;
    state.idField.value = "";
    state.titleEl.textContent = "Create Credential";
    state.deleteBtn.style.display = "none";
    setBusy(state, false);
    setStatus(state, "Fill fields and save.", "info");
    renderSecretFields(state.fieldsWrap, String(state.kindField.value || "custom"), state.secret);
    state.dialog.showModal();
  }

  async function openEdit(credentialId) {
    const id = String(credentialId || "").trim();
    if (!id) {
      return;
    }

    state.mode = "edit";
    state.currentId = id;
    state.secret = {};
    state.form.reset();
    state.idField.value = id;
    state.idField.disabled = true;
    state.titleEl.textContent = `Edit Credential | ${id}`;
    state.deleteBtn.style.display = "inline-flex";
    setStatus(state, "Loading credential details...", "info");
    renderSecretFields(state.fieldsWrap, String(state.kindField.value || "custom"), state.secret);
    setBusy(state, true);
    state.dialog.showModal();

    try {
      const payload = await requestJson(`${state.apiItemBase}/${encodeURIComponent(id)}`);
      const item = payload?.credential || payload?.item;
      if (!item) {
        throw new Error("Credential payload missing");
      }

      state.currentId = item.credential_id || id;
      state.secret = toSecretRecord(item.secret);
      state.idField.value = state.currentId;

      const titleField = state.form.elements.namedItem("title");
      const kindInput = state.form.elements.namedItem("kind");
      const notesField = state.form.elements.namedItem("notes");
      if (titleField) {
        titleField.value = item.title || "";
      }
      if (kindInput) {
        kindInput.value = item.kind || "custom";
      }
      if (notesField) {
        notesField.value = item.notes || "";
      }

      renderSecretFields(state.fieldsWrap, String(state.kindField.value || "custom"), state.secret);
      setStatus(state, "Loaded. Update fields and save.", "ok");
    } catch (err) {
      setStatus(state, `Failed to load credential: ${err?.message || String(err)}`, "error");
    } finally {
      setBusy(state, false);
    }
  }

  state.kindField.addEventListener("change", () => {
    const activeKind = String(state.kindField.value || "custom");
    state.secret = collectSecret(state.fieldsWrap, activeKind);
    renderSecretFields(state.fieldsWrap, activeKind, state.secret);
    setStatus(state, `Editing ${activeKind} credential fields.`, "info");
  });

  state.form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (state.busy) {
      return;
    }

    const formData = new FormData(state.form);
    const kind = String(formData.get("kind") || "custom");
    const credentialId = state.mode === "edit"
      ? state.currentId
      : sanitizeCredentialId(formData.get("credential_id"));

    if (!credentialId) {
      setStatus(state, "Credential ID is required.", "error");
      return;
    }

    const payload = {
      credential_id: credentialId,
      title: String(formData.get("title") || "").trim(),
      kind,
      notes: String(formData.get("notes") || "").trim(),
      secret: collectSecret(state.fieldsWrap, kind),
    };

    if (!payload.title) {
      setStatus(state, "Title is required.", "error");
      return;
    }

    setBusy(state, true);
    setStatus(state, "Saving credential...", "info");
    try {
      if (state.mode === "edit") {
        await requestJson(`${state.apiItemBase}/${encodeURIComponent(payload.credential_id)}`, {
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

  state.deleteBtn.addEventListener("click", async () => {
    if (state.mode !== "edit" || !state.currentId || state.busy) {
      return;
    }
    setBusy(state, true);
    setStatus(state, "Deleting credential...", "info");
    try {
      await requestJson(`${state.apiItemBase}/${encodeURIComponent(state.currentId)}`, {
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
    createBtn.addEventListener("click", () => openCreate());
  }

  await loadList();
}

document.querySelectorAll("[data-project-credentials='true']").forEach((root) => {
  initCredentials(root).catch((err) => {
    console.error("credentials ui failed", err);
  });
});
