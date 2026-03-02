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

function setStatus(el, message, tone = "info") {
  if (!el) return;
  el.textContent = message;
  el.setAttribute("data-tone", tone);
}

async function initAssistantSettings(root) {
  const configApi = root.getAttribute("data-api-config") || "";
  const form = root.querySelector("[data-assistant-settings-form]");
  const high = root.querySelector("[data-assistant-high]");
  const general = root.querySelector("[data-assistant-general]");
  const status = root.querySelector("[data-assistant-status]");
  if (!configApi || !form || !high || !general || !status) return;

  async function loadConfig() {
    setStatus(status, "Loading...", "info");
    const payload = await requestJson(configApi);
    const cfg = payload?.config || {};
    high.value = String(cfg.llm_high_credential_id || "");
    general.value = String(cfg.llm_general_credential_id || "");
    const steps = form.elements.namedItem("max_steps");
    const replans = form.elements.namedItem("max_replans");
    const enabled = form.elements.namedItem("enabled");
    if (steps) steps.value = Number(cfg.max_steps || 50);
    if (replans) replans.value = Number(cfg.max_replans || 2);
    if (enabled) enabled.checked = !!cfg.enabled;
    setStatus(status, "Ready.", "info");
  }

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const saveButton = form.querySelector("[data-assistant-save]");
    if (saveButton) saveButton.disabled = true;
    setStatus(status, "Saving...", "info");
    try {
      const body = {
        llm_high_credential_id: String(high.value || "").trim() || null,
        llm_general_credential_id: String(general.value || "").trim() || null,
        max_steps: Number(form.elements.namedItem("max_steps")?.value || 50),
        max_replans: Number(form.elements.namedItem("max_replans")?.value || 2),
        enabled: !!form.elements.namedItem("enabled")?.checked,
      };
      await requestJson(configApi, {
        method: "PUT",
        body: JSON.stringify(body),
      });
      setStatus(status, "Saved.", "ok");
    } catch (err) {
      setStatus(status, `Failed: ${err?.message || String(err)}`, "error");
    } finally {
      if (saveButton) saveButton.disabled = false;
    }
  });

  try {
    await loadConfig();
  } catch (err) {
    setStatus(status, `Failed: ${err?.message || String(err)}`, "error");
  }
}

function boot() {
  document.querySelectorAll("[data-assistant-settings]").forEach((root) => {
    initAssistantSettings(root);
  });
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
