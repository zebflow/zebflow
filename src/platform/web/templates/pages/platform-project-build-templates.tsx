import { useState, useEffect, useRef, cx, Link } from "rwe";
import ProjectStudioShell from "@/components/layout/project-studio-shell";
import Sonner from "@/components/ui/sonner";
import TemplateFolderTree from "@/components/ui/template-folder-tree";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { loadEditorRuntime } from "@/components/behavior/template-editor-runtime";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
    links: [
      { rel: "stylesheet", href: "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css" },
    ],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

// --- Utilities ---

function tplParentDir(relPath) {
  if (!relPath || !String(relPath).includes("/")) return "";
  const p = String(relPath);
  return p.slice(0, p.lastIndexOf("/"));
}

function encodePath(path) {
  return encodeURIComponent(path);
}

async function requestJson(url, options: any = {}) {
  const response = await fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  });
  if (response.status === 204) return null;
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

// --- Workspace Component ---

function TemplateWorkspace({ api, initialFile, initialItems }) {
  const [items, setItems] = useState(Array.isArray(initialItems) ? initialItems : []);
  const [currentFile, setCurrentFile] = useState(initialFile ?? null);
  const [activePane, setActivePane] = useState("files");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState([]);
  const [gitItems, setGitItems] = useState([]);
  const [saveState, setSaveState] = useState("Clean");
  const [gitState, setGitState] = useState("Synced");
  const [compileState, setCompileState] = useState("Unknown");
  const [editorStatus, setEditorStatus] = useState("Booting");
  const [toasts, setToasts] = useState([]);
  const [createMenuOpen, setCreateMenuOpen] = useState(false);

  const editorHostRef = useRef(null);
  const editorViewRef = useRef(null);
  const workspaceRef = useRef(null);
  const currentFileRef = useRef(initialFile ?? null);
  const itemsRef = useRef(Array.isArray(initialItems) ? initialItems : []);
  const keywordSymbolsRef = useRef([
    { label: "Page", type: "type" },
    { label: "usePageState", type: "function" },
  ]);
  const gitMapRef = useRef(new Map());
  const runtimeRef = useRef(null);

  // --- Toast ---

  function pushToast(msg, variant = "info") {
    const id = Date.now();
    setToasts((t) => [...t, { id, msg, variant }]);
    if (typeof window !== "undefined") {
      window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 2800);
    }
  }

  // --- Runtime ---

  async function loadRuntime() {
    if (runtimeRef.current) return runtimeRef.current;
    const rt = await loadEditorRuntime();
    runtimeRef.current = rt;
    return rt;
  }

  // --- Editor ---

  function buildCompletionSource() {
    return (context) => {
      const word = context.matchBefore(/[@A-Za-z0-9_./-]*/);
      if (!word || (word.from === word.to && !context.explicit)) return null;
      const typed = word.text || "";
      const options = [];
      if (typed.startsWith("@/")) {
        const suggestions = itemsRef.current
          .filter((item) => item.kind === "file")
          .map((item) => ({
            label: `@/${item.rel_path.replace(/\.(tsx|ts|css)$/i, "")}`,
            type: "variable",
            detail: item.file_kind,
          }));
        options.push(...suggestions.filter((item) => item.label.startsWith(typed)));
      } else {
        options.push(
          ...keywordSymbolsRef.current.filter((item) =>
            item.label.toLowerCase().startsWith(typed.toLowerCase())
          )
        );
      }
      if (!options.length) return null;
      return { from: word.from, options, validFor: /[@A-Za-z0-9_./-]*/ };
    };
  }

  function mountEditor(content, fileKind, rt) {
    if (editorViewRef.current) {
      editorViewRef.current.destroy();
      editorViewRef.current = null;
    }
    if (!editorHostRef.current) return;
    const { EditorView, basicSetup, javascript, css, autocompletion, linter, lintGutter, oneDark } = rt.cm;
    const extensions = [
      basicSetup,
      oneDark,
      autocompletion({ override: [buildCompletionSource()] }),
      linter(() => []),
      lintGutter(),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) return;
        setSaveState("Unsaved");
        setEditorStatus("Dirty");
      }),
    ];
    if (fileKind === "style") {
      extensions.push(css());
    } else {
      extensions.push(javascript({ jsx: true, typescript: true }));
    }
    editorViewRef.current = new EditorView({
      doc: content,
      extensions,
      parent: editorHostRef.current,
    });
  }

  async function applyDiagnostics(diagnostics, rt) {
    if (!editorViewRef.current || !rt?.cm?.setDiagnostics) return;
    const { setDiagnostics } = rt.cm;
    const view = editorViewRef.current;
    const cmDiags = (diagnostics || []).map((d) => ({
      from: typeof d.from === "number" ? d.from : 0,
      to: typeof d.to === "number" ? d.to : Math.min(1, view.state.doc.length || 1),
      severity: d.severity === "error" ? "error" : "warning",
      message: d.message,
    }));
    try {
      view.dispatch(setDiagnostics(view.state, cmDiags));
    } catch (e) {
      console.error("[TEMPLATES] diagnostics apply failed", e);
    }
  }

  async function runDiagnostics(relPath, content) {
    try {
      const payload = await requestJson(api.diagnostics, {
        method: "POST",
        body: JSON.stringify({ rel_path: relPath, content }),
      });
      const hasError = (payload.diagnostics || []).some((d) => d.severity === "error");
      if (hasError || payload.ok === false) {
        setCompileState("Error");
      } else if ((payload.diagnostics || []).length) {
        setCompileState("Warnings");
      } else {
        setCompileState("OK");
      }
      if (runtimeRef.current) {
        await applyDiagnostics(payload.diagnostics, runtimeRef.current);
      }
      return payload;
    } catch (err) {
      console.error("[TEMPLATES] diagnostics failed", err);
      return { ok: false, diagnostics: [] };
    }
  }

  // --- Git ---

  function updateGitIndicator(map) {
    const gm = map ?? gitMapRef.current;
    const code = currentFileRef.current ? gm.get(currentFileRef.current.rel_path) || "" : "";
    if (!code) {
      setGitState("Synced");
    } else {
      setGitState(code === "??" ? "Untracked" : `Changed ${code}`);
    }
  }

  async function fetchGitStatus() {
    try {
      const gitList = await requestJson(api.git_status);
      const newGitItems = gitList || [];
      const newGitMap = new Map(newGitItems.map((item) => [item.rel_path, item.code]));
      setGitItems(newGitItems);
      gitMapRef.current = newGitMap;
      updateGitIndicator(newGitMap);
    } catch (err) {
      console.error("[TEMPLATES] git status failed", err);
    }
  }

  // --- Workspace ---

  async function refreshWorkspace() {
    try {
      const workspace = await requestJson(api.workspace);
      const newItems = Array.isArray(workspace?.items) ? workspace.items : [];
      setItems(newItems);
      itemsRef.current = newItems;
    } catch (err) {
      console.error("[TEMPLATES] workspace refresh failed", err);
    }
  }

  // --- File operations ---

  async function openFile(relPath) {
    if (!relPath) return;
    try {
      const file = await requestJson(`${api.file}?path=${encodePath(relPath)}`);
      currentFileRef.current = file;
      setCurrentFile(file);
      setSaveState("Saved");
      if (runtimeRef.current && editorHostRef.current) {
        mountEditor(file.content, file.file_kind, runtimeRef.current);
      }
      updateGitIndicator(gitMapRef.current);
      await runDiagnostics(file.rel_path, file.content);
      setEditorStatus("Ready");
    } catch (err) {
      pushToast(err.message || "Failed to open file", "error");
      setEditorStatus("Error");
    }
  }

  async function saveCurrentFile() {
    if (!currentFileRef.current || !editorViewRef.current) return;
    try {
      setEditorStatus("Saving");
      const content = editorViewRef.current.state.doc.toString();
      const payload = await requestJson(api.save, {
        method: "PUT",
        body: JSON.stringify({ rel_path: currentFileRef.current.rel_path, content }),
      });
      currentFileRef.current = payload;
      setCurrentFile(payload);
      setSaveState("Saved");
      await refreshWorkspace();
      const compile = await runDiagnostics(payload.rel_path, payload.content);
      const hasError = (compile.diagnostics || []).some((d) => d.severity === "error");
      pushToast(
        hasError || compile.ok === false ? "Saved with compile errors" : "File saved",
        hasError ? "error" : "success"
      );
      setEditorStatus(hasError ? "Saved with errors" : "Saved");
    } catch (err) {
      pushToast(err.message || "Save failed", "error");
      setEditorStatus("Save failed");
    }
  }

  async function deleteCurrentFile() {
    if (!currentFileRef.current) return;
    if (currentFileRef.current.is_protected) {
      pushToast("Protected file cannot be deleted", "error");
      return;
    }
    const relPath = currentFileRef.current.rel_path;
    if (!window.confirm(`Delete ${relPath}?`)) return;
    try {
      setEditorStatus("Deleting");
      await requestJson(`${api.delete}?path=${encodePath(relPath)}`, { method: "DELETE" });
      currentFileRef.current = null;
      setCurrentFile(null);
      await refreshWorkspace();
      pushToast("File deleted", "success");
      setEditorStatus("Deleted");
    } catch (err) {
      pushToast(err.message || "Delete failed", "error");
      setEditorStatus("Delete failed");
    }
  }

  async function createEntry(kind) {
    setCreateMenuOpen(false);
    const label = kind === "folder" ? "folder" : kind;
    const name = window.prompt(`New ${label} name`);
    if (!name) return;
    const parentRelPath = currentFileRef.current ? tplParentDir(currentFileRef.current.rel_path) : "";
    try {
      const payload = await requestJson(api.create, {
        method: "POST",
        body: JSON.stringify({ kind, name, parent_rel_path: parentRelPath }),
      });
      await refreshWorkspace();
      if (payload?.rel_path && payload?.kind !== "folder" && payload?.file_kind !== "folder") {
        await openFile(payload.rel_path);
      }
      pushToast(`${kind} created`, "success");
      setEditorStatus("Created");
    } catch (err) {
      pushToast(err.message || "Create failed", "error");
      setEditorStatus("Create failed");
    }
  }

  // --- Keyboard ---

  function handleKeyDown(event) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveCurrentFile();
    }
  }

  // --- Effects ---

  useEffect(() => {
    if (typeof window === "undefined") return;
    let destroyed = false;

    async function boot() {
      const rt = await loadRuntime();
      if (destroyed) return;

      // Load keyword symbols
      try {
        const kwPayload = await requestJson(
          "/assets/libraries/zeb/codemirror/0.1/keywords.json"
        );
        const baseSymbols = [
          { label: "Page", type: "type" },
          { label: "usePageState", type: "function" },
        ];
        const libSymbols = (kwPayload?.symbols || []).map((item) => ({
          label: item.name,
          type: item.kind === "wrapper" ? "class" : "variable",
          detail: item.import,
        }));
        keywordSymbolsRef.current = [...baseSymbols, ...libSymbols];
      } catch (err) {
        console.error("[TEMPLATES] keyword load failed", err);
      }

      if (destroyed) return;

      // Mount editor with initial content
      if (editorHostRef.current) {
        const initialContent = initialFile?.content || "";
        const initialKind = initialFile?.file_kind || "script";
        mountEditor(initialContent, initialKind, rt);
      }

      // Setup split pane
      if (workspaceRef.current) {
        rt.interact.createSplitPane(workspaceRef.current, {
          handleSelector: ".template-split-handle",
          targetSelector: "[data-split-target]",
          variable: "--template-sidebar-width",
          min: 220,
          max: 420,
        });
      }

      if (destroyed) return;
      setEditorStatus("Ready");

      await refreshWorkspace();
      await fetchGitStatus();

      if (initialFile?.rel_path) {
        await runDiagnostics(initialFile.rel_path, initialFile.content || "");
      }
    }

    boot().catch((err) => {
      if (!destroyed) {
        setEditorStatus("Failed");
        console.error("[TEMPLATES] boot failed", err);
      }
    });

    return () => {
      destroyed = true;
      if (editorViewRef.current) {
        editorViewRef.current.destroy();
        editorViewRef.current = null;
      }
    };
  }, []);

  // Update search results when query or items change
  useEffect(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      setSearchResults([]);
      return;
    }
    const results = itemsRef.current.filter(
      (item) => item.kind === "file" && item.rel_path.toLowerCase().includes(query)
    );
    setSearchResults(results);
  }, [searchQuery, items]);

  // Derived
  const selectedPath = currentFile?.rel_path || "";
  const selectedFolder = currentFile ? tplParentDir(currentFile.rel_path) : "";

  // --- Render ---

  return (
    <div className="template-workspace" ref={workspaceRef} onKeyDown={handleKeyDown}>
      <aside className="template-sidebar" data-split-target="true">
        <div className="template-sidebar-toolbar">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className={cx("template-sidebar-icon", activePane === "files" && "is-active")}
            title="Files"
            onClick={() => setActivePane("files")}
          >
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M4 6h7l2 2h7v10H4z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
            </svg>
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className={cx("template-sidebar-icon", activePane === "search" && "is-active")}
            title="Search"
            onClick={() => setActivePane("search")}
          >
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <circle cx="11" cy="11" r="6" stroke="currentColor" strokeWidth="1.7" />
              <path d="M16 16l4 4" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className={cx("template-sidebar-icon", activePane === "git" && "is-active")}
            title="Git"
            onClick={() => setActivePane("git")}
          >
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <circle cx="6" cy="6" r="2" stroke="currentColor" strokeWidth="1.7" />
              <circle cx="18" cy="6" r="2" stroke="currentColor" strokeWidth="1.7" />
              <circle cx="12" cy="18" r="2" stroke="currentColor" strokeWidth="1.7" />
              <path d="M8 7l3 9M16 7l-3 9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
          </Button>

          <span className="template-sidebar-toolbar-spacer"></span>

          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="template-sidebar-icon"
            title="New Folder"
            onClick={() => createEntry("folder")}
          >
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M4 7h6l2 2h8v8H4z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
              <path d="M12 11v4M10 13h4" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
          </Button>

          <div className="template-create-menu relative">
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="template-sidebar-icon"
              title="New"
              onClick={() => setCreateMenuOpen((o) => !o)}
            >
              <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                <path d="M12 5v14M5 12h14" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
              </svg>
            </Button>
            {createMenuOpen && (
              <>
                <div className="fixed inset-0 z-10" onClick={() => setCreateMenuOpen(false)} />
                <div className="template-create-menu-panel relative z-20">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="template-create-item"
                    onClick={() => createEntry("page")}
                  >
                    Page
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="template-create-item"
                    onClick={() => createEntry("component")}
                  >
                    Component
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="template-create-item"
                    onClick={() => createEntry("script")}
                  >
                    Script
                  </Button>
                </div>
              </>
            )}
          </div>
        </div>

        <section className={cx("template-sidebar-pane", activePane === "files" && "is-active")}>
          <div className="template-tree">
            <TemplateFolderTree
              items={items}
              selectedFile={selectedPath}
              selectedFolder={selectedFolder}
              onSelectFile={openFile}
            />
          </div>
        </section>

        <section className={cx("template-sidebar-pane", activePane === "search" && "is-active")}>
          <div className="template-mode-search">
            <div className="template-search-input-wrap">
              <Input
                type="search"
                className="template-search-input"
                placeholder="Search templates"
                value={searchQuery}
                onInput={(e) => setSearchQuery(e.target.value || "")}
              />
            </div>
            <div className="template-search-results">
              {searchResults.length > 0 ? (
                searchResults.map((item) => (
                  <Button
                    key={item.rel_path}
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="template-search-result"
                    onClick={() => {
                      setActivePane("files");
                      openFile(item.rel_path);
                    }}
                  >
                    <span className="template-search-main">{item.name}</span>
                    <span className="template-search-meta">{item.rel_path}</span>
                  </Button>
                ))
              ) : (
                <div className="template-search-empty">
                  {searchQuery ? "No matching template files." : "Type to search templates."}
                </div>
              )}
            </div>
          </div>
        </section>

        <section className={cx("template-sidebar-pane", activePane === "git" && "is-active")}>
          <div className="template-git-status">
            {gitItems.length > 0 ? (
              gitItems.map((item) => (
                <Button
                  key={item.rel_path}
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="template-git-item"
                  onClick={() => {
                    setActivePane("files");
                    openFile(item.rel_path);
                  }}
                >
                  <span className="template-git-code">{item.code}</span>
                  <span className="template-git-path">{item.rel_path}</span>
                </Button>
              ))
            ) : (
              <div className="template-git-empty">No template changes detected.</div>
            )}
          </div>
        </section>
      </aside>

      <div className="template-split-handle" aria-hidden="true"></div>

      <section className="template-editor-pane">
        <Sonner toasts={toasts} />
        <div className="template-editor-tabs">
          <div className="template-editor-tab is-active">
            <span className="template-editor-tab-label">{currentFile?.name || "No file"}</span>
            <span className="template-editor-tab-kind">{currentFile?.file_kind || ""}</span>
          </div>
          <div className="template-editor-tab-actions">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="template-editor-action"
              onClick={saveCurrentFile}
            >
              Save
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="template-editor-action is-danger"
              disabled={!currentFile || currentFile.is_protected}
              onClick={deleteCurrentFile}
            >
              Delete
            </Button>
          </div>
        </div>

        <div className="template-editor-surface">
          <div className="template-editor-host" ref={editorHostRef}></div>
        </div>

        <div className="template-editor-statusbar">
          <div className="template-editor-status-icon" title={currentFile?.rel_path || ""}>
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M7 4h7l4 4v12H7z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
              <path d="M14 4v4h4" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
            </svg>
            <span className="template-editor-status-value">{currentFile?.name || "-"}</span>
          </div>
          <div className="template-editor-status-icon" title="zeb/codemirror@0.1">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M8 6h8M8 10h8M8 14h5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
              <path d="M6 4h12v16H6z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
            </svg>
            <span className="template-editor-status-value">CM</span>
          </div>
          <div className="template-editor-status-icon" title="zeb/interact@0.1">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M8 7l3 9M16 7l-3 9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
              <circle cx="6" cy="6" r="2" stroke="currentColor" strokeWidth="1.7" />
              <circle cx="18" cy="6" r="2" stroke="currentColor" strokeWidth="1.7" />
              <circle cx="12" cy="18" r="2" stroke="currentColor" strokeWidth="1.7" />
            </svg>
            <span className="template-editor-status-value">IN</span>
          </div>
          <div className="template-editor-status-icon" title="zeb/stateutil@0.1">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M6 12h12M12 6v12" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
            <span className="template-editor-status-value">ST</span>
          </div>
          <div className="template-editor-status-spacer"></div>
          <div className="template-editor-status-icon" title="Save state">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M6 4h9l3 3v13H6z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
              <path d="M9 4v5h6" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
            </svg>
            <span className="template-editor-status-value">{saveState}</span>
          </div>
          <div className="template-editor-status-icon" title="Git status">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <circle cx="6" cy="6" r="2" stroke="currentColor" strokeWidth="1.7" />
              <circle cx="18" cy="6" r="2" stroke="currentColor" strokeWidth="1.7" />
              <circle cx="12" cy="18" r="2" stroke="currentColor" strokeWidth="1.7" />
              <path d="M8 7l3 9M16 7l-3 9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
            <span className="template-editor-status-value">{gitState}</span>
          </div>
          <div className="template-editor-status-icon" title="Compile state">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M12 8v5M12 17h.01" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
              <path
                d="M10.3 4.8L3.8 16a2 2 0 001.73 3h13a2 2 0 001.73-3L13.73 4.8a2 2 0 00-3.46 0z"
                stroke="currentColor"
                strokeWidth="1.7"
                strokeLinejoin="round"
              />
            </svg>
            <span className="template-editor-status-value">{compileState}</span>
          </div>
          <div className="template-editor-status-icon" title="Editor state">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <circle cx="12" cy="12" r="8" stroke="currentColor" strokeWidth="1.7" />
              <path d="M12 8v4l2.5 2.5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <span className="template-editor-status-value">{editorStatus}</span>
          </div>
        </div>
      </section>
    </div>
  );
}

export default function Page(input) {
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const workspace = input?.workspace ?? {};
  const workspaceApi = workspace?.api ?? {};
  const selectedFile = workspace?.selected_file ?? null;
  const workspaceItems = Array.isArray(workspace?.items) ? workspace.items : [];
  return (
    <>
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu={input.current_menu}
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
        <div className="project-workspace">
          <nav className="project-tab-strip">
            <Link href={navLinks.build_templates ?? "#"} className={cx("project-tab-link", navClasses.build_templates)}>
              Templates
            </Link>
            <Link href={navLinks.build_assets ?? "#"} className={cx("project-tab-link", navClasses.build_assets)}>
              Assets
            </Link>
            <Link href={navLinks.build_docs ?? "#"} className={cx("project-tab-link", navClasses.build_docs)}>
              Docs
            </Link>
          </nav>
          <section className="project-workspace-body">
            <TemplateWorkspace api={workspaceApi} initialFile={selectedFile} initialItems={workspaceItems} />
          </section>
        </div>
      </ProjectStudioShell>
    </>
  );
}
