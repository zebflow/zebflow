import {
  EditorView,
  basicSetup,
  javascript,
  css,
  autocompletion,
  snippetCompletion,
  linter,
  lintGutter,
  setDiagnostics,
  oneDark,
} from "/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs";
import { createSplitPane } from "/assets/libraries/zeb/interact/0.1/runtime/interact.bundle.mjs";
import { debounce } from "/assets/libraries/zeb/stateutil/0.1/runtime/stateutil.bundle.mjs";

const LIBRARY_KEYWORDS_URL = "/assets/libraries/zeb/codemirror/0.1/keywords.json";
const ZEBFLOW_KEYWORDS = [
  { label: "Page", type: "type" },
  { label: "zFor", type: "keyword" },
  { label: "zShow", type: "keyword" },
  { label: "zHide", type: "keyword" },
  { label: "zText", type: "keyword" },
  { label: "zModel", type: "keyword" },
  snippetCompletion(
    "export const page = {\n  head: {\n    title: \"$1\",\n    description: \"$2\",\n  },\n  html: {\n    lang: \"en\",\n  },\n  body: {\n    className: \"$3\",\n  },\n  navigation: \"history\",\n};\n\nexport const app = {};\n\nexport default function Page(input) {\n  return (\n    <Page>\n      $0\n    </Page>\n  );\n}\n",
    { label: "page scaffold", type: "snippet" },
  ),
  snippetCompletion(
    "export default function ${Component}(props) {\n  return (\n    <div>$0</div>\n  );\n}\n",
    { label: "component scaffold", type: "snippet" },
  ),
];

function iconForItem(item) {
  if (item.kind === "folder") {
    return '<svg viewBox="0 0 24 24" fill="none" class="w-4 h-4"><path d="M4 7h6l2 2h8v8H4z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>';
  }
  if (item.file_kind === "page") {
    return '<svg viewBox="0 0 24 24" fill="none" class="w-4 h-4"><path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/><path d="M14 4v4h4" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>';
  }
  if (item.file_kind === "component") {
    return '<svg viewBox="0 0 24 24" fill="none" class="w-4 h-4"><path d="M8 8h8v8H8z" stroke="currentColor" stroke-width="1.7"/><path d="M12 4v4M12 16v4M4 12h4M16 12h4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>';
  }
  if (item.file_kind === "script") {
    return '<svg viewBox="0 0 24 24" fill="none" class="w-4 h-4"><path d="M8 6h8M8 10h8M8 14h5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/><path d="M6 4h12v16H6z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>';
  }
  if (item.file_kind === "style") {
    return '<svg viewBox="0 0 24 24" fill="none" class="w-4 h-4"><path d="M7 5h10v14H7z" stroke="currentColor" stroke-width="1.7"/><path d="M9 9h6M9 13h6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>';
  }
  return '<svg viewBox="0 0 24 24" fill="none" class="w-4 h-4"><path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>';
}

function setActivePane(root, mode) {
  const triggers = root.querySelectorAll("[data-template-pane-trigger]");
  const panes = root.querySelectorAll("[data-template-pane]");
  for (const trigger of triggers) {
    const triggerMode = trigger.getAttribute("data-template-pane-trigger");
    trigger.classList.toggle("is-active", triggerMode === mode);
  }
  for (const pane of panes) {
    const paneMode = pane.getAttribute("data-template-pane");
    pane.classList.toggle("is-active", paneMode === mode);
  }
}

function initPaneToggles(root) {
  const triggers = root.querySelectorAll("[data-template-pane-trigger]");
  for (const trigger of triggers) {
    trigger.addEventListener("click", () => {
      const mode = trigger.getAttribute("data-template-pane-trigger") || "files";
      setActivePane(root, mode);
    });
  }
}

async function requestJson(url, options = {}) {
  const response = await fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  });

  if (response.status === 204) {
    return null;
  }
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
}

function dirname(relPath) {
  if (!relPath || !relPath.includes("/")) {
    return "";
  }
  return relPath.slice(0, relPath.lastIndexOf("/"));
}

function encodePath(path) {
  return encodeURIComponent(path);
}

function setIndicator(element, value, state) {
  if (!element) {
    return;
  }
  element.textContent = value;
  element.setAttribute("data-state", state || "neutral");
}

function setStatus(state, value, kind = "neutral") {
  setIndicator(state.statusEl, value, kind);
}

function showToast(state, message, kind = "info") {
  if (!state.toastHost) {
    return;
  }
  const toast = document.createElement("div");
  toast.className = `template-toast is-${kind}`;
  toast.textContent = message;
  state.toastHost.appendChild(toast);
  window.setTimeout(() => {
    toast.classList.add("is-leaving");
    window.setTimeout(() => toast.remove(), 220);
  }, 2600);
}

function setEditorDoc(view, content) {
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: content },
  });
}

function buildTreeItemHtml(item, state) {
  const isSelected = item.kind === "folder"
    ? item.rel_path === state.selectedFolder
    : !state.selectedFolder && item.rel_path === state.selectedFile;
  const isOpenFile = item.kind === "file" && item.rel_path === state.selectedFile;
  const classes = [
    "template-tree-item",
    item.kind === "folder" ? "is-folder" : "",
    item.is_protected ? "is-protected" : "",
    isOpenFile ? "is-open" : "",
    isSelected ? "is-selected" : "",
  ]
    .filter(Boolean)
    .join(" ");
  const gitCode = state.gitMap.get(item.rel_path) || "";
  const attrs = [
    `class="${classes}"`,
    `style="padding-left:${12 + item.depth * 14}px"`,
    `data-template-rel-path="${item.rel_path}"`,
    item.kind === "folder"
      ? 'data-template-folder-item="true"'
      : 'data-template-file-item="true"',
    `data-template-protected="${item.is_protected ? "true" : "false"}"`,
    'draggable="true"',
  ].join(" ");
  const lock = item.is_protected
    ? '<span class="template-tree-lock" title="Protected"><svg viewBox="0 0 24 24" fill="none" class="w-3.5 h-3.5"><path d="M8 11V8a4 4 0 118 0v3M7 11h10v9H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg></span>'
    : "";
  return `<div ${attrs}>
    <span class="template-tree-icon">${iconForItem(item)}</span>
    <span class="template-tree-label">${item.name}</span>
    ${lock}
    <span class="template-tree-git">${gitCode}</span>
  </div>`;
}

function renderTree(state) {
  const tree = state.treeEl;
  tree.innerHTML = state.workspace.items.map((item) => buildTreeItemHtml(item, state)).join("");
}

function renderSearch(state) {
  const query = (state.searchQuery || "").trim().toLowerCase();
  const results = !query
    ? []
    : state.workspace.items.filter(
        (item) => item.kind === "file" && item.rel_path.toLowerCase().includes(query),
      );
  state.searchResultsEl.innerHTML = results.length
    ? results
        .map(
          (item) => `<button type="button" class="template-search-result" data-template-search-rel-path="${item.rel_path}">
            <span class="template-search-icon">${iconForItem(item)}</span>
            <span class="template-search-main">${item.name}</span>
            <span class="template-search-meta">${item.rel_path}</span>
          </button>`,
        )
        .join("")
    : `<div class="template-search-empty">${query ? "No matching template files." : "Type to search templates."}</div>`;

  state.searchResultsEl.querySelectorAll("[data-template-search-rel-path]").forEach((node) => {
    node.addEventListener("click", () => {
      const relPath = node.getAttribute("data-template-search-rel-path");
      setActivePane(state.root, "files");
      openFile(state, relPath);
    });
  });
}

function renderGit(state) {
  if (!state.gitItems.length) {
    state.gitEl.innerHTML =
      '<div class="template-git-empty">No template changes detected.</div>';
    return;
  }
  state.gitEl.innerHTML = state.gitItems
    .map(
      (item) => `<button type="button" class="template-git-item" data-template-git-rel-path="${item.rel_path}">
        <span class="template-git-code">${item.code}</span>
        <span class="template-git-path">${item.rel_path}</span>
      </button>`,
    )
    .join("");
  state.gitEl.querySelectorAll("[data-template-git-rel-path]").forEach((node) => {
    node.addEventListener("click", () => {
      const relPath = node.getAttribute("data-template-git-rel-path");
      setActivePane(state.root, "files");
      openFile(state, relPath);
    });
  });
}

function updateTab(state) {
  const name = state.currentFile?.name || "No file";
  const kind = state.currentFile?.file_kind || "";
  const label = state.tabEl.querySelector(".template-editor-tab-label");
  const meta = state.tabEl.querySelector(".template-editor-tab-kind");
  if (label) {
    label.textContent = name;
  }
  if (meta) {
    meta.textContent = kind;
  }

  if (state.currentFileStatusEl) {
    state.currentFileStatusEl.textContent = name;
  }
  if (state.currentFileStatusIconEl) {
    state.currentFileStatusIconEl.setAttribute(
      "title",
      state.currentFile?.rel_path || name,
    );
  }
}

function setSelectedFolder(state, relPath) {
  state.selectedFolder = relPath || "";
  renderTree(state);
}

function currentImportSuggestions(state) {
  return state.workspace.items
    .filter((item) => item.kind === "file")
    .map((item) => {
      const pathWithoutExt = item.rel_path.replace(/\.(tsx|ts|css)$/i, "");
      return {
        label: `@/${pathWithoutExt}`,
        type: "variable",
        detail: item.file_kind,
      };
    });
}

function templateCompletions(state) {
  return (context) => {
    const word = context.matchBefore(/[@A-Za-z0-9_./-]*/);
    if (!word || (word.from === word.to && !context.explicit)) {
      return null;
    }
    const typed = word.text || "";
    const options = [];

    if (typed.startsWith("@/")) {
      options.push(...currentImportSuggestions(state).filter((item) => item.label.startsWith(typed)));
    } else {
      options.push(
        ...state.keywordSymbols.filter((item) => item.label.toLowerCase().startsWith(typed.toLowerCase())),
      );
    }

    if (!options.length) {
      return null;
    }
    return {
      from: word.from,
      options,
      validFor: /[@A-Za-z0-9_./-]*/,
    };
  };
}

function editorExtensions(state, fileKind) {
  const extensions = [
    basicSetup,
    oneDark,
    autocompletion({ override: [templateCompletions(state)] }),
    linter(() => []),
    lintGutter(),
    EditorView.updateListener.of((update) => {
      if (!update.docChanged) {
        return;
      }
      state.isDirty = true;
      setIndicator(state.saveStateEl, "Unsaved", "warning");
      setStatus(state, "Dirty", "warning");
      state.markDraftReady();
    }),
  ];

  if (fileKind === "style") {
    extensions.push(css());
  } else {
    extensions.push(javascript({ jsx: true, typescript: true }));
  }

  return extensions;
}

function mountEditor(state, content, fileKind) {
  if (state.view) {
    state.view.destroy();
    state.view = null;
  }
  const view = new EditorView({
    doc: content,
    extensions: editorExtensions(state, fileKind),
    parent: state.host,
  });
  state.host.setAttribute("data-template-editor-ready", "true");
  state.sourceEl.setAttribute("hidden", "hidden");
  state.view = view;
}

function applyDiagnostics(state, diagnostics) {
  state.currentDiagnostics = diagnostics || [];
  if (!state.view) {
    return;
  }
  const cmDiagnostics = (diagnostics || []).map((diag) => ({
    from: typeof diag.from === "number" ? diag.from : 0,
    to: typeof diag.to === "number" ? diag.to : Math.min(1, state.view.state.doc.length || 1),
    severity: diag.severity === "error" ? "error" : "warning",
    message: diag.message,
  }));
  try {
    state.view.dispatch(setDiagnostics(state.view.state, cmDiagnostics));
  } catch (error) {
    console.error("[ZEBFLOW][TEMPLATES] diagnostics apply failed", error);
  }
}

async function runDiagnostics(state, contentOverride = null) {
  if (!state.currentFile) {
    return { ok: true, diagnostics: [] };
  }
  const payload = await requestJson(state.api.diagnostics, {
    method: "POST",
    body: JSON.stringify({
      rel_path: state.currentFile.rel_path,
      content: contentOverride ?? state.view.state.doc.toString(),
    }),
  });
  applyDiagnostics(state, payload.diagnostics || []);
  const hasError = (payload.diagnostics || []).some((item) => item.severity === "error");
  if (hasError || payload.ok === false) {
    setIndicator(state.compileStateEl, "Error", "error");
    setStatus(state, "Compile error", "error");
  } else if ((payload.diagnostics || []).length) {
    setIndicator(state.compileStateEl, "Warnings", "warning");
    setStatus(state, "Compile warnings", "warning");
  } else {
    setIndicator(state.compileStateEl, "OK", "success");
    if (!state.isDirty) {
      setStatus(state, "Ready", "success");
    }
  }
  return payload;
}

function updateGitIndicator(state) {
  const code = state.currentFile ? state.gitMap.get(state.currentFile.rel_path) || "" : "";
  if (!code) {
    setIndicator(state.gitStateEl, "Synced", "success");
    return;
  }
  const label = code === "??" ? "Untracked" : `Changed ${code}`;
  setIndicator(state.gitStateEl, label, "warning");
}

async function refreshWorkspace(state, preferredFile = null) {
  const workspace = await requestJson(state.api.workspace);
  state.workspace = workspace;
  await refreshGitStatus(state);
  renderTree(state);
  renderSearch(state);

  const nextFile =
    preferredFile ||
    (state.currentFile && workspace.items.some((item) => item.rel_path === state.currentFile.rel_path)
      ? state.currentFile.rel_path
      : workspace.default_file);

  if (nextFile) {
    await openFile(state, nextFile, { silentStatus: true });
  } else {
    state.currentFile = null;
    updateTab(state);
  }
}

async function refreshGitStatus(state) {
  const items = await requestJson(state.api.gitStatus);
  state.gitItems = items || [];
  state.gitMap = new Map((state.gitItems || []).map((item) => [item.rel_path, item.code]));
  renderGit(state);
  updateGitIndicator(state);
}

async function openFile(state, relPath, options = {}) {
  if (!relPath) {
    return;
  }
  const file = await requestJson(`${state.api.file}?path=${encodePath(relPath)}`);
  state.currentFile = file;
  state.selectedFile = file.rel_path;
  state.selectedFolder = dirname(file.rel_path);
  state.lastSavedContent = file.content;
  state.isDirty = false;
  state.sourceEl.value = file.content;
  mountEditor(state, file.content, file.file_kind);
  updateTab(state);
  renderTree(state);
  setIndicator(state.saveStateEl, "Saved", "success");
  state.deleteButton?.toggleAttribute("disabled", !!file.is_protected);
  updateGitIndicator(state);
  await runDiagnostics(state, file.content);
  if (!options.silentStatus) {
    setStatus(state, "Ready", "success");
  }
}

async function saveCurrent(state) {
  if (!state.currentFile) {
    return;
  }
  const content = state.view.state.doc.toString();
  setStatus(state, "Saving", "warning");
  const payload = await requestJson(state.api.save, {
    method: "PUT",
    body: JSON.stringify({
      rel_path: state.currentFile.rel_path,
      content,
    }),
  });
  state.currentFile = payload;
  state.selectedFile = payload.rel_path;
  state.lastSavedContent = payload.content;
  state.isDirty = false;
  updateTab(state);
  setIndicator(state.saveStateEl, "Saved", "success");
  await refreshWorkspace(state, payload.rel_path);
  const compile = await runDiagnostics(state, payload.content);
  const hasError = (compile.diagnostics || []).some((item) => item.severity === "error");
  if (hasError || compile.ok === false) {
    showToast(state, "Saved with compile errors", "error");
  } else {
    showToast(state, "File saved", "success");
  }
  setStatus(state, hasError ? "Saved with errors" : "Saved", hasError ? "error" : "success");
}

async function createEntry(state, kind) {
  const label = kind === "folder" ? "folder" : kind;
  const name = window.prompt(`New ${label} name`);
  if (!name) {
    return;
  }
  const parent = state.selectedFolder || (state.currentFile ? dirname(state.currentFile.rel_path) : "");
  const payload = await requestJson(state.api.create, {
    method: "POST",
    body: JSON.stringify({
      kind,
      name,
      parent_rel_path: parent,
    }),
  });
  if (payload?.file_kind === "folder") {
    await refreshWorkspace(state);
    showToast(state, "Folder created", "success");
    return;
  }
  await refreshWorkspace(state, payload.rel_path);
  showToast(state, `${kind} created`, "success");
  setStatus(state, "Created", "success");
}

async function moveEntry(state, fromRelPath, toParentRelPath) {
  setStatus(state, "Moving", "warning");
  const payload = await requestJson(state.api.move, {
    method: "POST",
    body: JSON.stringify({
      from_rel_path: fromRelPath,
      to_parent_rel_path: toParentRelPath,
    }),
  });
  const preferred = state.currentFile?.rel_path === fromRelPath ? payload.rel_path : state.currentFile?.rel_path;
  await refreshWorkspace(state, preferred);
  showToast(state, "Entry moved", "success");
  setStatus(state, "Moved", "success");
}

async function deleteCurrent(state) {
  if (!state.currentFile) {
    return;
  }
  if (state.currentFile.is_protected) {
    showToast(state, "Protected file cannot be deleted", "error");
    setStatus(state, "Protected", "error");
    return;
  }
  const relPath = state.currentFile.rel_path;
  const ok = window.confirm(`Delete ${relPath}?`);
  if (!ok) {
    return;
  }
  setStatus(state, "Deleting", "warning");
  await requestJson(`${state.api.delete}?path=${encodePath(relPath)}`, {
    method: "DELETE",
  });
  state.currentFile = null;
  state.selectedFile = "";
  await refreshWorkspace(state);
  showToast(state, "File deleted", "success");
  setStatus(state, "Deleted", "success");
}

async function loadKeywordSymbols() {
  const payload = await requestJson(LIBRARY_KEYWORDS_URL);
  const symbols = (payload?.symbols || []).map((item) => ({
    label: item.name,
    type: item.kind === "wrapper" ? "class" : "variable",
    detail: item.import,
  }));
  return [...ZEBFLOW_KEYWORDS, ...symbols];
}

function bindUi(state) {
  state.treeEl?.addEventListener("click", (event) => {
    const fileNode = event.target.closest("[data-template-file-item]");
    if (fileNode) {
      event.preventDefault();
      openFile(state, fileNode.getAttribute("data-template-rel-path"));
      return;
    }
    const folderNode = event.target.closest("[data-template-folder-item]");
    if (folderNode) {
      event.preventDefault();
      setSelectedFolder(state, folderNode.getAttribute("data-template-rel-path") || "");
    }
  });

  state.treeEl?.addEventListener("dragstart", (event) => {
    const item = event.target.closest("[draggable='true']");
    if (!item) {
      return;
    }
    state.draggingRelPath = item.getAttribute("data-template-rel-path");
    event.dataTransfer?.setData("text/plain", state.draggingRelPath || "");
    event.dataTransfer.effectAllowed = "move";
  });

  state.treeEl?.addEventListener("dragend", () => {
    state.draggingRelPath = null;
    state.treeEl.querySelectorAll(".is-drop-target").forEach((node) => node.classList.remove("is-drop-target"));
  });

  state.treeEl?.addEventListener("dragover", (event) => {
    event.preventDefault();
    const folderNode = event.target.closest("[data-template-folder-item]");
    state.treeEl.querySelectorAll(".is-drop-target").forEach((node) => {
      if (node !== folderNode) {
        node.classList.remove("is-drop-target");
      }
    });
    if (folderNode) {
      folderNode.classList.add("is-drop-target");
    }
  });

  state.treeEl?.addEventListener("dragleave", (event) => {
    const folderNode = event.target.closest("[data-template-folder-item]");
    if (folderNode) {
      folderNode.classList.remove("is-drop-target");
    }
  });

  state.treeEl?.addEventListener("drop", async (event) => {
    event.preventDefault();
    const folderNode = event.target.closest("[data-template-folder-item]");
    state.treeEl.querySelectorAll(".is-drop-target").forEach((node) => node.classList.remove("is-drop-target"));
    const fromRelPath = state.draggingRelPath || event.dataTransfer?.getData("text/plain");
    if (!fromRelPath) {
      return;
    }
    const toParentRelPath = folderNode?.getAttribute("data-template-rel-path") || "";
    if (!toParentRelPath && !dirname(fromRelPath)) {
      return;
    }
    if (dirname(fromRelPath) === toParentRelPath) {
      return;
    }
    await moveEntry(state, fromRelPath, toParentRelPath);
  });

  state.root.querySelector("[data-template-new-folder]")?.addEventListener("click", () => {
    createEntry(state, "folder").catch((error) => {
      setStatus(state, "Create failed", "error");
      showToast(state, error.message || "Create failed", "error");
      console.error("[ZEBFLOW][TEMPLATES] create folder failed", error);
    });
  });

  state.root.querySelectorAll("[data-template-create-kind]").forEach((node) => {
    node.addEventListener("click", () => {
      const kind = node.getAttribute("data-template-create-kind");
      node.closest("details")?.removeAttribute("open");
      createEntry(state, kind).catch((error) => {
        setStatus(state, "Create failed", "error");
        showToast(state, error.message || "Create failed", "error");
        console.error("[ZEBFLOW][TEMPLATES] create entry failed", error);
      });
    });
  });

  state.root.querySelector("[data-template-save]")?.addEventListener("click", () => {
    saveCurrent(state).catch((error) => {
      setStatus(state, "Save failed", "error");
      setIndicator(state.compileStateEl, "Error", "error");
      showToast(state, error.message || "Save failed", "error");
      console.error("[ZEBFLOW][TEMPLATES] save failed", error);
    });
  });

  state.root.querySelector("[data-template-delete]")?.addEventListener("click", () => {
    deleteCurrent(state).catch((error) => {
      setStatus(state, "Delete failed", "error");
      showToast(state, error.message || "Delete failed", "error");
      console.error("[ZEBFLOW][TEMPLATES] delete failed", error);
    });
  });

  state.searchInputEl?.addEventListener("input", (event) => {
    state.searchQuery = event.target.value || "";
    renderSearch(state);
  });

  state.root.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveCurrent(state).catch((error) => {
        setStatus(state, "Save failed", "error");
        setIndicator(state.compileStateEl, "Error", "error");
        showToast(state, error.message || "Save failed", "error");
        console.error("[ZEBFLOW][TEMPLATES] save failed", error);
      });
    }
  });
}

function createWorkspaceState(root) {
  return {
    root,
    api: {
      workspace: root.dataset.templateApiWorkspace,
      file: root.dataset.templateApiFile,
      save: root.dataset.templateApiSave,
      create: root.dataset.templateApiCreate,
      move: root.dataset.templateApiMove,
      delete: root.dataset.templateApiDelete,
      gitStatus: root.dataset.templateApiGitStatus,
      diagnostics: root.dataset.templateApiDiagnostics,
    },
    selectedFile: root.dataset.templateSelectedFile || "",
    selectedFolder: "",
    workspace: { default_file: null, items: [] },
    currentFile: null,
    currentDiagnostics: [],
    keywordSymbols: [...ZEBFLOW_KEYWORDS],
    gitItems: [],
    gitMap: new Map(),
    isDirty: false,
    draggingRelPath: null,
    searchQuery: "",
    treeEl: root.querySelector("[data-template-tree]"),
    gitEl: root.querySelector("[data-template-git-status]"),
    searchInputEl: root.querySelector("[data-template-search-input]"),
    searchResultsEl: root.querySelector("[data-template-search-results]"),
    host: root.querySelector("[data-template-editor-host]"),
    sourceEl: root.querySelector("[data-template-editor-source]"),
    toastHost: root.querySelector("[data-template-sonner]"),
    statusEl: root.querySelector("[data-template-status]"),
    saveStateEl: root.querySelector("[data-template-save-state]"),
    gitStateEl: root.querySelector("[data-template-git-state]"),
    compileStateEl: root.querySelector("[data-template-compile-state]"),
    tabEl: root.querySelector("[data-template-editor-tab]"),
    currentFileStatusIconEl: root.querySelector("[data-template-current-file]"),
    currentFileStatusEl: root.querySelector("[data-template-current-file-value]"),
    deleteButton: root.querySelector("[data-template-delete]"),
    view: null,
    markDraftReady: null,
  };
}

async function bootWorkspace(root) {
  const state = createWorkspaceState(root);
  state.keywordSymbols = await loadKeywordSymbols().catch((error) => {
    console.error("[ZEBFLOW][TEMPLATES] keyword load failed", error);
    return [...ZEBFLOW_KEYWORDS];
  });
  state.markDraftReady = debounce(() => {
    if (!state.isDirty) {
      return;
    }
    setStatus(state, "Draft ready", "warning");
  }, 240);

  setActivePane(root, "files");
  initPaneToggles(root);
  createSplitPane(root, {
    handleSelector: "[data-template-split-handle]",
    targetSelector: "[data-split-target]",
    variable: "--template-sidebar-width",
    min: 220,
    max: 420,
  });
  bindUi(state);
  setIndicator(state.saveStateEl, "Clean", "neutral");
  setIndicator(state.gitStateEl, "Synced", "success");
  setIndicator(state.compileStateEl, "Unknown", "neutral");
  setStatus(state, "Booting", "neutral");
  await refreshWorkspace(state, state.selectedFile || null);
}

document.querySelectorAll("[data-template-workspace]").forEach((root) => {
  bootWorkspace(root).catch((error) => {
    const status = root.querySelector("[data-template-status]");
    if (status) {
      status.textContent = "Workspace failed";
      status.setAttribute("data-state", "error");
    }
    console.error("[ZEBFLOW][TEMPLATES] workspace bootstrap failed", error);
  });
});
