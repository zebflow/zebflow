let editorViewCtor = null;
let basicSetupExt = null;
let markdownExt = null;
let oneDarkExt = null;
let createSplitPaneFn = null;
let runtimePromise = null;

async function ensureDocsEditorRuntime() {
  if (editorViewCtor && basicSetupExt && oneDarkExt && createSplitPaneFn) {
    return;
  }
  if (runtimePromise) {
    return runtimePromise;
  }
  runtimePromise = (async () => {
    const base = window.location.origin;
    const codeMirrorUrl = new URL(
      "/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
      base
    );
    const interactUrl = new URL(
      "/assets/libraries/zeb/interact/0.1/runtime/interact.bundle.mjs",
      base
    );
    const codeMirrorRuntime = await import(codeMirrorUrl.href);
    const interactRuntime = await import(interactUrl.href);
    editorViewCtor = codeMirrorRuntime.EditorView;
    basicSetupExt = codeMirrorRuntime.basicSetup;
    markdownExt = codeMirrorRuntime.markdown;
    oneDarkExt = codeMirrorRuntime.oneDark;
    createSplitPaneFn = interactRuntime.createSplitPane;
  })();
  return runtimePromise;
}

let editorInstance = null;

function createEditorView(host, content) {
  if (!editorViewCtor || !basicSetupExt || !oneDarkExt) return null;
  const exts = [basicSetupExt, oneDarkExt];
  if (markdownExt) {
    const mdExt = typeof markdownExt === "function" ? markdownExt() : markdownExt;
    exts.push(mdExt);
  }
  return new editorViewCtor({
    doc: content,
    extensions: exts,
    parent: host,
  });
}

function getEditorContent() {
  if (editorInstance) {
    return editorInstance.state.doc.toString();
  }
  return null;
}

function setEditorContent(content) {
  if (editorInstance) {
    editorInstance.dispatch({
      changes: { from: 0, to: editorInstance.state.doc.length, insert: content },
    });
  }
}

function showToast(message, kind = "info") {
  const sonner = document.querySelector("[data-sonner-toaster]");
  if (!sonner) {
    console.log("[docs] " + kind + ": " + message);
    return;
  }
  const toast = document.createElement("li");
  toast.className = "sonner-toast sonner-toast-" + kind;
  toast.textContent = message;
  sonner.appendChild(toast);
  setTimeout(function() { toast.remove(); }, 3500);
}

let docsInitScheduled = false;

function runDocsBehavior() {
  docsInitScheduled = false;
  const root = document.querySelector("[data-docs-workspace]");
  if (!root) return;

  const apiList = root.getAttribute("data-docs-api-list") || "";
  const apiRead = root.getAttribute("data-docs-api-read") || "";
  const apiCreate = root.getAttribute("data-docs-api-create") || "";
  const apiAgentRead = root.getAttribute("data-docs-api-agent-read") || "";
  const apiAgentSave = root.getAttribute("data-docs-api-agent-save") || "";

  const editorHost = root.querySelector("[data-docs-editor-host]");
  const editorSource = root.querySelector("[data-docs-editor-source]");
  const fileLabel = root.querySelector("[data-docs-current-file-label]");
  const fileValue = root.querySelector("[data-docs-current-file-value]");
  const saveState = root.querySelector("[data-docs-save-state]");
  const saveBtn = root.querySelector("[data-docs-save]");
  const deleteBtn = root.querySelector("[data-docs-delete]");
  const newBtn = root.querySelector("[data-docs-new]");
  const fileList = root.querySelector("[data-docs-file-list]");
  const agentList = root.querySelector("[data-docs-agent-list]");

  let currentTab = "user";
  let currentPath = root.getAttribute("data-docs-selected-path") || "";
  let currentAgentFile = "";

  function markClean() {
    if (saveState) saveState.textContent = "Clean";
  }

  function markDirty() {
    if (saveState) saveState.textContent = "Modified";
  }

  function setCurrentFileLabel(label) {
    if (fileLabel) fileLabel.textContent = label || "Select a file";
    if (fileValue) fileValue.textContent = label || "(none)";
  }

  function showActions(canSave, canDelete) {
    if (saveBtn) saveBtn.style.display = canSave ? "" : "none";
    if (deleteBtn) deleteBtn.style.display = canDelete ? "" : "none";
  }

  async function loadEditorForContent(content, label, canSave, canDelete) {
    await ensureDocsEditorRuntime();
    setCurrentFileLabel(label);
    showActions(canSave, canDelete);

    if (!editorInstance && editorHost) {
      editorInstance = createEditorView(editorHost, content);
      if (editorSource) editorSource.style.display = "none";
    } else if (editorInstance) {
      setEditorContent(content);
    }
    markClean();
  }

  // Tab switching
  const tabBtns = root.querySelectorAll("[data-docs-tab]");
  const panels = root.querySelectorAll("[data-docs-panel]");
  for (const btn of tabBtns) {
    btn.addEventListener("click", function() {
      const tab = btn.getAttribute("data-docs-tab");
      currentTab = tab;
      for (const b of tabBtns) {
        b.classList.toggle("is-active", b.getAttribute("data-docs-tab") === tab);
      }
      for (const p of panels) {
        p.classList.toggle("is-active", p.getAttribute("data-docs-panel") === tab);
      }
      currentPath = "";
      currentAgentFile = "";
      setCurrentFileLabel("");
      showActions(false, false);
      if (editorInstance) setEditorContent("");
    });
  }

  // User docs: file click
  function bindUserDocLinks(container) {
    const links = container.querySelectorAll("[data-docs-file]");
    for (const link of links) {
      link.addEventListener("click", async function(e) {
        e.preventDefault();
        const path = link.getAttribute("data-docs-file");
        if (!path) return;
        currentPath = path;
        currentAgentFile = "";
        for (const l of container.querySelectorAll("[data-docs-file]")) {
          l.classList.toggle("is-active", l.getAttribute("data-docs-file") === path);
        }
        try {
          const res = await fetch(apiRead + "?path=" + encodeURIComponent(path));
          const data = await res.json();
          const content = (data && data.doc && data.doc.content) ? data.doc.content : "";
          await loadEditorForContent(content, path, true, true);
        } catch (err) {
          showToast("Failed to load " + path, "error");
        }
      });
    }
  }

  if (fileList) bindUserDocLinks(fileList);

  // Agent docs: file click
  function bindAgentDocLinks(container) {
    const links = container.querySelectorAll("[data-docs-agent-file]");
    for (const link of links) {
      link.addEventListener("click", async function(e) {
        e.preventDefault();
        const name = link.getAttribute("data-docs-agent-file");
        if (!name) return;
        currentAgentFile = name;
        currentPath = "";
        for (const l of container.querySelectorAll("[data-docs-agent-file]")) {
          l.classList.toggle("is-active", l.getAttribute("data-docs-agent-file") === name);
        }
        try {
          const res = await fetch(apiAgentRead + "?path=" + encodeURIComponent(name));
          const data = await res.json();
          const content = (data && data.doc && data.doc.content) ? data.doc.content : "";
          const isReadonly = name === "MEMORY.md";
          await loadEditorForContent(content, name, !isReadonly, false);
        } catch (err) {
          showToast("Failed to load " + name, "error");
        }
      });
    }
  }

  if (agentList) bindAgentDocLinks(agentList);

  // Save handler
  if (saveBtn) {
    saveBtn.addEventListener("click", async function() {
      const content = getEditorContent();
      if (content === null) return;
      try {
        if (currentTab === "agent" && currentAgentFile) {
          const res = await fetch(apiAgentSave + "?path=" + encodeURIComponent(currentAgentFile), {
            method: "PUT",
            body: content,
          });
          if (!res.ok) throw new Error("Save failed");
          showToast("Saved " + currentAgentFile, "success");
          markClean();
        } else if (currentPath) {
          const res = await fetch(apiRead + "?path=" + encodeURIComponent(currentPath), {
            method: "PUT",
            headers: { "Content-Type": "text/plain" },
            body: content,
          });
          if (!res.ok) throw new Error("Save failed");
          showToast("Saved " + currentPath, "success");
          markClean();
        }
      } catch (err) {
        showToast("Save failed", "error");
      }
    });
  }

  // Delete handler (user docs only)
  if (deleteBtn) {
    deleteBtn.addEventListener("click", async function() {
      if (!currentPath) return;
      if (!confirm("Delete " + currentPath + "?")) return;
      try {
        const res = await fetch(apiRead + "?path=" + encodeURIComponent(currentPath), {
          method: "DELETE",
        });
        if (!res.ok) throw new Error("Delete failed");
        showToast("Deleted " + currentPath, "success");
        currentPath = "";
        setCurrentFileLabel("");
        showActions(false, false);
        if (editorInstance) setEditorContent("");
        await refreshUserDocList();
      } catch (err) {
        showToast("Delete failed", "error");
      }
    });
  }

  // New doc handler
  if (newBtn) {
    newBtn.addEventListener("click", async function() {
      const name = prompt("New doc filename (e.g. README.md):", "README.md");
      if (!name || !name.trim()) return;
      const path = name.trim().endsWith(".md") ? name.trim() : name.trim() + ".md";
      try {
        const res = await fetch(apiCreate, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path: path, content: "# " + path + "\n" }),
        });
        if (!res.ok) throw new Error("Create failed");
        showToast("Created " + path, "success");
        currentPath = path;
        await refreshUserDocList();
        await loadEditorForContent("# " + path + "\n", path, true, true);
      } catch (err) {
        showToast("Create failed", "error");
      }
    });
  }

  async function refreshUserDocList() {
    if (!fileList || !apiList) return;
    try {
      const res = await fetch(apiList);
      const data = await res.json();
      const items = Array.isArray(data && data.items) ? data.items : [];
      fileList.innerHTML = items.map(function(item) {
        const active = item.path === currentPath ? " is-active" : "";
        return '<a class="docs-file-item' + active + '" data-docs-file="' + item.path + '" href="#">'
          + '<svg viewBox="0 0 24 24" fill="none" class="w-3 h-3 shrink-0"><path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>'
          + '<span>' + item.name + '</span>'
          + '</a>';
      }).join("");
      bindUserDocLinks(fileList);
    } catch (err) {
      // silently ignore
    }
  }

  // Init split pane and editor
  ensureDocsEditorRuntime().then(function() {
    const sidebar = root.querySelector("[data-split-target]");
    const handle = root.querySelector("[data-docs-split-handle]");
    if (sidebar && handle && createSplitPaneFn) {
      createSplitPaneFn(root, sidebar, handle, { minSize: 160, defaultSize: 220 });
    }

    const initialContent = editorSource ? editorSource.textContent || "" : "";
    const initialPath = root.getAttribute("data-docs-selected-path") || "";
    if (editorHost && initialPath) {
      loadEditorForContent(initialContent, initialPath, true, true);
    }
  });
}

export function initDocsBehavior() {
  if (typeof document === "undefined") return;
  if (typeof Deno !== "undefined") return;
  if (docsInitScheduled) return;
  docsInitScheduled = true;
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(runDocsBehavior);
  } else {
    setTimeout(runDocsBehavior, 0);
  }
}
