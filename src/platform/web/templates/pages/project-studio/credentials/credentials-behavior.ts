import { studioTableTdClass } from "@/components/ui/studio-data-table";

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

const SECRET_SCHEMAS = {
  postgres: [
    { key: "host",     label: "Host",     help: "Hostname or IP of PostgreSQL server." },
    { key: "port",     label: "Port",     placeholder: "5432", help: "TCP port for PostgreSQL." },
    { key: "database", label: "Database", help: "Database name." },
    { key: "user",     label: "User",     help: "Login username." },
    { key: "password", label: "Password", type: "password", help: "Login password." },
    { key: "sslmode",  label: "SSL Mode", placeholder: "prefer", help: "disable, prefer, require, verify-ca, verify-full." },
  ],
  mysql: [
    { key: "host",     label: "Host",     help: "Hostname or IP of MySQL server." },
    { key: "port",     label: "Port",     placeholder: "3306", help: "TCP port for MySQL." },
    { key: "database", label: "Database", help: "Database name." },
    { key: "user",     label: "User",     help: "Login username." },
    { key: "password", label: "Password", type: "password", fullWidth: true, help: "Login password." },
  ],
  openai: [
    { key: "api_key",  label: "API Key", type: "password", fullWidth: true, help: "Provider API token." },
    { key: "base_url", label: "Base URL", placeholder: "https://api.openai.com/v1", default: "https://api.openai.com/v1", fullWidth: true, help: "Provider API root. Paths below are joined to this URL." },
    { key: "model", label: "Default Model", placeholder: "gpt-5.5", fullWidth: true, help: "Fallback model id for requests." },
    { key: "response_surface", label: "Response Surface", type: "select", default: "responses", options: [
      { value: "responses", label: "Responses API" },
      { value: "chat_completions", label: "Chat Completions API" },
    ], help: "Protocol shape used for request payloads, tool calls, and response parsing." },
    { key: "response_path", label: "Response Path", placeholder: "/responses", default: "/responses", help: "Path appended to Base URL for text/tool responses." },
    { key: "embedding_path", label: "Embedding Path", placeholder: "/embeddings", default: "/embeddings", help: "Path appended to Base URL for embeddings." },
    { key: "store", label: "Provider Storage", type: "select", default: "false", options: [
      { value: "false", label: "Do not store" },
      { value: "true", label: "Allow provider storage" },
    ], help: "Responses API store flag. Zebflow defaults to no provider-side response storage." },
  ],
  openrouter: [
    { key: "api_key",  label: "API Key", type: "password", fullWidth: true, help: "OpenRouter API token." },
    { key: "base_url", label: "Base URL", placeholder: "https://openrouter.ai/api/v1", default: "https://openrouter.ai/api/v1", fullWidth: true, help: "Provider API root. Paths below are joined to this URL." },
    { key: "model", label: "Default Model", placeholder: "openai/gpt-4o-mini", fullWidth: true, help: "Fallback model id for requests." },
    { key: "response_surface", label: "Response Surface", type: "select", default: "chat_completions", options: [
      { value: "chat_completions", label: "Chat Completions API" },
      { value: "responses", label: "Responses API" },
    ], help: "Protocol shape used for request payloads, tool calls, and response parsing." },
    { key: "response_path", label: "Response Path", placeholder: "/chat/completions", default: "/chat/completions", help: "Path appended to Base URL for text/tool responses." },
    { key: "embedding_path", label: "Embedding Path", placeholder: "/embeddings", default: "/embeddings", help: "Path appended to Base URL for embeddings if the provider/model supports them." },
  ],
  http: [
    { key: "base_url", label: "Base URL", help: "Service root URL." },
    { key: "token",    label: "Token",    type: "password", help: "Bearer token or API key." },
  ],
  github: [
    { key: "username",  label: "GitHub Username", help: "Your GitHub username for API auth and git push." },
    { key: "token",     label: "Personal Access Token", type: "password", fullWidth: true, help: "PAT with repo scope. Starts with ghp_ or github_pat_." },
    { key: "git_name",  label: "Git Name",  help: "Full name for git commits (git config user.name)." },
    { key: "git_email", label: "Git Email", help: "Email for git commits (git config user.email). Must match GitHub account." },
  ],
  gitlab: [
    { key: "url",       label: "Instance URL", placeholder: "https://gitlab.com", fullWidth: true, help: "GitLab instance URL. Use https://gitlab.com for SaaS." },
    { key: "username",  label: "GitLab Username", help: "Your GitLab username for API auth and git push." },
    { key: "token",     label: "Personal Access Token", type: "password", fullWidth: true, help: "PAT with read_repository and write_repository scope." },
    { key: "git_name",  label: "Git Name",  help: "Full name for git commits (git config user.name)." },
    { key: "git_email", label: "Git Email", help: "Email for git commits (git config user.email)." },
  ],
  jwt_signing_key: [
    {
      key: "algorithm",
      label: "Algorithm",
      type: "select",
      options: [
        { value: "HS256", label: "HS256 — HMAC-SHA256 (symmetric)" },
        { value: "HS384", label: "HS384 — HMAC-SHA384 (symmetric)" },
        { value: "HS512", label: "HS512 — HMAC-SHA512 (symmetric)" },
        { value: "RS256", label: "RS256 — RSA-PKCS1v15-SHA256 (asymmetric)" },
        { value: "RS384", label: "RS384 — RSA-PKCS1v15-SHA384 (asymmetric)" },
        { value: "RS512", label: "RS512 — RSA-PKCS1v15-SHA512 (asymmetric)" },
        { value: "ES256", label: "ES256 — ECDSA P-256 (asymmetric)" },
        { value: "ES384", label: "ES384 — ECDSA P-384 (asymmetric)" },
      ],
      default: "HS256",
      help: "JWT signing algorithm. HS* uses a shared secret; RS*/ES* use a private key.",
    },
    { key: "secret", label: "HMAC Secret", type: "password", fullWidth: true, generate: "random_hex_32", help: "Secret for HS* algorithms. Click Generate for a secure 256-bit random value." },
    { key: "private_key", label: "Private Key (PEM)", type: "textarea", rows: 6, fullWidth: true, help: "PEM private key for RS*/ES* algorithms. Leave blank for HS*." },
    { key: "auth_redirect", label: "Unauthenticated Redirect", placeholder: "/login", help: "Where to redirect when the token is missing or invalid. Leave blank to return 401 JSON." },
    { key: "auth_forbidden_redirect", label: "Forbidden Redirect", placeholder: "/403", help: "Where to redirect when the token is valid but the role is insufficient. Leave blank to return 403 JSON." },
    { key: "auth_roles", label: "Allowed Roles", type: "tags", fullWidth: true, placeholder: "e.g. admin", help: "Roles available for this credential. Used by webhook nodes to populate the Required Role checkboxes." },
  ],
  browser_browserless: [
    { key: "url", label: "URL", placeholder: "http://localhost:3000", fullWidth: true, help: "Browserless instance root URL. Self-hosted or cloud endpoint." },
    { key: "token", label: "Token", type: "password", fullWidth: true, help: "Optional API token. Leave blank for unauthenticated self-hosted instances." },
  ],
  custom: [
    {
      key: "json",
      label: "Secret JSON",
      type: "textarea",
      rows: 10,
      fullWidth: true,
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

function createHelpTooltip(text: string): HTMLElement {
  const wrapper = document.createElement("span");
  wrapper.className = "group relative inline-flex items-center outline-none";
  (wrapper as any).tabIndex = 0;
  wrapper.setAttribute("aria-label", text);
  const icon = document.createElement("span");
  icon.className = "inline-flex text-ui-text-muted transition-colors duration-150 group-hover:text-ui-text group-focus-within:text-ui-text";
  icon.setAttribute("aria-hidden", "true");
  icon.innerHTML = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.8"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/><circle cx="12" cy="17" r="0.5" fill="currentColor" stroke="currentColor" stroke-width="1.5"/></svg>`;
  const content = document.createElement("span");
  content.className = "pointer-events-none invisible absolute bottom-[calc(100%+8px)] left-1/2 z-50 min-w-40 max-w-60 -translate-x-1/2 rounded-md border border-ui-border bg-gray-800 px-2.5 py-1.5 text-[11px] font-normal leading-[1.5] tracking-normal text-gray-100 opacity-0 shadow-lg transition-[opacity,visibility] duration-150 group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100";
  content.setAttribute("role", "tooltip");
  content.textContent = text;
  const arrow = document.createElement("span");
  arrow.className = "absolute left-1/2 top-full h-0 w-0 -translate-x-1/2 border-x-[5px] border-t-[5px] border-x-transparent border-t-gray-800";
  content.appendChild(arrow);
  wrapper.appendChild(icon);
  wrapper.appendChild(content);
  return wrapper;
}

function addTagChip(container: HTMLElement, value: string) {
  const v = String(value || "").trim();
  if (!v) return;
  const existing = Array.from(container.querySelectorAll("[data-tag-value]"))
    .map((el) => el.getAttribute("data-tag-value"));
  if (existing.includes(v)) return;
  const chip = document.createElement("span");
  chip.className = "inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs bg-surface-2 border border-border text-body";
  chip.setAttribute("data-tag-value", v);
  const label = document.createElement("span");
  label.textContent = v;
  const removeBtn = document.createElement("button");
  removeBtn.type = "button";
  removeBtn.className = "text-body-soft hover:text-danger leading-none cursor-pointer bg-transparent border-0 p-0";
  removeBtn.setAttribute("aria-label", `Remove ${v}`);
  removeBtn.textContent = "×";
  removeBtn.addEventListener("click", () => chip.remove());
  chip.appendChild(label);
  chip.appendChild(removeBtn);
  container.appendChild(chip);
}

function generateValue(type) {
  if (type === "random_hex_32") {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    return Array.from(bytes).map((b) => b.toString(16).padStart(2, "0")).join("");
  }
  if (type === "random_hex_16") {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    return Array.from(bytes).map((b) => b.toString(16).padStart(2, "0")).join("");
  }
  return "";
}

function renderSecretFields(container, kind, secret = {}) {
  const schema = SECRET_SCHEMAS[kind] || SECRET_SCHEMAS.custom;
  const payload = toSecretRecord(secret);
  container.innerHTML = "";

  schema.forEach((field) => {
    const row = document.createElement("label");
    row.className = field.fullWidth ? "pipeline-editor-field is-full-width" : "pipeline-editor-field";

    const labelRow = document.createElement("span");
    labelRow.className = "credential-field-label-row";
    const label = document.createElement("span");
    label.textContent = field.label;
    labelRow.appendChild(label);
    if (field.help) labelRow.appendChild(createHelpTooltip(field.help));
    row.appendChild(labelRow);

    let input;
    if (field.type === "tags") {
      // Array tags input — stores a JSON array in the secret
      const wrap = document.createElement("div");
      wrap.className = "flex flex-col gap-1.5";
      wrap.setAttribute("data-secret-key", field.key);
      wrap.setAttribute("data-tags-input", "true");

      const tagsContainer = document.createElement("div");
      tagsContainer.className = "flex flex-wrap gap-1 min-h-6";

      const existing = Array.isArray(payload[field.key]) ? payload[field.key] : [];
      existing.forEach((tag) => addTagChip(tagsContainer, String(tag)));

      const addRow = document.createElement("div");
      addRow.className = "flex gap-1.5 items-stretch";
      const textInput = document.createElement("input");
      textInput.type = "text";
      textInput.placeholder = field.placeholder || "Add role...";
      textInput.className = "flex-1 min-w-0";
      const addBtn = document.createElement("button");
      addBtn.type = "button";
      addBtn.className = "credential-gen-btn";
      addBtn.textContent = "+ Add";
      addBtn.addEventListener("click", () => {
        addTagChip(tagsContainer, textInput.value);
        textInput.value = "";
        textInput.focus();
      });
      textInput.addEventListener("keydown", (e: KeyboardEvent) => {
        if (e.key === "Enter") { e.preventDefault(); addBtn.click(); }
      });

      addRow.appendChild(textInput);
      addRow.appendChild(addBtn);
      wrap.appendChild(tagsContainer);
      wrap.appendChild(addRow);

      row.appendChild(wrap);
      container.appendChild(row);
      return; // skip the rest (no data-secret-key on a plain input)
    } else if (field.type === "select") {
      input = document.createElement("select");
      const currentVal = typeof payload[field.key] === "string" ? payload[field.key] : (field.default || "");
      (field.options || []).forEach((opt) => {
        const option = document.createElement("option");
        option.value = opt.value;
        option.textContent = opt.label;
        if (opt.value === currentVal) option.selected = true;
        input.appendChild(option);
      });
    } else if (field.type === "textarea") {
      input = document.createElement("textarea");
      input.rows = Number(field.rows || 6);
      input.value = typeof payload[field.key] === "string" ? payload[field.key] : "";
    } else {
      input = document.createElement("input");
      input.type = field.type || "text";
      input.value = typeof payload[field.key] === "string" ? payload[field.key] : "";
    }
    input.setAttribute("data-secret-key", field.key);
    if (field.placeholder && field.type !== "select") {
      input.placeholder = field.placeholder;
    }

    if (field.generate) {
      const wrap = document.createElement("div");
      wrap.className = "credential-gen-wrap";
      wrap.appendChild(input);
      const genBtn = document.createElement("button");
      genBtn.type = "button";
      genBtn.className = "credential-gen-btn";
      genBtn.textContent = "Generate";
      genBtn.title = "Generate a secure random value";
      genBtn.addEventListener("click", () => {
        input.value = generateValue(field.generate);
        input.type = "text";
        genBtn.textContent = "Regenerate";
      });
      wrap.appendChild(genBtn);
      row.appendChild(wrap);
    } else {
      row.appendChild(input);
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
    if (!key) return;

    // Tags (array) field
    if (field.getAttribute("data-tags-input") === "true") {
      const chips = field.querySelectorAll("[data-tag-value]");
      const values = Array.from(chips)
        .map((c) => String(c.getAttribute("data-tag-value") || "").trim())
        .filter((v) => v);
      if (values.length > 0) {
        out[key] = values;
      }
      return;
    }

    const value = String((field as any).value || "").trim();
    if (value) {
      out[key] = value;
    }
  });
  return out;
}

async function initCredentials(root) {
  const runtimeNode = root.querySelector("#project-credentials-runtime");
  let runtimeConfig = null;
  try {
    runtimeConfig = runtimeNode ? JSON.parse(runtimeNode.textContent || "{}") : null;
  } catch (_err) {
    runtimeConfig = null;
  }
  const apiList = String(runtimeConfig?.api?.list || "");
  const apiItemBase = String(runtimeConfig?.api?.item_base || "");

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
      td.className = studioTableTdClass;
      td.textContent = "No credentials yet";
      tr.appendChild(td);
      state.rows.appendChild(tr);
      return;
    }

    state.items.forEach((item) => {
      const updatedAtStr = formatTs(item.updated_at);
      const tr = document.createElement("tr");
      tr.innerHTML = `
        <td>${item.credential_id || ""}</td>
        <td>${item.title || ""}</td>
        <td>${item.kind || ""}</td>
        <td>${item.has_secret ? "yes" : "no"}</td>
        <td>${updatedAtStr}</td>
        <td><button type="button" class="project-inline-chip" data-edit-id="${item.credential_id || ""}">Edit</button></td>
      `;
      tr.querySelectorAll("td").forEach((cell) => {
        cell.className = studioTableTdClass;
      });
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

const initializedRoots = new WeakSet();
let scanScheduled = false;

function scanCredentialRoots() {
  document.querySelectorAll("[data-project-credentials='true']").forEach((root) => {
    if (initializedRoots.has(root)) {
      return;
    }
    initializedRoots.add(root);
    initCredentials(root).catch((err) => {
      console.error("credentials ui failed", err);
    });
  });
}

export function initProjectCredentialsBehavior() {
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
    scanCredentialRoots();
  };
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(run);
  } else {
    setTimeout(run, 0);
  }
}
