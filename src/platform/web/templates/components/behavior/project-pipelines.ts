export function initPipelineRegistryBehavior() {
  if (typeof document === "undefined") return;
  requestAnimationFrame(() => {
    const root = document.querySelector<HTMLElement>("[data-pipeline-registry]");
    if (!root) return;

    const owner = root.dataset.owner ?? "";
    const project = root.dataset.project ?? "";
    const apiDelete = root.dataset.apiDelete ?? "";
    const apiGitStatus = root.dataset.apiGitStatus ?? "";
    const apiGitCommit = root.dataset.apiGitCommit ?? "";

    // ── New Pipeline inline form ──────────────────────────────────────────────
    const newPipelineToggle = root.querySelector<HTMLButtonElement>("[data-new-pipeline-toggle]");
    const newPipelineForm = root.querySelector<HTMLElement>("[data-new-pipeline-form]");
    const newPipelineSubmit = root.querySelector<HTMLButtonElement>("[data-new-pipeline-submit]");
    const newPipelineCancel = root.querySelector<HTMLButtonElement>("[data-new-pipeline-cancel]");

    newPipelineToggle?.addEventListener("click", () => {
      if (newPipelineForm) newPipelineForm.hidden = false;
      if (newPipelineToggle) newPipelineToggle.hidden = true;
    });
    newPipelineCancel?.addEventListener("click", () => {
      if (newPipelineForm) newPipelineForm.hidden = true;
      if (newPipelineToggle) newPipelineToggle.hidden = false;
      clearForm(newPipelineForm);
    });
    newPipelineSubmit?.addEventListener("click", () => {
      const name = (newPipelineForm?.querySelector<HTMLInputElement>("[name=name]")?.value ?? "").trim();
      const title = (newPipelineForm?.querySelector<HTMLInputElement>("[name=title]")?.value ?? "").trim();
      const triggerKind = (newPipelineForm?.querySelector<HTMLSelectElement>("[name=trigger_kind]")?.value ?? "webhook");
      if (!name) return;
      createPipelineAndNavigate(getCurrentVirtualPath(), name, title, triggerKind);
    });

    // ── New Folder inline form ────────────────────────────────────────────────
    const newFolderToggle = root.querySelector<HTMLButtonElement>("[data-new-folder-toggle]");
    const newFolderForm = root.querySelector<HTMLElement>("[data-new-folder-form]");
    const newFolderSubmit = root.querySelector<HTMLButtonElement>("[data-new-folder-submit]");
    const newFolderCancel = root.querySelector<HTMLButtonElement>("[data-new-folder-cancel]");

    newFolderToggle?.addEventListener("click", () => {
      if (newFolderForm) newFolderForm.hidden = false;
      if (newFolderToggle) newFolderToggle.hidden = true;
    });
    newFolderCancel?.addEventListener("click", () => {
      if (newFolderForm) newFolderForm.hidden = true;
      if (newFolderToggle) newFolderToggle.hidden = false;
      clearForm(newFolderForm);
    });
    newFolderSubmit?.addEventListener("click", () => {
      const folderName = (newFolderForm?.querySelector<HTMLInputElement>("[name=folder_name]")?.value ?? "").trim();
      if (!folderName) return;
      const currentPath = getCurrentVirtualPath();
      const newPath = currentPath === "/" ? `/${folderName}` : `${currentPath}/${folderName}`;
      window.location.href = `/projects/${owner}/${project}/pipelines/registry?path=${encodeURIComponent(newPath)}`;
    });

    // ── Delete dialog ─────────────────────────────────────────────────────────
    const deleteDialog = root.querySelector<HTMLElement>("[data-delete-pipeline-dialog]");
    const deleteNameDisplay = root.querySelector<HTMLElement>("[data-delete-pipeline-name]");
    const deleteConfirmInput = root.querySelector<HTMLInputElement>("[data-delete-confirm-input]");
    const deleteConfirmBtn = root.querySelector<HTMLButtonElement>("[data-delete-confirm-btn]");
    const deleteCancelBtns = root.querySelectorAll<HTMLElement>("[data-delete-cancel-btn]");

    let pendingDeletePath = "";
    let pendingDeleteName = "";

    root.addEventListener("click", (e) => {
      const btn = (e.target as Element).closest<HTMLButtonElement>("[data-delete-pipeline]");
      if (!btn) return;
      pendingDeletePath = btn.dataset.relPath ?? "";
      pendingDeleteName = btn.dataset.pipelineName ?? "";
      openDeleteDialog(pendingDeleteName);
    });

    function openDeleteDialog(name: string) {
      if (!deleteDialog) return;
      deleteDialog.hidden = false;
      if (deleteNameDisplay) deleteNameDisplay.textContent = name;
      if (deleteConfirmInput) { deleteConfirmInput.value = ""; deleteConfirmInput.focus(); }
      if (deleteConfirmBtn) deleteConfirmBtn.disabled = true;
    }

    function closeDeleteDialog() {
      if (deleteDialog) deleteDialog.hidden = true;
      pendingDeletePath = "";
      pendingDeleteName = "";
      if (deleteConfirmInput) deleteConfirmInput.value = "";
      if (deleteConfirmBtn) deleteConfirmBtn.disabled = true;
    }

    deleteConfirmInput?.addEventListener("input", () => {
      if (deleteConfirmBtn) {
        deleteConfirmBtn.disabled = deleteConfirmInput.value.trim() !== pendingDeleteName;
      }
    });

    deleteCancelBtns.forEach((el) => el.addEventListener("click", closeDeleteDialog));

    deleteConfirmBtn?.addEventListener("click", async () => {
      if (!pendingDeletePath || !apiDelete) return;
      deleteConfirmBtn.disabled = true;
      deleteConfirmBtn.textContent = "Deleting…";
      try {
        const resp = await fetch(apiDelete, {
          method: "DELETE",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ file_rel_path: pendingDeletePath }),
        });
        if (!resp.ok) {
          const data = await resp.json().catch(() => ({}));
          alert(`Delete failed: ${(data as any).error ?? resp.status}`);
          deleteConfirmBtn.disabled = false;
          deleteConfirmBtn.textContent = "Delete";
          return;
        }
        const row = root.querySelector<HTMLElement>(`[data-pipeline-row][data-rel-path="${CSS.escape(pendingDeletePath)}"]`);
        row?.remove();
        const deletedPath = pendingDeletePath;
        closeDeleteDialog();
        openGitCommitDialog([{ rel_path: deletedPath, code: "D" }]);
      } catch {
        alert("Network error during delete");
        deleteConfirmBtn.disabled = false;
        deleteConfirmBtn.textContent = "Delete";
      }
    });

    // ── Commit toolbar button ─────────────────────────────────────────────────
    const commitBtn = root.querySelector<HTMLButtonElement>("[data-registry-commit]");
    commitBtn?.addEventListener("click", async () => {
      if (!apiGitStatus) return;
      if (commitBtn) commitBtn.disabled = true;
      try {
        const resp = await fetch(apiGitStatus);
        if (!resp.ok) { if (commitBtn) commitBtn.disabled = false; return; }
        const items = (await resp.json()) as Array<{ rel_path: string; code: string }>;
        openGitCommitDialog(items);
      } catch { /* ignore */ }
      if (commitBtn) commitBtn.disabled = false;
    });

    // ── Git commit dialog (pre-rendered HTML, populated here) ─────────────────
    const commitDialog = root.querySelector<HTMLElement>("[data-git-commit-dialog]");
    const commitFileList = root.querySelector<HTMLElement>("[data-git-commit-file-list]");
    const commitMessage = root.querySelector<HTMLTextAreaElement>("[data-git-commit-message]");
    const commitPush = root.querySelector<HTMLInputElement>("[data-git-commit-push]");
    const commitError = root.querySelector<HTMLElement>("[data-git-commit-error]");
    const commitSubmit = root.querySelector<HTMLButtonElement>("[data-git-commit-submit]");
    const commitCloseBtns = root.querySelectorAll<HTMLElement>("[data-git-commit-close]");

    function openGitCommitDialog(files: Array<{ rel_path: string; code: string }>) {
      if (!commitDialog || !commitFileList) return;

      // Populate file checkboxes
      commitFileList.innerHTML = "";
      for (const f of files) {
        const label = document.createElement("label");
        label.className = "git-commit-file-row";

        const cb = document.createElement("input");
        cb.type = "checkbox";
        cb.checked = true;
        cb.value = f.rel_path;
        cb.name = "commit-file";
        cb.className = "git-commit-file-cb";

        const code = document.createElement("code");
        const codeClass = f.code === "??" ? "git-status-untracked" : `git-status-${f.code.trim()}`;
        code.className = `git-status-code ${codeClass}`;
        code.textContent = f.code === "??" ? "??" : f.code;

        const path = document.createElement("span");
        path.className = "git-commit-file-path";
        path.textContent = f.rel_path;

        label.append(cb, code, path);
        commitFileList.appendChild(label);
      }

      if (commitMessage) commitMessage.value = "";
      if (commitPush) commitPush.checked = false;
      if (commitError) { commitError.textContent = ""; commitError.hidden = true; }
      updateCommitSubmitState();
      commitDialog.hidden = false;
      commitMessage?.focus();
    }

    function closeCommitDialog() {
      if (commitDialog) commitDialog.hidden = true;
    }

    function updateCommitSubmitState() {
      if (!commitSubmit) return;
      const hasFiles = !!commitFileList?.querySelector<HTMLInputElement>("input[type=checkbox]:checked");
      const hasMsg = (commitMessage?.value.trim().length ?? 0) > 0;
      commitSubmit.disabled = !hasFiles || !hasMsg;
    }

    commitMessage?.addEventListener("input", updateCommitSubmitState);
    commitFileList?.addEventListener("change", updateCommitSubmitState);

    commitCloseBtns.forEach((el) => el.addEventListener("click", closeCommitDialog));
    commitDialog?.addEventListener("click", (e) => {
      if ((e.target as Element).classList.contains("git-commit-backdrop")) closeCommitDialog();
    });

    commitSubmit?.addEventListener("click", async () => {
      if (!commitFileList || !commitMessage || !commitPush || !commitSubmit) return;
      const files = Array.from(commitFileList.querySelectorAll<HTMLInputElement>("input[type=checkbox]:checked"))
        .map((cb) => cb.value);
      const message = commitMessage.value.trim();
      const push = commitPush.checked;
      if (!files.length || !message || !apiGitCommit) return;

      commitSubmit.disabled = true;
      commitSubmit.textContent = "Committing…";
      if (commitError) { commitError.textContent = ""; commitError.hidden = true; }

      try {
        const resp = await fetch(apiGitCommit, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ files, message, push }),
        });
        const data = await resp.json().catch(() => ({}));
        if (!resp.ok) {
          if (commitError) { commitError.textContent = (data as any).error ?? "Commit failed"; commitError.hidden = false; }
          commitSubmit.disabled = false;
          commitSubmit.textContent = "Commit";
          return;
        }
        closeCommitDialog();
        window.dispatchEvent(new CustomEvent("zf:repo:changed"));
        window.location.reload();
      } catch (err: unknown) {
        if (commitError) { commitError.textContent = (err as Error)?.message ?? "Network error"; commitError.hidden = false; }
        commitSubmit.disabled = false;
        commitSubmit.textContent = "Commit";
      }
    });

    // ── Helpers ───────────────────────────────────────────────────────────────
    function getCurrentVirtualPath(): string {
      return new URLSearchParams(window.location.search).get("path") ?? "/";
    }

    function clearForm(form: HTMLElement | null) {
      form?.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("input, textarea").forEach((el) => { el.value = ""; });
    }

    async function createPipelineAndNavigate(virtualPath: string, name: string, title: string, triggerKind: string) {
      const defaultSource = JSON.stringify({
        kind: "zebflow.pipeline", version: "0.1", id: name,
        metadata: { virtual_path: virtualPath },
        entry_nodes: [], nodes: [], edges: [],
      }, null, 2);
      const resp = await fetch(`/api/projects/${owner}/${project}/pipelines/definition`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ virtual_path: virtualPath, name, title, description: "", trigger_kind: triggerKind, source: defaultSource }),
      });
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        alert(`Failed to create pipeline: ${(data as any).error ?? resp.status}`);
        return;
      }
      const data = await resp.json() as any;
      const fileId: string = data?.meta?.file_rel_path ?? "";
      window.location.href = `/projects/${owner}/${project}/pipelines/editor?path=${encodeURIComponent(virtualPath)}&id=${encodeURIComponent(fileId)}`;
    }
  });
}
