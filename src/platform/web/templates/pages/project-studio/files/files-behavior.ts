export function initFilesBehavior() {
  if (typeof document === "undefined") return;
  requestAnimationFrame(() => {
    const root = document.querySelector<HTMLElement>("[data-files-browser]");
    if (!root) return;

    const apiMkdir = root.dataset.apiMkdir ?? "";
    const apiUpload = root.dataset.apiUpload ?? "";
    const apiRm    = root.dataset.apiRm    ?? "";

    // Current path comes from the URL ?path= param
    function getCurrentPath(): string {
      return new URLSearchParams(window.location.search).get("path") ?? "";
    }

    function navigateTo(path: string) {
      const base = window.location.pathname;
      window.location.href = path ? `${base}?path=${encodeURIComponent(path)}` : base;
    }

    // ── New folder form ───────────────────────────────────────────────────────
    const newFolderToggle = root.querySelector<HTMLButtonElement>("[data-new-folder-toggle]");
    const newFolderForm   = root.querySelector<HTMLElement>("[data-new-folder-form]");
    const newFolderInput  = root.querySelector<HTMLInputElement>("[data-new-folder-input]");
    const newFolderSubmit = root.querySelector<HTMLButtonElement>("[data-new-folder-submit]");
    const newFolderCancel = root.querySelector<HTMLButtonElement>("[data-new-folder-cancel]");
    const fileUploadTrigger = root.querySelector<HTMLButtonElement>("[data-file-upload-trigger]");
    const fileUploadInput = root.querySelector<HTMLInputElement>("[data-file-upload-input]");

    newFolderToggle?.addEventListener("click", () => {
      if (newFolderForm) newFolderForm.hidden = false;
      if (newFolderToggle) newFolderToggle.hidden = true;
      newFolderInput?.focus();
    });

    newFolderCancel?.addEventListener("click", () => {
      if (newFolderForm) newFolderForm.hidden = true;
      if (newFolderToggle) newFolderToggle.hidden = false;
      if (newFolderInput) newFolderInput.value = "";
    });

    newFolderSubmit?.addEventListener("click", async () => {
      const name = (newFolderInput?.value ?? "").trim();
      if (!name) return;
      const currentPath = getCurrentPath();
      const fullPath = currentPath ? `${currentPath}/${name}` : name;
      const resp = await fetch(apiMkdir, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: fullPath }),
      });
      if (resp.ok) {
        window.location.reload();
      } else {
        const err = await resp.json().catch(() => ({ error: "unknown" }));
        alert(`Failed: ${err.error ?? "unknown"}`);
      }
    });

    fileUploadTrigger?.addEventListener("click", () => {
      fileUploadInput?.click();
    });

    fileUploadInput?.addEventListener("change", async () => {
      const file = fileUploadInput.files?.[0];
      if (!file || !apiUpload) return;
      const currentPath = getCurrentPath() || "uploads";
      const form = new FormData();
      form.append("file", file);
      const resp = await fetch(`${apiUpload}?path=${encodeURIComponent(currentPath)}`, {
        method: "POST",
        body: form,
      });
      if (resp.ok) {
        window.location.reload();
      } else {
        const err = await resp.json().catch(() => ({ error: "unknown" }));
        alert(`Failed: ${err.error ?? "unknown"}`);
      }
      fileUploadInput.value = "";
    });

    // ── Click delegation ──────────────────────────────────────────────────────
    root.addEventListener("click", async (e) => {
      const target = e.target as HTMLElement;

      // Delete — check first (inside folder/file rows)
      const deleteBtn = target.closest<HTMLElement>("[data-delete-btn]");
      if (deleteBtn) {
        e.preventDefault();
        e.stopPropagation();
        const path = deleteBtn.dataset.deletePath ?? "";
        if (!path) return;
        const name = path.split("/").pop() ?? path;
        if (!confirm(`Delete "${name}"? This cannot be undone.`)) return;
        const resp = await fetch(apiRm, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path }),
        });
        if (resp.ok) {
          window.location.reload();
        } else {
          const err = await resp.json().catch(() => ({ error: "unknown" }));
          alert(`Failed: ${err.error ?? "unknown"}`);
        }
        return;
      }

      // Folder row → navigate into it
      const folderRow = target.closest<HTMLElement>("[data-folder-path]");
      if (folderRow) {
        e.preventDefault();
        navigateTo(folderRow.dataset.folderPath ?? "");
        return;
      }

      // Breadcrumb → navigate
      const crumb = target.closest<HTMLElement>("[data-crumb-path]");
      if (crumb) {
        e.preventDefault();
        navigateTo(crumb.dataset.crumbPath ?? "");
      }
    });
  });
}
