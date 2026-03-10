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
    const historyPairs = form.elements.namedItem("chat_history_pairs");
    if (steps) steps.value = Number(cfg.max_steps || 50);
    if (replans) replans.value = Number(cfg.max_replans || 2);
    if (historyPairs) historyPairs.value = Number(cfg.chat_history_pairs ?? 10);
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
        chat_history_pairs: Number(form.elements.namedItem("chat_history_pairs")?.value ?? 10),
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

function initNodeRegistry(root) {
  const searchInput = root.querySelector("[data-node-search]");
  const tabBtns = root.querySelectorAll("[data-node-tab-btn]");
  const panels = root.querySelectorAll("[data-node-tab-panel]");
  const items = root.querySelectorAll("[data-node-item]");
  const groups = root.querySelectorAll("[data-node-group]");
  const summary = root.querySelector("[data-node-summary]");
  const total = items.length;

  let searchQuery = "";

  // Tab switching
  tabBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      const target = btn.getAttribute("data-node-tab-btn");
      tabBtns.forEach((b) => b.classList.remove("node-tab-active"));
      btn.classList.add("node-tab-active");
      panels.forEach((panel) => {
        panel.hidden = panel.getAttribute("data-node-tab-panel") !== target;
      });
    });
  });

  // Search filtering (applies to installed panel)
  function updateSearch() {
    let visible = 0;
    items.forEach((item) => {
      const text = item.getAttribute("data-search-text") || "";
      const show = !searchQuery || text.includes(searchQuery);
      item.style.display = show ? "" : "none";
      if (show) visible++;
    });
    // Hide group containers when all their items are hidden
    groups.forEach((group) => {
      const groupItems = group.querySelectorAll("[data-node-item]");
      const anyVisible = Array.from(groupItems).some(
        (el) => (el as HTMLElement).style.display !== "none"
      );
      group.style.display = anyVisible ? "" : "none";
    });
    if (summary) {
      summary.textContent =
        visible === total
          ? `${total} nodes · ${total} built-in`
          : `${visible} of ${total} nodes · ${total} built-in`;
    }
  }

  if (searchInput) {
    searchInput.addEventListener("input", () => {
      searchQuery = (searchInput.value || "").toLowerCase().trim();
      updateSearch();
    });
  }
}

const initializedRoots = new WeakSet();
let scanScheduled = false;

function scanAssistantSettingsRoots() {
  document.querySelectorAll("[data-assistant-settings]").forEach((root) => {
    if (initializedRoots.has(root)) {
      return;
    }
    initializedRoots.add(root);
    initAssistantSettings(root).catch((err) => {
      console.error("assistant settings ui failed", err);
    });
  });
}

function scanNodeRegistryRoots() {
  document.querySelectorAll("[data-node-registry]").forEach((root) => {
    if (initializedRoots.has(root)) {
      return;
    }
    initializedRoots.add(root);
    initNodeRegistry(root);
  });
}

export function initProjectSettingsBehavior() {
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
    scanAssistantSettingsRoots();
    scanNodeRegistryRoots();
  };
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(run);
  } else {
    setTimeout(run, 0);
  }
}
