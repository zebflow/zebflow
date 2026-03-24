/**
 * Behavior for the ⬇ Install catalog modal on the pipeline registry page.
 * Handles: open/close, fetching catalog, checkbox selection, POST install.
 */
export function initInstallCatalogBehavior() {
  if (typeof document === "undefined") return;

  requestAnimationFrame(() => {
    const root = document.querySelector<HTMLElement>("[data-pipeline-registry]");
    if (!root) return;

    const owner = root.dataset.owner ?? "";
    const project = root.dataset.project ?? "";
    const catalogApiUrl = `/api/projects/${owner}/${project}/install/catalog/ui`;
    const installApiUrl = `/api/projects/${owner}/${project}/install/ui`;

    const dialog = root.querySelector<HTMLElement>("[data-install-catalog-dialog]");
    if (!dialog) return;

    const openBtn = root.querySelector<HTMLButtonElement>("[data-install-catalog-open]");
    const closeBtns = root.querySelectorAll<HTMLElement>("[data-install-catalog-close]");
    const componentList = dialog.querySelector<HTMLElement>("[data-install-component-list]");
    const submitBtn = dialog.querySelector<HTMLButtonElement>("[data-install-submit]");
    const resultEl = dialog.querySelector<HTMLElement>("[data-install-result]");
    const selectAllBtn = dialog.querySelector<HTMLButtonElement>("[data-install-select-all]");
    const selectNoneBtn = dialog.querySelector<HTMLButtonElement>("[data-install-select-none]");
    const selectEssentialsBtn = dialog.querySelector<HTMLButtonElement>("[data-install-select-essentials]");

    const ESSENTIALS = ["button", "input", "textarea", "label", "checkbox", "badge", "card", "dialog", "select", "tabs", "separator", "alert"];

    let catalogData: any[] = [];
    let loaded = false;

    // ── Tab switching ─────────────────────────────────────────────────────────
    const tabBtns = dialog.querySelectorAll<HTMLButtonElement>("[data-install-tab-btn]");
    const tabContents = dialog.querySelectorAll<HTMLElement>("[data-install-tab-content]");

    tabBtns.forEach(btn => {
      btn.addEventListener("click", () => {
        const target = btn.dataset.installTabBtn ?? "";
        tabBtns.forEach(b => {
          b.classList.toggle("is-active", b.dataset.installTabBtn === target);
          b.dataset.installTabActive = b.dataset.installTabBtn === target ? "true" : "";
        });
        tabContents.forEach(c => {
          (c as HTMLElement).hidden = c.dataset.installTabContent !== target;
        });
      });
    });

    // ── Open dialog ───────────────────────────────────────────────────────────
    openBtn?.addEventListener("click", () => {
      dialog.hidden = false;
      if (!loaded) loadCatalog();
    });

    // ── Close dialog ──────────────────────────────────────────────────────────
    closeBtns.forEach(btn => btn.addEventListener("click", () => { dialog.hidden = true; }));

    // ── Load catalog from API ─────────────────────────────────────────────────
    async function loadCatalog() {
      if (!componentList) return;
      componentList.innerHTML = '<div style="padding:16px;color:var(--zf-ui-text-muted);font-size:12px;">Loading…</div>';
      try {
        const res = await fetch(catalogApiUrl, { headers: { "Accept": "application/json" } });
        const json = await res.json();
        catalogData = json?.components ?? [];
        loaded = true;
        renderList();
      } catch {
        componentList.innerHTML = '<div style="padding:16px;color:var(--zf-ui-text-muted);font-size:12px;">Failed to load catalog.</div>';
      }
    }

    function renderList() {
      if (!componentList) return;
      if (catalogData.length === 0) {
        componentList.innerHTML = '<div style="padding:16px;color:var(--zf-ui-text-muted);font-size:12px;">No components found.</div>';
        return;
      }
      componentList.innerHTML = catalogData.map(comp => `
        <label style="display:flex;align-items:center;gap:6px;padding:5px 6px;border-radius:5px;cursor:pointer;font-size:12px;background:${comp.installed ? 'var(--zf-ui-bg-subtle)' : ''}" title="${comp.description}">
          <input type="checkbox" data-component-name="${comp.name}" ${comp.installed ? "" : ""} style="accent-color:var(--zf-color-brand-blue);" />
          <span style="flex:1;color:var(--zf-ui-text)">${comp.name}</span>
          ${comp.installed ? '<span style="font-size:10px;color:#22c55e;">✓</span>' : ''}
          <span style="font-size:10px;color:var(--zf-ui-text-muted);text-transform:capitalize;">${comp.category}</span>
        </label>
      `).join("");
    }

    // ── Select helpers ────────────────────────────────────────────────────────
    function getCheckboxes() {
      return Array.from(componentList?.querySelectorAll<HTMLInputElement>("input[type=checkbox]") ?? []);
    }

    selectAllBtn?.addEventListener("click", () => {
      getCheckboxes().forEach(cb => { cb.checked = true; });
    });

    selectNoneBtn?.addEventListener("click", () => {
      getCheckboxes().forEach(cb => { cb.checked = false; });
    });

    selectEssentialsBtn?.addEventListener("click", () => {
      getCheckboxes().forEach(cb => {
        const name = cb.dataset.componentName ?? "";
        cb.checked = ESSENTIALS.includes(name);
      });
    });

    // ── Install selected ──────────────────────────────────────────────────────
    submitBtn?.addEventListener("click", async () => {
      const names = getCheckboxes()
        .filter(cb => cb.checked)
        .map(cb => cb.dataset.componentName ?? "")
        .filter(Boolean);

      if (names.length === 0) {
        if (resultEl) { resultEl.hidden = false; resultEl.textContent = "Select at least one component."; }
        return;
      }

      submitBtn.disabled = true;
      submitBtn.textContent = "Installing…";
      if (resultEl) resultEl.hidden = true;

      try {
        const res = await fetch(installApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ names, overwrite: false }),
        });
        const json = await res.json();
        if (json?.ok) {
          const { installed = [], skipped = [] } = json.report ?? {};
          const parts: string[] = [];
          if (installed.length > 0) parts.push(`Installed: ${installed.join(", ")}`);
          if (skipped.length > 0) parts.push(`Skipped (already exist): ${skipped.join(", ")}`);
          if (resultEl) {
            resultEl.hidden = false;
            resultEl.textContent = parts.join(" · ") || "Done.";
          }
          if (installed.length > 0) {
            // Dialog close + sidebar refresh now handled by Preact state in the page component
            dialog.hidden = true;
            loaded = false;
            loadCatalog();
          } else {
            // Nothing new installed — just refresh the catalog list
            loaded = false;
            loadCatalog();
          }
        } else {
          if (resultEl) { resultEl.hidden = false; resultEl.textContent = `Error: ${json?.error ?? "unknown"}`; }
        }
      } catch (err) {
        if (resultEl) { resultEl.hidden = false; resultEl.textContent = "Network error."; }
      } finally {
        submitBtn.disabled = false;
        submitBtn.textContent = "Install Selected";
      }
    });
  });
}
