import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { loadEditorRuntime } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/template-editor-runtime";
import { cx, Link, useEffect, useState, useRef, useNavigate } from "zeb";
import { useSplitPane } from "zeb/use";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";
import Input from "@/components/ui/input";
import PipelineEditor from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/index";
import { Select, SelectOption } from "@/components/ui/select";
import DropdownMenu from "@/components/ui/dropdown-menu";
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";

import {
  PipelineIcon, FolderIcon, FileKindIcon, StatusDot, TrashIcon, PlusIcon, DownloadIcon, DocIcon,
} from "@/pages/project-studio/pipelines/registry/components/editor-icons";
import {
  pipelineNavLastSegment, expandFolderPaths, getDirectChildFolders, peSanitizeSegment, peNormalizeVirtualPath, peEmptyPipelineGraph,
} from "@/pages/project-studio/pipelines/registry/components/registry-helpers";
import { RegistryInstallCatalog } from "@/pages/project-studio/pipelines/registry/components/registry-install-catalog";
import { notifyStudioRepoChanged } from "@/pages/project-studio/components/studio-chrome-bridge";

// Unified pipelines registry + folder / template / doc / pipeline editors (studio).
export default function UnifiedRegistryEditor(input) {
  const editorBase = String(input?.editor_base ?? "");
  const editorType = String(input?.editor_type ?? "folder");
  const isPipeline = editorType === "pipeline";
  const isTemplate = editorType === "template";
  const isDoc = editorType === "doc";
  const isFolder = editorType === "folder";
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};

  // ── Sidebar data ─────────────────────────────────────────────────────────
  const sidebar = input?.sidebar ?? {};
  const scopeHierarchy = Array.isArray(sidebar?.scope_hierarchy) ? sidebar.scope_hierarchy : [];
  const scopeFolders = Array.isArray(sidebar?.scope_folders) ? sidebar.scope_folders : [];
  const sidebarPipelines = Array.isArray(sidebar?.pipelines) ? sidebar.pipelines : [];
  const sidebarTemplateFiles = Array.isArray(sidebar?.template_files) ? sidebar.template_files : [];
  const currentPath = String(sidebar?.scope_path ?? "/");
  const expandedFolders = expandFolderPaths(scopeFolders, editorBase);
  const directChildFolders = getDirectChildFolders(expandedFolders, currentPath);
  const listingChildFolders = Array.isArray(sidebar?.child_folders) ? sidebar.child_folders : [];
  const isRoot = currentPath === "/";
  const SPECIAL = new Set(["assets", "styles", "docs"]);
  const SPECIAL_ORDER = ["docs", "styles", "assets"];
  const normalFolders = directChildFolders.filter(f =>
    !SPECIAL.has(pipelineNavLastSegment(f?.virtual_path ?? "").replace("/", "")));
  // Special folders (assets/, styles/, docs/) come from the listing child_folders — ordered docs→styles→assets
  const specialFolders = isRoot
    ? [...listingChildFolders.filter(f => SPECIAL.has(f?.name ?? ""))].sort(
        (a, b) => SPECIAL_ORDER.indexOf(a?.name) - SPECIAL_ORDER.indexOf(b?.name)
      )
    : [];
  // Physical-only folders: exist on disk but have no pipelines (not in pipeline metadata)
  // and are not the SPECIAL set (docs/styles/assets handled separately at root)
  const directChildNames = new Set(
    directChildFolders.map(f => pipelineNavLastSegment(f?.virtual_path ?? ""))
  );
  const physicalOnlyFolders = listingChildFolders.filter(f => {
    const name = f?.name ?? "";
    return !SPECIAL.has(name) && !directChildNames.has(name);
  });

  function specialFolderEditorClass(name: string) {
    if (name === "assets") return "registry-folder-assets";
    if (name === "styles") return "registry-folder-styles";
    if (name === "docs") return "registry-folder-docs";
    return "";
  }

  // ── Pipeline editor data ──────────────────────────────────────────────────
  const pipeline = input?.pipeline ?? {};
  const editorApi = pipeline?.api ?? {};

  // ── Template editor state ─────────────────────────────────────────────────
  const template = input?.template ?? {};
  const [templateSaveState, setTemplateSaveState] = useState("Saved");
  const templateEditorHostRef = useRef(null);
  const templateEditorViewRef = useRef(null);
  const templateRuntimeRef = useRef(null);

  // ── Doc editor state ──────────────────────────────────────────────────────
  const doc = input?.doc ?? {};
  const [docSaveState, setDocSaveState] = useState("Saved");
  const docEditorHostRef = useRef(null);
  const docEditorViewRef = useRef(null);

  // ── Split pane ────────────────────────────────────────────────────────────
  const pipelineEditorRef = useSplitPane({
    handleSelector: ".pipeline-editor-split-handle",
    variable: "--pipeline-editor-sidebar-width",
    min: 220,
    max: 480,
  });

  // ── Creation dialogs ──────────────────────────────────────────────────────
  const owner = String(input?.owner ?? "");
  const project = String(input?.project ?? "");
  const projectApiBase = `/api/projects/${owner}/${project}`;
  const newPipelineDialogRef = useRef(null);
  const newFileDialogRef = useRef(null);
  const newFolderDialogRef = useRef(null);
  const newDocDialogRef = useRef(null);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState(null as string | null);

  // ── Install catalog state ──────────────────────────────────────────────────
  const nav = useNavigate();
  const [installOpen, setInstallOpen] = useState(false);
  const [catalogData, setCatalogData] = useState([] as any[]);
  const [catalogLoaded, setCatalogLoaded] = useState(false);
  const [selectedComponents, setSelectedComponents] = useState(new Set<string>());
  const [installing, setInstalling] = useState(false);
  const [installResult, setInstallResult] = useState(null as string | null);
  const [installTab, setInstallTab] = useState("ui");

  // ── Helpers ───────────────────────────────────────────────────────────────

  async function requestJson(url, options: any = {}) {
    const response = await fetch(url, {
      headers: {
        Accept: "application/json",
        ...(options.body ? { "Content-Type": "application/json" } : {}),
      },
      ...options,
    });
    if (response.status === 401) { nav("/login"); return null; }
    if (response.status === 204) return null;
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const msg = payload?.error?.message || payload?.message || `${response.status} ${response.statusText}`;
      throw new Error(msg);
    }
    return payload;
  }

  function mountTemplateEditor(content, fileKind, rt) {
    if (templateEditorViewRef.current) {
      templateEditorViewRef.current.destroy();
      templateEditorViewRef.current = null;
    }
    if (!templateEditorHostRef.current) return;
    const { EditorView, basicSetup, javascript, css, autocompletion, linter, lintGutter, oneDark } = rt.cm;
    const extensions = [
      basicSetup,
      oneDark,
      EditorView.theme({ "&": { height: "100%" }, ".cm-scroller": { overflow: "auto" } }),
      autocompletion(),
      linter(() => []),
      lintGutter(),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) return;
        setTemplateSaveState("Unsaved");
      }),
    ];
    if (fileKind === "style") {
      extensions.push(css());
    } else {
      extensions.push(javascript({ jsx: true, typescript: true }));
    }
    templateEditorViewRef.current = new EditorView({
      doc: content,
      extensions,
      parent: templateEditorHostRef.current,
    });
  }

  useEffect(() => {
    if (!isTemplate) return;
    const content = template?.content ?? "";
    const fileKind = template?.file_kind ?? "template";
    setTemplateSaveState("Loading…");
    (async () => {
      try {
        let rt = templateRuntimeRef.current;
        if (!rt) {
          rt = await loadEditorRuntime();
          templateRuntimeRef.current = rt;
        }
        mountTemplateEditor(content, fileKind, rt);
        setTemplateSaveState("Saved");
      } catch (err) {
        setTemplateSaveState("Error");
        console.error("[EDITOR] template init failed", err);
      }
    })();
  }, []);

  async function handleSaveTemplate() {
    if (!templateEditorViewRef.current) return;
    setTemplateSaveState("Saving…");
    try {
      const content = templateEditorViewRef.current.state.doc.toString();
      await requestJson(template?.api?.save ?? "", {
        method: "PUT",
        body: JSON.stringify({ rel_path: template?.rel_path ?? "", content }),
      });
      setTemplateSaveState("Saved");
      notifyStudioRepoChanged();
    } catch (err) {
      setTemplateSaveState("Error");
    }
  }

  useEffect(() => {
    if (!isDoc) return;
    const content = doc?.content ?? "";
    setDocSaveState("Loading…");
    (async () => {
      try {
        let rt = templateRuntimeRef.current;
        if (!rt) {
          rt = await loadEditorRuntime();
          templateRuntimeRef.current = rt;
        }
        if (docEditorViewRef.current) {
          docEditorViewRef.current.destroy();
          docEditorViewRef.current = null;
        }
        if (!docEditorHostRef.current) return;
        const { EditorView, basicSetup, javascript, autocompletion, linter, lintGutter, oneDark } = rt.cm;
        docEditorViewRef.current = new EditorView({
          doc: content,
          extensions: [
            basicSetup,
            oneDark,
            EditorView.theme({ "&": { height: "100%" }, ".cm-scroller": { overflow: "auto" } }),
            autocompletion(),
            linter(() => []),
            lintGutter(),
            EditorView.updateListener.of((update) => {
              if (!update.docChanged) return;
              setDocSaveState("Unsaved");
            }),
            javascript({ jsx: false, typescript: false }),
          ],
          parent: docEditorHostRef.current,
        });
        setDocSaveState("Saved");
      } catch (err) {
        setDocSaveState("Error");
        console.error("[EDITOR] doc init failed", err);
      }
    })();
  }, []);

  async function handleSaveDoc() {
    if (!docEditorViewRef.current) return;
    setDocSaveState("Saving…");
    try {
      const content = docEditorViewRef.current.state.doc.toString();
      const docName = String(doc?.name ?? "");
      await fetch(`${projectApiBase}/docs/file?path=${encodeURIComponent(docName)}`, {
        method: "PUT",
        body: content,
        headers: { "Content-Type": "text/plain" },
      });
      setDocSaveState("Saved");
      notifyStudioRepoChanged();
    } catch (err) {
      setDocSaveState("Error");
    }
  }

  // ── Install handlers ──────────────────────────────────────────────────────

  async function loadCatalog() {
    try {
      const res = await fetch(`${projectApiBase}/install/catalog/ui`, { headers: { Accept: "application/json" } });
      const json = await res.json();
      setCatalogData(json?.components ?? []);
      setCatalogLoaded(true);
    } catch {
      setCatalogData([]);
    }
  }

  async function handleInstallSubmit() {
    const names = Array.from(selectedComponents);
    if (names.length === 0) { setInstallResult("Select at least one component."); return; }
    setInstalling(true);
    setInstallResult(null);
    try {
      const res = await fetch(`${projectApiBase}/install/ui`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ names, overwrite: false }),
      });
      const json = await res.json();
      if (json?.ok) {
        const { installed = [], skipped = [] } = json.report ?? {};
        const parts: string[] = [];
        if (installed.length) parts.push(`Installed: ${installed.join(", ")}`);
        if (skipped.length) parts.push(`Skipped: ${skipped.join(", ")}`);
        setInstallResult(parts.join(" · ") || "Done.");
        if (installed.length > 0) {
          setTimeout(() => {
            setInstallOpen(false);
            nav(`${editorBase}?path=${encodeURIComponent(currentPath)}`);
          }, 1200);
        } else {
          setCatalogLoaded(false);
          loadCatalog();
        }
      } else {
        setInstallResult(`Error: ${json?.error ?? "unknown"}`);
      }
    } catch {
      setInstallResult("Network error.");
    } finally {
      setInstalling(false);
    }
  }

  // ── Create handlers ───────────────────────────────────────────────────────

  async function handleCreatePipeline(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const triggerKind = String(fd.get("trigger_kind") || "webhook");
    const name = peSanitizeSegment(fd.get("name"));
    const virtualPath = peNormalizeVirtualPath(currentPath);
    const title = String(fd.get("title") || "");
    const source = JSON.stringify(peEmptyPipelineGraph(name, triggerKind), null, 2);
    const cleanVp = (virtualPath || "/").replace(/^\//, "");
    const fileRelPath = cleanVp ? `pipelines/${cleanVp}/${name}.zf.json` : `pipelines/${name}.zf.json`;
    setCreating(true);
    setCreateError(null);
    try {
      const payload = await requestJson(`${projectApiBase}/pipelines/definition`, {
        method: "POST",
        body: JSON.stringify({ file_rel_path: fileRelPath, title, description: "", trigger_kind: triggerKind, source }),
      });
      const id = payload?.meta?.file_rel_path;
      if (id) {
        const path = payload?.meta?.virtual_path || virtualPath;
        nav(`${editorBase}?type=pipeline&path=${encodeURIComponent(path)}&file=${encodeURIComponent(id)}`);
      }
      if (newPipelineDialogRef.current) newPipelineDialogRef.current.close();
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    } finally {
      setCreating(false);
    }
  }

  async function handleCreateFile(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const kind = String(fd.get("kind") || "page");
    const name = String(fd.get("name") || "").trim();
    const parentRelPath = currentPath.replace(/^\//, "") || null;
    setCreating(true);
    setCreateError(null);
    try {
      const payload = await requestJson(`${projectApiBase}/templates/create`, {
        method: "POST",
        body: JSON.stringify({ kind, name, parent_rel_path: parentRelPath }),
      });
      const relPath = payload?.rel_path;
      if (relPath) {
        const parts = relPath.split("/");
        const dir = parts.slice(0, -1).join("/");
        nav(`${editorBase}?type=template&path=${encodeURIComponent(dir)}&file=${encodeURIComponent(relPath)}`);
      }
      if (newFileDialogRef.current) newFileDialogRef.current.close();
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    } finally {
      setCreating(false);
    }
  }

  async function handleCreateFolder(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const name = peSanitizeSegment(fd.get("name"));
    const parentRelPath = currentPath.replace(/^\//, "") || null;
    setCreating(true);
    setCreateError(null);
    try {
      await requestJson(`${projectApiBase}/templates/create`, {
        method: "POST",
        body: JSON.stringify({ kind: "folder", name, parent_rel_path: parentRelPath }),
      });
      const newFolderVPath = currentPath === "/" ? `/${name}` : `${currentPath}/${name}`;
      nav(`${editorBase}?path=${encodeURIComponent(newFolderVPath)}`);
      if (newFolderDialogRef.current) newFolderDialogRef.current.close();
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    } finally {
      setCreating(false);
    }
  }

  async function handleCreateDoc(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const rawName = String(fd.get("name") || "").trim().replace(/\.md$/i, "");
    if (!rawName) return;
    const filename = `${rawName}.md`;
    setCreating(true);
    setCreateError(null);
    try {
      await fetch(`${projectApiBase}/docs/file?path=${encodeURIComponent(filename)}`, {
        method: "PUT",
        body: "",
        headers: { "Content-Type": "text/plain" },
      });
      if (newDocDialogRef.current) newDocDialogRef.current.close();
      nav(`${editorBase}?type=doc&path=%2Fdocs&file=docs%2F${encodeURIComponent(filename)}`);
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    } finally {
      setCreating(false);
    }
  }

  // ── Folder view data ─────────────────────────────────────────────────────
  const folder = input?.folder ?? {};
  const folderChildFoldersRaw = Array.isArray(folder?.child_folders) ? folder.child_folders : [];
  const folderNormalFolders = folderChildFoldersRaw.filter(f => !SPECIAL.has(f?.name ?? "")).sort(
    (a, b) => (a?.name ?? "").localeCompare(b?.name ?? "")
  );
  const folderSpecialFolders = folderChildFoldersRaw.filter(f => SPECIAL.has(f?.name ?? "")).sort(
    (a, b) => SPECIAL_ORDER.indexOf(a?.name) - SPECIAL_ORDER.indexOf(b?.name)
  );
  const folderChildFolders = [...folderNormalFolders, ...folderSpecialFolders];
  const folderPipelines = Array.isArray(folder?.pipelines) ? folder.pipelines : [];
  const folderTemplateFiles = Array.isArray(folder?.template_files) ? folder.template_files : [];

  // ── Dynamic listing state (updated on delete without full re-render) ─────
  const [dynFolderPipelines, setDynFolderPipelines] = useState(folderPipelines);
  const [dynFolderTemplates, setDynFolderTemplates] = useState(folderTemplateFiles);
  const [dynFolderNormalFolders, setDynFolderNormalFolders] = useState(folderNormalFolders);
  const [dynFolderSpecialFolders, setDynFolderSpecialFolders] = useState(folderSpecialFolders);
  const [dynSidebarPipelines, setDynSidebarPipelines] = useState(sidebarPipelines);
  const [dynSidebarTemplates, setDynSidebarTemplates] = useState(sidebarTemplateFiles);

  // ── Delete state (folder view) ───────────────────────────────────────────
  const [pendingDelete, setPendingDelete] = useState(null as any);
  const [deleteInput, setDeleteInput] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState(null as any);

  async function handleDeleteConfirm() {
    if (!pendingDelete) return;
    const owner = input?.owner ?? "";
    const project = input?.project ?? "";
    setDeleting(true);
    setDeleteError(null);
    try {
      let resp;
      if (pendingDelete.isPipeline) {
        resp = await fetch(`/api/projects/${owner}/${project}/pipelines/definition`, {
          method: "DELETE",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ file_rel_path: pendingDelete.path }),
        });
      } else if (pendingDelete.isDoc) {
        // Doc files live in repo/docs — use the dedicated docs endpoint.
        // pendingDelete.path is "docs/foo.md"; strip the "docs/" prefix.
        const docPath = pendingDelete.path.replace(/^docs\//, "");
        resp = await fetch(`/api/projects/${owner}/${project}/docs/file?path=${encodeURIComponent(docPath)}`, {
          method: "DELETE",
        });
      } else {
        resp = await fetch(`/api/projects/${owner}/${project}/templates/file?path=${encodeURIComponent(pendingDelete.path)}`, {
          method: "DELETE",
        });
      }
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        setDeleteError(data?.error ?? `Delete failed: ${resp.status}`);
        setDeleting(false);
        return;
      }
      setPendingDelete(null);
      setDeleteInput("");
      setDeleting(false);

      if (pendingDelete.isFolder) {
        nav(`${editorBase}?path=${encodeURIComponent(pendingDelete.parentPath ?? "/")}`);
      } else if (pendingDelete.isPipeline) {
        setDynFolderPipelines(p => p.filter(x => (x as any).id !== pendingDelete.path));
        setDynSidebarPipelines(p => p.filter(x => (x as any).id !== pendingDelete.path));
        if (isPipeline && (pipeline?.selected_id ?? "") === pendingDelete.path) {
          nav(`${editorBase}?path=${encodeURIComponent(currentPath)}`);
        }
      } else {
        setDynFolderTemplates(p => p.filter(x => (x as any).rel_path !== pendingDelete.path));
        setDynSidebarTemplates(p => p.filter(x => (x as any).rel_path !== pendingDelete.path));
        if (isTemplate) {
          nav(`${editorBase}?path=${encodeURIComponent(currentPath)}`);
        } else if (isDoc) {
          nav(`${editorBase}?path=${encodeURIComponent("/docs")}`);
        }
      }
    } catch (err) {
      setDeleteError(err?.message ?? "Network error");
      setDeleting(false);
    }
  }

  // ── Render ────────────────────────────────────────────────────────────────
  // No `<Page>` wrapper: `rewrite_page_root_tag` in RWE runs only on the route entry file, not on inlined imports (would leave `Page` undefined).
  return (
    <ProjectStudioShell
      projectHref={input?.project_href}
      projectLabel={input?.title}
      currentMenu={input?.current_menu}
      owner={input?.owner}
      project={input?.project}
      nav={input?.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <nav className="shrink-0 flex items-stretch border-b border-[var(--studio-border)] bg-[var(--studio-panel)] px-[0.625rem]">
          <Link href={navLinks.pipelines_registry ?? "#"} className={cx("project-tab-link", navClasses.pipeline_registry)}>Registry</Link>
          <Link href={navLinks.pipelines_webhooks ?? "#"} className={cx("project-tab-link", navClasses.pipeline_webhooks)}>Webhooks</Link>
          <Link href={navLinks.pipelines_schedules ?? "#"} className={cx("project-tab-link", navClasses.pipeline_schedules)}>Schedules</Link>
          <Link href={navLinks.pipelines_manual ?? "#"} className={cx("project-tab-link", navClasses.pipeline_manual)}>Manual</Link>
          <Link href={navLinks.pipelines_functions ?? "#"} className={cx("project-tab-link", navClasses.pipeline_functions)}>Functions</Link>
        </nav>
        <div
          ref={pipelineEditorRef}
          className="pipeline-editor-shell"
          data-pipeline-registry="true"
          data-owner={input?.owner ?? ""}
          data-project={input?.project ?? ""}
        >
          {/* ── Sidebar ──────────────────────────────────────────────── */}
          <aside className="pipeline-editor-sidebar">
            <div className="pipeline-editor-sidebar-head">
              <p className="pipeline-editor-title">Editor</p>
              <div className="flex items-center gap-1">
                <DropdownMenu
                  trigger={<Button size="sm" variant="outline" className="flex items-center gap-1.5"><PlusIcon />New</Button>}
                  align="right"
                >
                  <DropdownMenuItem
                    label="Pipeline"
                    onClick={() => {
                      setCreateError(null);
                      if (newPipelineDialogRef.current) newPipelineDialogRef.current.showModal();
                    }}
                  />
                  <DropdownMenuItem
                    label="Template file"
                    onClick={() => {
                      setCreateError(null);
                      if (newFileDialogRef.current) newFileDialogRef.current.showModal();
                    }}
                  />
                  <DropdownMenuItem
                    label="Folder"
                    onClick={() => {
                      setCreateError(null);
                      if (newFolderDialogRef.current) newFolderDialogRef.current.showModal();
                    }}
                  />
                  <DropdownMenuItem
                    label="Documentation"
                    onClick={() => {
                      setCreateError(null);
                      if (newDocDialogRef.current) newDocDialogRef.current.showModal();
                    }}
                  />
                </DropdownMenu>
                <Button size="sm" variant="ghost"
                  onClick={() => { setInstallResult(null); setInstallOpen(true); if (!catalogLoaded) loadCatalog(); }}
                  title="Install UI components"
                  className="flex items-center gap-1.5">
                  <DownloadIcon />Install
                </Button>
              </div>
            </div>

            {/* Scrollable sidebar body — folder nav + pipelines + templates always together */}
            <div className="pipeline-editor-sidebar-body">
              {/* Folder breadcrumbs + child folders */}
              <div className="pipeline-editor-folder-nav">
                <div className="pipeline-editor-folder-crumbs">
                  {scopeHierarchy.map((seg, index) => (
                    <span key={`crumb-${index}`} className="pipeline-editor-folder-crumb">
                      {index > 0 ? <span className="pipeline-editor-crumb-sep">/</span> : null}
                      <Link href={seg?.href ?? "#"} className="pipeline-editor-crumb-link">{seg?.name}</Link>
                    </span>
                  ))}
                </div>
                {normalFolders.map((folder, index) => (
                  <Link
                    key={`child-folder-${index}`}
                    href={folder?.href ?? "#"}
                    className="pipeline-editor-nav-row"
                  >
                    <svg viewBox="0 0 24 24" fill="none" className="pipeline-editor-nav-icon" aria-hidden="true">
                      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                    </svg>
                    <span className="pipeline-editor-nav-label">{pipelineNavLastSegment(folder?.virtual_path)}/</span>
                    <span className="pipeline-editor-nav-count">{folder?.count ?? 0}</span>
                  </Link>
                ))}
                {physicalOnlyFolders.map((folder, index) => (
                  <Link
                    key={`physical-folder-${index}`}
                    href={folder?.href ?? "#"}
                    className="pipeline-editor-nav-row"
                  >
                    <svg viewBox="0 0 24 24" fill="none" className="pipeline-editor-nav-icon" aria-hidden="true">
                      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                    </svg>
                    <span className="pipeline-editor-nav-label">{folder?.name}/</span>
                    <span className="pipeline-editor-nav-count">{folder?.count ?? 0}</span>
                  </Link>
                ))}
                {isRoot && specialFolders.length > 0 && (
                  <>
                    <div className="pipeline-editor-section-sep" />
                    {specialFolders.map((folder, index) => (
                      <Link
                        key={`special-folder-${index}`}
                        href={folder?.href ?? "#"}
                        className={cx("pipeline-editor-nav-row", specialFolderEditorClass(folder?.name ?? ""))}
                      >
                        <svg viewBox="0 0 24 24" fill="none" className="pipeline-editor-nav-icon" aria-hidden="true">
                          <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                        </svg>
                        <span className="pipeline-editor-nav-label">{pipelineNavLastSegment(folder?.virtual_path)}/</span>
                        <span className="pipeline-editor-nav-count">{folder?.count ?? 0}</span>
                      </Link>
                    ))}
                  </>
                )}
              </div>

              {/* Pipelines list */}
              <div data-editor-pipeline-list="true">
                {dynSidebarPipelines.map((item, index) => (
                  <div key={`${item?.id ?? "p"}-${index}`} className="pipeline-editor-item-wrap">
                    <Link
                      href={item?.editor_href ?? "#"}
                      className={cx("pipeline-editor-item", item?.is_selected ? "is-selected" : "")}
                      data-editor-pipeline-id={item?.id ?? ""}
                    >
                      <div className="pipeline-editor-item-head">
                        <div className="flex items-center gap-1.5">
                          <PipelineIcon className="w-3.5 h-3.5 text-[var(--studio-accent)]" />
                          <StatusDot isActive={item?.is_active} hasDraft={item?.has_draft} />
                          <span className="pipeline-editor-item-name">{item?.name}</span>
                        </div>
                        <span className="pipeline-editor-item-status">{item?.status_label}</span>
                      </div>
                      <p className="pipeline-editor-item-meta">{item?.trigger_kind}</p>
                    </Link>
                    <button
                      type="button"
                      className="pipeline-registry-row-del"
                      title={`Delete ${item?.name ?? "pipeline"}`}
                      onClick={() => { setPendingDelete({ path: item?.id ?? "", name: item?.name ?? "", isPipeline: true }); setDeleteInput(""); setDeleteError(null); }}
                    >
                      <TrashIcon />
                    </button>
                  </div>
                ))}
              </div>

              {/* Templates section */}
              {dynSidebarTemplates.length > 0 && (
                <>
                  <div className="pipeline-editor-section-head">Templates</div>
                  <div>
                    {dynSidebarTemplates.map((file, index) => (
                      <div key={`tpl-${file?.template_path ?? index}`} className="pipeline-editor-item-wrap">
                        <Link
                          href={file?.editor_href ?? "#"}
                          className={cx("pipeline-editor-item", file?.is_selected ? "is-selected" : "")}
                        >
                          <div className="pipeline-editor-item-head">
                            <div className="flex items-center gap-1.5">
                              <FileKindIcon name={file?.name ?? ""} />
                              <span className="pipeline-editor-item-name">{file?.name}</span>
                            </div>
                            {file?.git_status ? (
                              <span className="pipeline-editor-item-status pipeline-editor-item-git">{file.git_status}</span>
                            ) : null}
                          </div>
                          <p className="pipeline-editor-item-meta">{file?.kind}</p>
                        </Link>
                        <button
                          type="button"
                          className="pipeline-registry-row-del"
                          title={`Delete ${file?.name ?? "file"}`}
                          onClick={() => { setPendingDelete({ path: file?.rel_path ?? "", name: file?.name ?? "", isPipeline: false, isDoc: file?.kind === "doc" }); setDeleteInput(""); setDeleteError(null); }}
                        >
                          <TrashIcon />
                        </button>
                      </div>
                    ))}
                  </div>
                </>
              )}
            </div>
          </aside>

          {/* ── Split handle ─────────────────────────────────────────── */}
          <div className="pipeline-editor-split-handle" aria-hidden="true"></div>

          {/* ── Main pane ────────────────────────────────────────────── */}
          <section className="pipeline-editor-main">

            {/* ── Folder view ─────────────────────────────────────────── */}
            {isFolder && (
              <>
              <div className="flex flex-col flex-1 min-h-0 overflow-auto">
                <div className="pipeline-editor-toolbar">
                  <div className="pipeline-editor-toolbar-main">
                    <p className="pipeline-editor-title">{currentPath === "/" ? "Root" : pipelineNavLastSegment(currentPath)}</p>
                    <p className="pipeline-editor-subtitle">{currentPath}</p>
                  </div>
                </div>
                <div className="flex flex-col py-3 px-3 gap-1">

                  {/* Sub-folders — normal first, then special (docs/styles/assets) */}
                  {(dynFolderNormalFolders.length + dynFolderSpecialFolders.length) > 0 ? (
                    <div className="pipeline-registry-section-head">Folders</div>
                  ) : null}
                  {dynFolderNormalFolders.map((f, index) => {
                    const folderRelPath = (f?.virtual_path ?? "").replace(/^\//, "");
                    return (
                      <div
                        key={`ffolder-${index}`}
                        className="pipeline-registry-row pipeline-registry-folder-row"
                      >
                        <Link href={f?.href ?? "#"} className="pipeline-registry-row-link">
                          <span className="shrink-0 flex items-center text-[var(--studio-text-soft)]"><FolderIcon /></span>
                          <span className="pipeline-registry-row-name">{f?.name}/</span>
                        </Link>
                        <button
                          type="button"
                          className="pipeline-registry-row-del"
                          title={`Delete folder ${f?.name}`}
                          onClick={() => { setPendingDelete({ path: folderRelPath, name: f?.name ?? "folder", isPipeline: false, isFolder: true, parentPath: currentPath }); setDeleteInput(""); setDeleteError(null); }}
                        >
                          <TrashIcon />
                        </button>
                      </div>
                    );
                  })}
                  {dynFolderSpecialFolders.length > 0 ? (
                    <div className="pipeline-registry-special-sep" aria-hidden="true" />
                  ) : null}
                  {dynFolderSpecialFolders.map((f, index) => (
                    <Link
                      key={`fspecial-${index}`}
                      href={f?.href ?? "#"}
                      className={cx("pipeline-registry-row pipeline-registry-folder-row pipeline-registry-special-folder", specialFolderEditorClass(f?.name ?? ""))}
                    >
                      <span className="shrink-0 flex items-center text-[var(--studio-text-soft)]"><FolderIcon /></span>
                      <span className="pipeline-registry-row-name">{f?.name}/</span>
                    </Link>
                  ))}

                  {/* Pipelines */}
                  {dynFolderPipelines.length > 0 ? (
                    <div className="pipeline-registry-section-head">Pipelines</div>
                  ) : null}
                  {dynFolderPipelines.map((item, index) => (
                    <div
                      key={`fpipeline-${index}`}
                      className="pipeline-registry-row"
                      data-pipeline-row=""
                      data-rel-path={item?.id ?? ""}
                    >
                      <Link href={item?.editor_href ?? "#"} className="pipeline-registry-row-link">
                        <span className="shrink-0 flex items-center text-[var(--studio-text-soft)]"><PipelineIcon /></span>
                        <StatusDot isActive={item?.is_active} hasDraft={item?.has_draft} />
                        <span className="pipeline-registry-row-name">{item?.title || item?.name}</span>
                        <Badge variant="secondary">{item?.trigger_kind}</Badge>
                      </Link>
                      <button
                        type="button"
                        className="pipeline-registry-row-del"
                        title={`Delete ${item?.name ?? "pipeline"}`}
                        onClick={() => { setPendingDelete({ path: item?.id ?? "", name: item?.name ?? "", isPipeline: true }); setDeleteInput(""); setDeleteError(null); }}
                      >
                        <TrashIcon />
                      </button>
                    </div>
                  ))}

                  {/* Template files */}
                  {dynFolderTemplates.length > 0 ? (
                    <div className="pipeline-registry-section-head">Templates</div>
                  ) : null}
                  {dynFolderTemplates.map((file, index) => (
                    <div
                      key={`ffile-${index}`}
                      className="pipeline-registry-row pipeline-registry-file-row"
                      data-pipeline-row=""
                      data-rel-path={file?.rel_path ?? ""}
                    >
                      <Link href={file?.editor_href ?? "#"} className="pipeline-registry-row-link">
                        <span className="shrink-0 flex items-center text-[var(--studio-text-soft)]"><FileKindIcon name={file?.name ?? ""} /></span>
                        <span className="pipeline-registry-row-name">{file?.name}</span>
                      </Link>
                      <button
                        type="button"
                        className="pipeline-registry-row-del"
                        title={`Delete ${file?.name ?? "file"}`}
                        onClick={() => { setPendingDelete({ path: file?.rel_path ?? "", name: file?.name ?? "", isPipeline: false, isDoc: file?.kind === "doc" }); setDeleteInput(""); setDeleteError(null); }}
                      >
                        <TrashIcon />
                      </button>
                    </div>
                  ))}

                  {(dynFolderNormalFolders.length + dynFolderSpecialFolders.length) === 0 && dynFolderPipelines.length === 0 && dynFolderTemplates.length === 0 ? (
                    <p className="p-6 text-center text-[0.78rem] text-[var(--studio-text-soft)]">Empty folder. Use <strong>+ New</strong> to add pipelines.</p>
                  ) : null}
                </div>
              </div>

              </>
            )}

            {/* ── Template editor ─────────────────────────────────────── */}
            {isTemplate && (
              <div className="flex flex-col flex-1 min-h-0">
                <div className="pipeline-editor-toolbar">
                  <div className="pipeline-editor-toolbar-main">
                    <p className="pipeline-editor-title">{template?.name}</p>
                    <p className="pipeline-editor-subtitle">{template?.rel_path}</p>
                  </div>
                  <div className="pipeline-editor-toolbar-actions">
                    <span className="pipeline-editor-indicator">{templateSaveState}</span>
                    <span className="pipeline-editor-indicator">{template?.file_kind}</span>
                    <Button variant="outline" size="xs" onClick={handleSaveTemplate}>Save</Button>
                    <Button
                      variant="destructive"
                      size="xs"
                      onClick={() => {
                        setPendingDelete({ path: template?.rel_path ?? "", name: template?.name ?? "", isPipeline: false, isDoc: false });
                        setDeleteInput("");
                        setDeleteError(null);
                      }}
                    >Delete</Button>
                    <Link href={`${editorBase}?path=${currentPath}`} className="zf-btn zf-btn-ghost zf-btn-xs">✕ Close</Link>
                  </div>
                </div>
                <div className="pipeline-editor-template-host" ref={templateEditorHostRef} />
                <div className="pipeline-editor-foot">
                  <span className="pipeline-editor-foot-item">{template?.name}</span>
                  <span className="pipeline-editor-foot-item">{templateSaveState}</span>
                  <span className="pipeline-editor-foot-item">zeb/codemirror@0.1</span>
                </div>
              </div>
            )}

            {/* ── Doc editor ──────────────────────────────────────────── */}
            {isDoc && (
              <div className="flex flex-col flex-1 min-h-0">
                <div className="pipeline-editor-toolbar">
                  <div className="pipeline-editor-toolbar-main">
                    <p className="pipeline-editor-title">{doc?.name}</p>
                    <p className="pipeline-editor-subtitle">{doc?.rel_path}</p>
                  </div>
                  <div className="pipeline-editor-toolbar-actions">
                    <span className="pipeline-editor-indicator">{docSaveState}</span>
                    <span className="pipeline-editor-indicator">doc</span>
                    <Button variant="outline" size="xs" onClick={handleSaveDoc}>Save</Button>
                    <Button
                      variant="destructive"
                      size="xs"
                      onClick={() => {
                        setPendingDelete({ path: doc?.rel_path ?? "", name: doc?.name ?? "", isPipeline: false, isDoc: true });
                        setDeleteInput("");
                        setDeleteError(null);
                      }}
                    >Delete</Button>
                    <Link href={`${editorBase}?path=/docs`} className="zf-btn zf-btn-ghost zf-btn-xs">✕ Close</Link>
                  </div>
                </div>
                <div className="pipeline-editor-template-host" ref={docEditorHostRef} />
                <div className="pipeline-editor-foot">
                  <span className="pipeline-editor-foot-item">{doc?.name}</span>
                  <span className="pipeline-editor-foot-item">{docSaveState}</span>
                </div>
              </div>
            )}

            {/* ── Pipeline editor ──────────────────────────────────────── */}
            {isPipeline && (
              <PipelineEditor
                api={{
                  byId: editorApi?.by_id ?? "",
                  definition: editorApi?.definition ?? "",
                  activate: editorApi?.activate ?? "",
                  deactivate: editorApi?.deactivate ?? "",
                  hits: editorApi?.hits ?? "",
                  invocations: editorApi?.invocations ?? "",
                  nodes: editorApi?.nodes ?? "",
                  credentials: editorApi?.credentials ?? "",
                  templatesWorkspace: editorApi?.templates_workspace ?? "",
                  templateFile: editorApi?.template_file ?? "",
                  templateSave: editorApi?.template_save ?? "",
                }}
                selectedId={pipeline?.selected_id ?? ""}
                owner={owner}
                project={project}
                scopePath={currentPath}
                graphuiSrc={pipeline?.graphui?.runtime_src ?? ""}
                graphuiPackageLabel={pipeline?.graphui?.package_label ?? "Graph UI"}
                onDeleteClick={() => {
                  const pName = String(pipeline?.selected_meta?.name
                    ?? (pipeline?.selected_id ?? "").split("/").pop()?.replace(".zf.json", "")
                    ?? "");
                  setPendingDelete({ path: pipeline?.selected_id ?? "", name: pName, isPipeline: true });
                  setDeleteInput("");
                  setDeleteError(null);
                }}
              />
            )}

            {/* ── No-selection placeholder (folder mode with no folder content) ── */}
            {!isPipeline && !isTemplate && !isDoc && !isFolder && (
              <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-[var(--studio-muted)]">
                <p className="text-sm font-medium text-[var(--studio-text)]">Select a file to edit</p>
              </div>
            )}
          </section>

          {/* ── Delete confirm dialog (global — works from any view) ──────── */}
          {pendingDelete && (
            <div className="pipeline-delete-overlay">
              <div className="pipeline-delete-backdrop" onClick={() => { setPendingDelete(null); setDeleteInput(""); }} />
              <div className="pipeline-delete-box">
                <p className="pipeline-delete-title">Delete <strong>{pendingDelete.name}</strong>?</p>
                <p className="pipeline-delete-warn">This action cannot be undone. Type the name to confirm.</p>
                <input
                  type="text"
                  className="pipeline-delete-input"
                  placeholder={pendingDelete.name}
                  value={deleteInput}
                  onInput={(e) => setDeleteInput(e.currentTarget.value)}
                />
                {deleteError ? <p className="pipeline-delete-error">{deleteError}</p> : null}
                <div className="pipeline-delete-actions">
                  <button
                    type="button"
                    className="zf-btn zf-btn-destructive zf-btn-sm"
                    disabled={deleteInput.trim() !== pendingDelete.name || deleting}
                    onClick={handleDeleteConfirm}
                  >
                    {deleting ? "Deleting…" : "Delete"}
                  </button>
                  <button
                    type="button"
                    className="zf-btn zf-btn-ghost zf-btn-sm"
                    onClick={() => { setPendingDelete(null); setDeleteInput(""); }}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* ── New pipeline dialog (Preact-managed, always rendered) ──── */}
          <dialog ref={newPipelineDialogRef} className="pipeline-editor-dialog">
            <form className="pipeline-editor-dialog-form" onSubmit={handleCreatePipeline}>
              <h3 className="pipeline-editor-dialog-title">Create Pipeline</h3>
              <label className="pipeline-editor-field">
                <span>Trigger</span>
                <Select name="trigger_kind" required>
                  <SelectOption value="webhook" label="Webhook" />
                  <SelectOption value="schedule" label="Schedule" />
                  <SelectOption value="manual" label="Manual" />
                  <SelectOption value="function" label="Function" />
                </Select>
              </label>
              <label className="pipeline-editor-field">
                <span>Name</span>
                <Input name="name" type="text" placeholder="my-pipeline" required />
              </label>
              <label className="pipeline-editor-field">
                <span>Title</span>
                <Input name="title" type="text" placeholder="My Pipeline" />
              </label>
              {createError ? <p className="pipeline-editor-dialog-error">{createError}</p> : null}
              <div className="pipeline-editor-dialog-actions">
                <Button variant="outline" size="xs" type="button" onClick={() => { if (newPipelineDialogRef.current) newPipelineDialogRef.current.close(); }}>Cancel</Button>
                <Button size="xs" type="submit" disabled={creating}>{creating ? "Creating…" : "Create"}</Button>
              </div>
            </form>
          </dialog>

          {/* ── New template file dialog ─────────────────────────────────── */}
          <dialog ref={newFileDialogRef} className="pipeline-editor-dialog">
            <form className="pipeline-editor-dialog-form" onSubmit={handleCreateFile}>
              <h3 className="pipeline-editor-dialog-title">New Template File</h3>
              <label className="pipeline-editor-field">
                <span>Kind</span>
                <Select name="kind">
                  <SelectOption value="page" label="Page (pages/)" />
                  <SelectOption value="component" label="Component (components/)" />
                  <SelectOption value="script" label="Script (scripts/)" />
                </Select>
              </label>
              <label className="pipeline-editor-field">
                <span>Name</span>
                <Input name="name" type="text" placeholder="my-page" required />
              </label>
              <label className="pipeline-editor-field">
                <span>Parent folder</span>
                <Input name="parent_display" type="text" value={currentPath.replace(/^\//, "") || "/"} readOnly />
              </label>
              {createError ? <p className="pipeline-editor-dialog-error">{createError}</p> : null}
              <div className="pipeline-editor-dialog-actions">
                <Button variant="outline" size="xs" type="button" onClick={() => { if (newFileDialogRef.current) newFileDialogRef.current.close(); }}>Cancel</Button>
                <Button size="xs" type="submit" disabled={creating}>{creating ? "Creating…" : "Create"}</Button>
              </div>
            </form>
          </dialog>

          {/* ── New folder dialog ─────────────────────────────────────────── */}
          <dialog ref={newFolderDialogRef} className="pipeline-editor-dialog">
            <form className="pipeline-editor-dialog-form" onSubmit={handleCreateFolder}>
              <h3 className="pipeline-editor-dialog-title">New Folder</h3>
              <label className="pipeline-editor-field">
                <span>Folder name</span>
                <Input name="name" type="text" placeholder="blog" required />
              </label>
              <label className="pipeline-editor-field">
                <span>Parent path</span>
                <Input name="parent_display" type="text" value={currentPath} readOnly />
              </label>
              {createError ? <p className="pipeline-editor-dialog-error">{createError}</p> : null}
              <div className="pipeline-editor-dialog-actions">
                <Button variant="outline" size="xs" type="button" onClick={() => { if (newFolderDialogRef.current) newFolderDialogRef.current.close(); }}>Cancel</Button>
                <Button size="xs" type="submit" disabled={creating}>{creating ? "Creating…" : "Create"}</Button>
              </div>
            </form>
          </dialog>

          {/* ── New documentation dialog ─────────────────────────────────── */}
          <dialog ref={newDocDialogRef} className="pipeline-editor-dialog">
            <form className="pipeline-editor-dialog-form" onSubmit={handleCreateDoc}>
              <h3 className="pipeline-editor-dialog-title">New Documentation</h3>
              <label className="pipeline-editor-field">
                <span>File name</span>
                <Input name="name" type="text" placeholder="guide" required />
                <small className="pipeline-editor-field-help">Saved as <code>docs/{"{name}"}.md</code></small>
              </label>
              {createError ? <p className="pipeline-editor-dialog-error">{createError}</p> : null}
              <div className="pipeline-editor-dialog-actions">
                <Button variant="outline" size="xs" type="button" onClick={() => { if (newDocDialogRef.current) newDocDialogRef.current.close(); }}>Cancel</Button>
                <Button size="xs" type="submit" disabled={creating}>{creating ? "Creating…" : "Create"}</Button>
              </div>
            </form>
          </dialog>

          {installOpen && (
            <RegistryInstallCatalog
              onClose={() => setInstallOpen(false)}
              installTab={installTab}
              setInstallTab={setInstallTab}
              catalogData={catalogData}
              selectedComponents={selectedComponents}
              setSelectedComponents={setSelectedComponents}
              installResult={installResult}
              installing={installing}
              onInstallSubmit={handleInstallSubmit}
            />
          )}

        </div>
      </div>
    </ProjectStudioShell>
  );
}
