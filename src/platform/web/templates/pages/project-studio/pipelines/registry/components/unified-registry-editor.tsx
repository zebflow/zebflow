import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { loadEditorRuntime } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/template-editor-runtime";
import { cx, Link, useEffect, useState, useRef, useRouter } from "zeb/react";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import { useSplitPane } from "zeb/use";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";
import Input from "@/components/ui/input";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import PipelineEditor from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/index";
import { Select, SelectOption } from "@/components/ui/select";
import DropdownMenu from "@/components/ui/dropdown-menu";
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";

import { PipelineIcon, FolderIcon, StatusDot, TrashIcon, PlusIcon, DownloadIcon } from "@/pages/project-studio/pipelines/registry/components/editor-icons";
import { LockIcon, LockOpenIcon } from "@/pages/project-studio/components/icons";
import {
  pipelineNavLastSegment, expandFolderPaths, getDirectChildFolders, peSanitizeSegment, peNormalizeVirtualPath, peEmptyPipelineDocument,
} from "@/pages/project-studio/pipelines/registry/components/registry-helpers";
import { RegistryInstallCatalog } from "@/pages/project-studio/pipelines/registry/components/registry-install-catalog";
import { notifyStudioRepoChanged } from "@/pages/project-studio/components/studio-chrome-bridge";
import { subscribeEditorPreferences } from "@/pages/project-studio/components/editor-preferences";
import { loadEditorCompletionCatalog, refreshEditorCompletionCatalog } from "@/pages/project-studio/components/editor-catalog";
import ScriptPromptDialog from "@/pages/project-studio/pipelines/registry/components/script-prompt-dialog";
import AssetManager from "@/pages/project-studio/pipelines/registry/components/asset-manager";
import SidebarSearchButton from "@/pages/project-studio/pipelines/registry/components/sidebar-search-button";
import FileKindIcon from "@/components/ui/file-kind-icon";
import RepoTree from "@/components/ui/repo-tree";
import { navigate } from "@/pages/project-studio/components/studio-shell-behavior";
import { parentFolderOf } from "@/components/lib/repo-tree";

export default function UnifiedRegistryEditor(input) {
  const editorBase = String(input?.editor_base ?? "");
  const editorType = String(input?.editor_type ?? "folder");
  const selectedLine = Number(input?.selected_line ?? 0);
  const isPipeline = editorType === "pipeline";
  // One editor for every repository file. `template` and `doc` were the same
  // thing reached by two roads.
  const isFile = editorType === "file";
  const isFolder = editorType === "folder";
  const assetsApi = String(input?.assets?.api ?? "");
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};

  // ── Sidebar data ─────────────────────────────────────────────────────────
  const sidebar = input?.sidebar ?? {};
  const scopeHierarchy = Array.isArray(sidebar?.scope_hierarchy) ? sidebar.scope_hierarchy : [];
  const scopeFolders = Array.isArray(sidebar?.scope_folders) ? sidebar.scope_folders : [];
  const sidebarPipelines = Array.isArray(sidebar?.pipelines) ? sidebar.pipelines : [];
  const sidebarTemplateFiles = Array.isArray(sidebar?.template_files) ? sidebar.template_files : [];
  const scopePath = String(sidebar?.scope_path ?? "/");
  // Where a create lands. The scope this page was opened at, unless the reader
  // right-clicked a different folder in the tree and said "here".
  const [createTarget, setCreateTarget] = useState(null as string | null);
  const currentPath = createTarget ?? scopePath;
  // What the tree shows as current: the file this editor has open, unless the
  // reader has since picked a folder to work in.
  const editingRelPath = String(
    input?.template?.rel_path ?? input?.pipeline?.file_rel_path ?? "",
  );
  const [pickedFolder, setPickedFolder] = useState(null as string | null);
  const treeSelection = pickedFolder ?? editingRelPath;
  // Bumped after a write so the tree re-reads the folders already on screen.
  const [treeRefreshToken, setTreeRefreshToken] = useState(0);
  const [renaming, setRenaming] = useState(null as any);
  const [renameInput, setRenameInput] = useState("");

  /**
   * A row in the tree was clicked.
   *
   * A file opens. A folder — or the repository root — becomes the place the
   * next thing is made, which is what the New button and the row menus mean by
   * "here".
   */
  function handleTreeSelect(item: any) {
    if (item?.kind === "folder" || item?.kind === "root") {
      const folder = String(item.rel_path || "");
      setPickedFolder(folder);
      setCreateTarget(folder ? `/${folder}` : "/");
      return;
    }
    const relPath = String(item?.rel_path ?? item ?? "");
    if (!relPath) return;
    setPickedFolder(null);
    const parent = parentFolderOf(relPath);
    const type = relPath.endsWith(".zf.json") ? "pipeline" : "file";
    navigate(
      `${editorBase}?type=${type}&path=${encodeURIComponent(parent ? `/${parent}` : "/")}&file=${encodeURIComponent(relPath)}`,
    );
  }

  /** Copies a file beside itself, so the reader can start from what is there. */
  async function duplicateRepoFile(relPath: string) {
    const dot = relPath.lastIndexOf(".");
    const copy = dot > 0 ? `${relPath.slice(0, dot)}-copy${relPath.slice(dot)}` : `${relPath}-copy`;
    try {
      const res = await fetch(
        `${projectApiBase}/repo/file?path=${encodeURIComponent(relPath)}`,
      );
      const body = await res.text();
      const write = await fetch(`${projectApiBase}/repo/file?path=${encodeURIComponent(copy)}`, {
        method: "PUT",
        headers: { "Content-Type": "text/plain" },
        body,
      });
      if (!write.ok) throw new Error(`Duplicate failed: ${write.status}`);
      setTreeRefreshToken((n) => n + 1);
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    }
  }

  /**
   * A row menu chose something to do to that row.
   *
   * Creating happens where the reader pointed, so the target is set before the
   * dialog opens and cleared when it closes — otherwise the next create from
   * the toolbar would silently inherit it.
   */
  function handleTreeAction(kind: string, item: any) {
    const relPath = String(item?.rel_path ?? "");
    const isFolderish = item?.kind === "folder" || item?.kind === "root";
    const folder = isFolderish ? relPath : parentFolderOf(relPath);
    setCreateError(null);

    if (kind === "delete") {
      setPendingDelete({
        path: relPath,
        name: item?.name ?? "",
        isPipeline: relPath.endsWith(".zf.json"),
        isFolder: item?.kind === "folder",
        parentPath: `/${parentFolderOf(relPath)}`,
      });
      setDeleteInput("");
      return;
    }
    if (kind === "rename") {
      setRenaming(item);
      setRenameInput(String(item?.name ?? ""));
      return;
    }
    if (kind === "duplicate") {
      duplicateRepoFile(relPath);
      return;
    }

    if (kind === "open-folder") {
      // The panel beside the tree already lists a folder — its folders, its
      // pipelines and its files — which is exactly what the root shows. Opening
      // a folder is that same view, pointed one level down.
      navigate(`${editorBase}?path=${encodeURIComponent(folder ? `/${folder}` : "/")}`);
      return;
    }

    setCreateTarget(folder ? `/${folder}` : "/");
    setPickedFolder(folder);

    if (kind === "add") {
      // The hub install reads the same target, so what lands, lands here.
      setInstallResult(null);
      setInstallOpen(true);
      if (!catalogLoaded) loadCatalog();
      return;
    }
    if (kind === "new-file" && newFileDialogRef.current) newFileDialogRef.current.showModal();
    if (kind === "new-folder" && newFolderDialogRef.current) newFolderDialogRef.current.showModal();
    if (kind === "new-pipeline" && newPipelineDialogRef.current) newPipelineDialogRef.current.showModal();
  }

  /** Renames in place: a move inside the same parent. */
  async function confirmRename() {
    const from = String(renaming?.rel_path ?? "");
    const name = renameInput.trim();
    if (!from || !name) return;
    const parent = parentFolderOf(from);
    const to = parent ? `${parent}/${name}` : name;
    try {
      await requestJson(`${projectApiBase}/repo/move`, {
        method: "POST",
        body: JSON.stringify({ from_path: from, to_path: to }),
      });
      setRenaming(null);
      setTreeRefreshToken((n) => n + 1);
      if (from === editingRelPath) handleTreeSelect({ rel_path: to, kind: "file" });
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    }
  }

  const isAssets = isFolder && (scopePath === "/static" || scopePath.startsWith("/static/"));
  const assetsSubfolder = scopePath.startsWith("/static/") ? scopePath.slice("/static/".length) : "";
  const expandedFolders = expandFolderPaths(scopeFolders, editorBase);
  const directChildFolders = getDirectChildFolders(expandedFolders, scopePath);
  const listingChildFolders = Array.isArray(sidebar?.child_folders) ? sidebar.child_folders : [];
  const isRoot = scopePath === "/";
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



  /// The destination path for a move: the chosen parent folder plus the
  /// entry's own name. Both sides are repository-relative, so nothing has to be
  /// un-prefixed on the way in or out.
  function joinRepoPath(parent: string, fromPath: string) {
    const dir = String(parent || "").trim().replace(/^\/+/, "").replace(/\/+$/, "");
    const name = String(fromPath || "").split("/").pop() ?? "";
    return dir ? `${dir}/${name}` : name;
  }


  // ── Lock data ─────────────────────────────────────────────────────────────
  const lockedTemplates: string[] = Array.isArray(input?.locked_templates) ? input.locked_templates : [];
  const selectedTemplateLocked: boolean = !!input?.selected_template_locked;

  function isFilePathLocked(relPath: string): boolean {
    return lockedTemplates.some(p => relPath === p || relPath.startsWith(p.replace(/\/$/, "") + "/"));
  }

  function isFolderPathLocked(relPath: string): boolean {
    const clean = String(relPath || "").replace(/^\/+/, "").replace(/\/+$/, "");
    if (!clean) return false;
    return lockedTemplates.some(p => {
      const lockedPath = String(p || "").replace(/^\/+/, "").replace(/\/+$/, "");
      return lockedPath === clean || lockedPath.startsWith(`${clean}/`);
    });
  }

  function renderDeleteAffordance(options: {
    locked: boolean;
    title: string;
    onDelete?: () => void;
  }) {
    if (options.locked) {
      return (
        <span
          className="pipeline-registry-row-del inline-flex items-center justify-center text-dark-accent1"
          title="Locked — cannot delete"
          aria-label="Locked item"
        >
          <LockIcon />
        </span>
      );
    }
    return (
      <button
        type="button"
        className="pipeline-registry-row-del"
        title={options.title}
        onClick={options.onDelete}
      >
        <TrashIcon />
      </button>
    );
  }

  // ── Pipeline editor data ──────────────────────────────────────────────────
  const pipeline = input?.pipeline ?? {};
  const editorApi = pipeline?.api ?? {};

  // ── Template editor state ─────────────────────────────────────────────────
  const template = input?.template ?? {};
  const templateOutlineUrl = String(template?.api?.outline ?? editorApi?.template_outline ?? "");
  const [templateSaveState, setTemplateSaveState] = useState("Saved");
  const [editorPrefsVersion, setEditorPrefsVersion] = useState(0);
  const templateEditorHostRef = useRef(null);
  const templateEditorViewRef = useRef(null);
  const templateRuntimeRef = useRef(null);
  const templateEditorRelPathRef = useRef("");

  // ── Doc editor state ──────────────────────────────────────────────────────

  useEffect(() => {
    return subscribeEditorPreferences(() => {
      setEditorPrefsVersion((version) => version + 1);
    });
  }, []);

  // ── Split pane ────────────────────────────────────────────────────────────
  const pipelineEditorRef = useSplitPane({
    handleSelector: ".pipeline-editor-split-handle",
    variable: "--pipeline-editor-sidebar-width",
    min: 220,
    max: 480,
  });

  // ── Lock handlers ──────────────────────────────────────────────────────────

  async function handleTogglePipelineLock(newLocked: boolean) {
    const selectedId = pipeline?.selected_id ?? "";
    if (!selectedId) return;
    try {
      await requestJson(`/api/projects/${input?.owner ?? ""}/${input?.project ?? ""}/pipelines/lock-toggle`, {
        method: "POST",
        body: JSON.stringify({ file_rel_path: selectedId, locked: newLocked }),
      });
      nav(window.location.href);
    } catch (_) {}
  }

  async function handleToggleTemplateLock() {
    const relPath = template?.rel_path ?? "";
    if (!relPath) return;
    try {
      await requestJson(`/api/projects/${input?.owner ?? ""}/${input?.project ?? ""}/templates/lock-toggle`, {
        method: "POST",
        body: JSON.stringify({ rel_path: relPath, locked: !selectedTemplateLocked }),
      });
      nav(window.location.href);
    } catch (_) {}
  }

  // ── Creation dialogs ──────────────────────────────────────────────────────
  const owner = String(input?.owner ?? "");
  const project = String(input?.project ?? "");
  const projectApiBase = `/api/projects/${owner}/${project}`;

  // ── Live preview ──────────────────────────────────────────────────────────
  const [previewActive, setPreviewActive] = useState(false);
  const previewPollRef = useRef(null as any);
  const isTsxTemplate = (template?.rel_path ?? "").endsWith(".tsx");
  const previewApiBase = `${projectApiBase}/preview`;
  const previewUrl = `/preview/${owner}/${project}?file=${encodeURIComponent(template?.rel_path ?? "")}`;

  useEffect(() => {
    if (!isTsxTemplate) return;
    const checkStatus = async () => {
      try {
        const res = await fetch(`${previewApiBase}/status?file=${encodeURIComponent(template?.rel_path ?? "")}`);
        const data = await res.json();
        setPreviewActive(!!data.active);
      } catch (_) {}
    };
    checkStatus(); // immediate on mount / template change
    previewPollRef.current = setInterval(checkStatus, 3000);
    return () => clearInterval(previewPollRef.current);
  }, [template?.rel_path]);

  async function handleTogglePreview() {
    const next = !previewActive;
    try {
      await requestJson(`${previewApiBase}/toggle`, {
        method: "POST",
        body: JSON.stringify({ active: next, file: template?.rel_path ?? "" }),
      });
      setPreviewActive(next);
      if (next) window.open(previewUrl, "_blank");
    } catch (_) {}
  }
  const newPipelineDialogRef = useRef(null);
  const newFileDialogRef = useRef(null);
  const newFolderDialogRef = useRef(null);
  const newDocDialogRef = useRef(null);
  const [scriptPromptOpen, setScriptPromptOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState(null as string | null);

  // ── Install catalog state ──────────────────────────────────────────────────
  const nav = useRouter().push;
  const [installOpen, setInstallOpen] = useState(false);
  const [catalogData, setCatalogData] = useState([] as any[]);
  const [catalogLoaded, setCatalogLoaded] = useState(false);
  const [hubPacks, setHubPacks] = useState([] as any[]);
  const [packSearch, setPackSearch] = useState("");
  const [selectedComponents, setSelectedComponents] = useState(new Set<string>());
  const [installing, setInstalling] = useState(false);
  const [installResult, setInstallResult] = useState(null as string | null);
  const [installTab, setInstallTab] = useState("packs");
  const [hubInstallReview, setHubInstallReview] = useState(null as any);
  const [pendingHubAdd, setPendingHubAdd] = useState(null as any);
  const [hubReviewTargetFolder, setHubReviewTargetFolder] = useState("");
  const [hubReviewDirty, setHubReviewDirty] = useState(false);
  const [uiInstallReview, setUiInstallReview] = useState(null as any);
  const [uiReviewDirty, setUiReviewDirty] = useState(false);

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

  function openTemplateEditorPathAtLine(relPath: string, line?: number | null) {
    const normalized = String(relPath || "").replace(/^\/+/, "");
    if (!normalized) return;
    const parts = normalized.split("/");
    const dir = parts.slice(0, -1).join("/");
    const suffix = line && line > 0 ? `&line=${encodeURIComponent(String(line))}` : "";
    nav(`${editorBase}?type=file&path=${encodeURIComponent(dir)}&file=${encodeURIComponent(normalized)}${suffix}`);
  }

  function revealEditorLine(view: any, lineNumber?: number | null) {
    const line = Number(lineNumber || 0);
    if (!view || !line || line < 1) return;
    const runReveal = () => {
      const targetLine = Math.min(line, view.state.doc.lines);
      const lineInfo = view.state.doc.line(targetLine);
      const block = typeof view.lineBlockAt === "function" ? view.lineBlockAt(lineInfo.from) : null;
      const scroller = view.scrollDOM || view.dom?.querySelector?.(".cm-scroller");
      view.dispatch({
        selection: { anchor: lineInfo.from },
      });
      if (scroller && block) {
        scroller.scrollTop = Math.max(0, block.top - Math.max(scroller.clientHeight * 0.28, 48));
      }
      view.focus();
    };
    if (typeof requestAnimationFrame === "function") {
      requestAnimationFrame(() => {
        runReveal();
        requestAnimationFrame(runReveal);
      });
      return;
    }
    setTimeout(runReveal, 0);
  }

  function mountTemplateEditor(content, fileKind, rt, editorOptions: any = {}) {
    if (templateEditorViewRef.current) {
      templateEditorViewRef.current.destroy();
      templateEditorViewRef.current = null;
    }
    if (!templateEditorHostRef.current) return;
    const { EditorView, presets } = rt.cm;
    const extensions = presets.zebflow({
      kind: fileKind === "style" ? "css" : "template",
      height: "100%",
      autocomplete: true,
      diagnostics: true,
      clipboardSource: "template-editor",
      readonly: !!editorOptions.readonly,
      projectFiles: editorOptions.projectFiles || [],
      templateOutlineUrl: editorOptions.templateOutlineUrl || "",
      completionCatalog: editorOptions.completionCatalog || null,
      onOpenImport: editorOptions.onOpenImport,
      onSave: () => { void handleSaveTemplate(); },
      onDocumentChange: (update) => {
        if (!update.docChanged) return;
        setTemplateSaveState("Unsaved");
      },
    });
    templateEditorViewRef.current = new EditorView({
      doc: content,
      extensions,
      parent: templateEditorHostRef.current,
    });
    templateEditorRelPathRef.current = String(template?.rel_path ?? "");
    revealEditorLine(templateEditorViewRef.current, editorOptions.initialLine);
  }

  useEffect(() => {
    if (!isFile) return;
    const relPath = String(template?.rel_path ?? "");
    const content = templateEditorRelPathRef.current === relPath
      ? (templateEditorViewRef.current?.state?.doc?.toString?.() ?? template?.content ?? "")
      : (template?.content ?? "");
    const fileKind = template?.file_kind ?? "template";
    setTemplateSaveState("Loading…");
    (async () => {
      try {
        let rt = templateRuntimeRef.current;
        if (!rt) {
          rt = await loadEditorRuntime();
          templateRuntimeRef.current = rt;
        }
        const [workspace, completionCatalog] = await Promise.all([
          requestJson(`${projectApiBase}/repo`).catch(() => null),
          loadEditorCompletionCatalog(projectApiBase),
        ]);
        const projectFiles = Array.isArray(workspace?.items)
          ? workspace.items
              .filter((item: any) => item?.kind !== "folder" && typeof item?.rel_path === "string")
              .map((item: any) => String(item.rel_path))
          : [];
        mountTemplateEditor(content, fileKind, rt, {
          projectFiles,
          templateOutlineUrl,
          completionCatalog,
          readonly: selectedTemplateLocked,
          initialLine: selectedLine,
          onOpenImport: (target: any) => {
            if (target?.kind === "project" && target?.relPath) {
              openTemplateEditorPathAtLine(target.relPath, target?.line);
            }
          },
        });
        setTemplateSaveState("Saved");
      } catch (err) {
        setTemplateSaveState("Error");
        console.error("[EDITOR] template init failed", err);
      }
    })();
  }, [isFile, template?.rel_path, template?.content, template?.file_kind, templateOutlineUrl, selectedLine, selectedTemplateLocked, editorPrefsVersion]);

  async function handleSaveTemplate() {
    if (!templateEditorViewRef.current || selectedTemplateLocked) return;
    setTemplateSaveState("Saving…");
    try {
      const content = templateEditorViewRef.current.state.doc.toString();
      await requestJson(template?.api?.save ?? "", {
        method: "PUT",
        body: JSON.stringify({ rel_path: template?.rel_path ?? "", content }),
      });
      setTemplateSaveState("Saved");
      void refreshEditorCompletionCatalog(projectApiBase);
      notifyStudioRepoChanged();
    } catch (err) {
      setTemplateSaveState("Error");
    }
  }


  // ── Install handlers ──────────────────────────────────────────────────────

  async function loadCatalog() {
    try {
      const [uiRes, packsRes] = await Promise.all([
        fetch(`${projectApiBase}/install/catalog/ui`, { headers: { Accept: "application/json" } }),
        fetch(`${projectApiBase}/hub/assets`, { headers: { Accept: "application/json" } }),
      ]);
      const uiJson = await uiRes.json().catch(() => null);
      const packsJson = await packsRes.json().catch(() => null);
      setCatalogData(uiJson?.components ?? []);
      setHubPacks(Array.isArray(packsJson?.items) ? packsJson.items : []);
      setCatalogLoaded(true);
    } catch {
      setCatalogData([]);
      setHubPacks([]);
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
        setUiInstallReview(null);
        setUiReviewDirty(false);
        setInstallResult(parts.join(" · ") || "Done.");
        if (installed.length > 0) {
          void refreshEditorCompletionCatalog(projectApiBase);
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

  async function reviewUiInstall() {
    const names = Array.from(selectedComponents);
    if (names.length === 0) { setInstallResult("Select at least one component."); return; }
    setInstalling(true);
    setInstallResult("Reviewing UI components...");
    try {
      const json = await requestJson(`${projectApiBase}/install/ui/review`, {
        method: "POST",
        body: JSON.stringify({ names, overwrite: false }),
      });
      setUiInstallReview(json?.review || null);
      setUiReviewDirty(false);
      setInstallResult(null);
    } catch (err: any) {
      setInstallResult(String(err?.message || err));
    } finally {
      setInstalling(false);
    }
  }

  function cancelUiReview() {
    setUiInstallReview(null);
    setUiReviewDirty(false);
    setInstallResult(null);
  }

  function hubAssetSlug(item: any) {
    const packageId = String(item?.package_id || "").trim();
    const tail = packageId.includes(".") ? packageId.split(".").pop() : packageId;
    return peSanitizeSegment(tail || item?.title || "hub-asset") || "hub-asset";
  }

  function defaultHubTargetFolder(item: any) {
    const slug = hubAssetSlug(item);
    const base = peNormalizeVirtualPath(currentPath);
    return base === "/" ? `/${slug}` : `${base.replace(/\/+$/, "")}/${slug}`;
  }

  async function handleAddPack(item: any, targetFolder?: string) {
    const packageId = item?.package_id;
    const version = item?.latest_version;
    if (!packageId || !version) {
      setInstallResult("Package is missing package id or version.");
      return;
    }
    const resolvedTargetFolder = String(targetFolder || defaultHubTargetFolder(item));
    setInstalling(true);
    setInstallResult(`Reviewing ${packageId}@${version}...`);
    try {
      const url = item?.source === "remote"
        ? `${projectApiBase}/hub/repositories/${encodeURIComponent(item.repository_id)}/packs/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/review`
        : `${projectApiBase}/hub/assets/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/review`;
      const json = await requestJson(url, {
        method: "POST",
        body: JSON.stringify({ target_folder: resolvedTargetFolder }),
      });
      setHubInstallReview(json?.review || null);
      setPendingHubAdd({ item, targetFolder: resolvedTargetFolder });
      setHubReviewTargetFolder(resolvedTargetFolder);
      setHubReviewDirty(false);
      setInstallResult(null);
    } catch (err: any) {
      setInstallResult(String(err?.message || err));
    } finally {
      setInstalling(false);
    }
  }

  async function confirmHubAdd() {
    const item = pendingHubAdd?.item;
    const targetFolder = String(pendingHubAdd?.targetFolder || hubReviewTargetFolder || "");
    const packageId = item?.package_id;
    const version = item?.latest_version;
    if (!item || !packageId || !version) {
      setInstallResult("No reviewed Hub package is pending.");
      return;
    }
    setInstalling(true);
    setInstallResult(`Adding ${packageId}@${version}...`);
    try {
      const url = item?.source === "remote"
        ? `${projectApiBase}/hub/repositories/${encodeURIComponent(item.repository_id)}/packs/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`
        : `${projectApiBase}/hub/assets/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`;
      const json = await requestJson(url, {
        method: "POST",
        body: JSON.stringify({ target_folder: targetFolder }),
      });
      const result = json?.result || {};
      setHubInstallReview(null);
      setPendingHubAdd(null);
      void refreshEditorCompletionCatalog(projectApiBase);
      setInstallResult(`Added ${result.files_written || 0} file(s) into ${result.install_root || "project"} workspace`);
      setTimeout(() => {
        setInstallOpen(false);
        nav(`${editorBase}?path=${encodeURIComponent(currentPath)}`);
      }, 1200);
    } catch (err: any) {
      setInstallResult(String(err?.message || err));
    } finally {
      setInstalling(false);
    }
  }

  function cancelHubAddReview() {
    setHubInstallReview(null);
    setPendingHubAdd(null);
    setHubReviewTargetFolder("");
    setHubReviewDirty(false);
    setInstallResult(null);
  }

  // ── Create handlers ───────────────────────────────────────────────────────

  async function handleCreatePipeline(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const triggerKind = String(fd.get("trigger_kind") || "webhook");
    const name = peSanitizeSegment(fd.get("name"));
    const virtualPath = peNormalizeVirtualPath(currentPath);
    const title = String(fd.get("title") || "");
    const source = JSON.stringify(peEmptyPipelineDocument(name, triggerKind), null, 2);
    // Identity is relative to the project's source root, which lives in
    // zebflow.yaml and is never spelled here.
    const cleanVp = (virtualPath || "/").replace(/^\//, "");
    const fileRelPath = cleanVp ? `${cleanVp}/${name}.zf.json` : `${name}.zf.json`;
    setCreating(true);
    setCreateError(null);
    try {
      const payload = await requestJson(`${projectApiBase}/pipelines/definition`, {
        method: "POST",
        body: JSON.stringify({ file_rel_path: fileRelPath, title, description: "", trigger_kind: triggerKind, source }),
      });
      setTreeRefreshToken((n) => n + 1);
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
      setCreateTarget(null);
    }
  }

  const NEW_FILE_STARTERS: Record<string, { ext: string; body: (name: string) => string }> = {
    page: {
      ext: "tsx",
      body: (name) => `export default function ${name}() {\n  return <div className="p-8">${name}</div>;\n}\n`,
    },
    component: {
      ext: "tsx",
      body: (name) => `export default function ${name}({ children }: any) {\n  return <div>{children}</div>;\n}\n`,
    },
    style: { ext: "css", body: () => "" },
    script: { ext: "ts", body: () => "export {};\n" },
    doc: { ext: "md", body: (name) => `# ${name}\n` },
  };

  function pascalCase(raw: string) {
    return String(raw || "")
      .split(/[^A-Za-z0-9]+/)
      .filter(Boolean)
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join("") || "Untitled";
  }

  // A file is created where you are standing. The road this replaces sent a
  // page to `{source}/pages` whatever folder you had open.
  async function handleCreateFile(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const kind = String(fd.get("kind") || "page");
    const rawName = String(fd.get("name") || "").trim();
    if (!rawName) return;
    const starter = NEW_FILE_STARTERS[kind] ?? NEW_FILE_STARTERS.doc;
    const base = rawName.replace(new RegExp(`\\.${starter.ext}$`, "i"), "");
    const parent = currentPath.replace(/^\//, "");
    const relPath = parent ? `${parent}/${base}.${starter.ext}` : `${base}.${starter.ext}`;
    setCreating(true);
    setCreateError(null);
    try {
      const resp = await fetch(`${projectApiBase}/repo/file?path=${encodeURIComponent(relPath)}`, {
        method: "PUT",
        body: starter.body(pascalCase(base)),
        headers: { "Content-Type": "text/plain" },
      });
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        throw new Error(data?.error?.message ?? `Create failed: ${resp.status}`);
      }
      setTreeRefreshToken((n) => n + 1);
      nav(`${editorBase}?type=file&path=${encodeURIComponent(currentPath)}&file=${encodeURIComponent(relPath)}`);
      if (newFileDialogRef.current) newFileDialogRef.current.close();
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    } finally {
      setCreating(false);
      setCreateTarget(null);
    }
  }

  async function handleCreateFolder(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const name = peSanitizeSegment(fd.get("name"));
    setCreating(true);
    setCreateError(null);
    try {
      const newFolderVPath = currentPath === "/" ? `/${name}` : `${currentPath}/${name}`;
      await requestJson(`${projectApiBase}/repo/folder`, {
        method: "POST",
        body: JSON.stringify({ path: newFolderVPath.replace(/^\//, "") }),
      });
      setTreeRefreshToken((n) => n + 1);
      nav(`${editorBase}?path=${encodeURIComponent(newFolderVPath)}`);
      if (newFolderDialogRef.current) newFolderDialogRef.current.close();
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    } finally {
      setCreating(false);
      setCreateTarget(null);
    }
  }

  async function handleCreateDoc(e) {
    e.preventDefault();
    const fd = new FormData(e.currentTarget);
    const rawName = String(fd.get("name") || "").trim().replace(/\.md$/i, "");
    if (!rawName) return;
    const parent = currentPath.replace(/^\//, "");
    const filename = parent ? `${parent}/${rawName}.md` : `${rawName}.md`;
    setCreating(true);
    setCreateError(null);
    try {
      await fetch(`${projectApiBase}/repo/file?path=${encodeURIComponent(filename)}`, {
        method: "PUT",
        body: "",
        headers: { "Content-Type": "text/plain" },
      });
      if (newDocDialogRef.current) newDocDialogRef.current.close();
      nav(`${editorBase}?type=file&path=${encodeURIComponent(currentPath)}&file=${encodeURIComponent(filename)}`);
    } catch (err: any) {
      setCreateError(String(err?.message || err));
    } finally {
      setCreating(false);
      setCreateTarget(null);
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
  const [pendingMove, setPendingMove] = useState(null as any);
  const [moveTargetParent, setMoveTargetParent] = useState("");
  const [moving, setMoving] = useState(false);
  const [moveError, setMoveError] = useState(null as any);

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
      } else {
        resp = await fetch(`/api/projects/${owner}/${project}/repo/file?path=${encodeURIComponent(pendingDelete.path)}`, {
          method: "DELETE",
        });
      }
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        setDeleteError(data?.error ?? `Delete failed: ${resp.status}`);
        setDeleting(false);
        return;
      }
      setTreeRefreshToken((n) => n + 1);
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
        if (isFile) {
          nav(`${editorBase}?path=${encodeURIComponent(pendingDelete.parentPath ?? currentPath)}`);
        }
      }
    } catch (err) {
      setDeleteError(err?.message ?? "Network error");
      setDeleting(false);
    }
  }

  function openMoveDialog(entry) {
    setPendingMove(entry);
    setMoveTargetParent(entry?.targetParent ?? "/");
    setMoveError(null);
  }

  async function handleMoveConfirm() {
    if (!pendingMove) return;
    setMoving(true);
    setMoveError(null);
    try {
      const payload = await requestJson(`${projectApiBase}/repo/move`, {
        method: "POST",
        body: JSON.stringify({
          from_path: pendingMove.fromPath,
          to_path: joinRepoPath(moveTargetParent, pendingMove.fromPath),
        }),
      });
      const movedPath = String(payload?.path ?? pendingMove.fromPath);
      const movedDir = movedPath.includes("/") ? movedPath.split("/").slice(0, -1).join("/") : "";
      setPendingMove(null);
      setMoving(false);
      if (pendingMove.isFolder) {
        nav(`${editorBase}?path=${encodeURIComponent(`/${movedPath}`)}`);
      } else {
        nav(`${editorBase}?type=file&path=${encodeURIComponent(movedDir ? `/${movedDir}` : "/")}&file=${encodeURIComponent(movedPath)}`);
      }
    } catch (err: any) {
      setMoveError(String(err?.message || err));
      setMoving(false);
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
        <StudioTabNav>
          <StudioTabLink href={navLinks.pipelines_registry ?? "#"} active={!!navClasses.pipeline_registry}>Registry</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_webhooks ?? "#"} active={!!navClasses.pipeline_webhooks}>Webhooks</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_schedules ?? "#"} active={!!navClasses.pipeline_schedules}>Schedules</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_manual ?? "#"} active={!!navClasses.pipeline_manual}>Manual</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_functions ?? "#"} active={!!navClasses.pipeline_functions}>Functions</StudioTabLink>
        </StudioTabNav>
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
                <SidebarSearchButton editorBase={editorBase} nav={nav} />
                <DropdownMenu
                  trigger={<Button size="sm" variant="outline" className="flex items-center gap-1.5"><PlusIcon />New</Button>}
                  align="right"
                >
                  <DropdownMenuItem
                    label="Pipeline"
                    onClick={() => {
                      setCreateError(null);
                      setCreateTarget(null);
                      if (newPipelineDialogRef.current) newPipelineDialogRef.current.showModal();
                    }}
                  />
                  <DropdownMenuItem
                    label="Template file"
                    onClick={() => {
                      setCreateError(null);
                      setCreateTarget(null);
                      if (newFileDialogRef.current) newFileDialogRef.current.showModal();
                    }}
                  />
                  <DropdownMenuItem
                    label="Folder"
                    onClick={() => {
                      setCreateError(null);
                      setCreateTarget(null);
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
                  <DropdownMenuItem
                    label="Script prompt"
                    onClick={() => setScriptPromptOpen(true)}
                  />
                </DropdownMenu>
                  <Button size="sm" variant="ghost"
                  onClick={() => { setInstallResult(null); setInstallOpen(true); if (!catalogLoaded) loadCatalog(); }}
                  title="Add packages, pipelines, templates, and UI"
                  className="flex items-center gap-1.5">
                  <DownloadIcon />Add+
                </Button>
              </div>
            </div>

            {/* Scrollable sidebar body — folder nav + pipelines + templates always together */}
            <div className="pipeline-editor-sidebar-body">
              {/* The repository as a tree. Opening a folder is one request
                  for that folder's children, kept afterwards — where this
                  used to be a breadcrumb plus a link per child folder, and
                  every folder click reloaded the page. */}
              <RepoTree
                owner={owner}
                project={project}
                rootLabel={input?.title || project || "repository"}
                selected={treeSelection}
                onSelect={handleTreeSelect}
                onAction={handleTreeAction}
                refreshToken={treeRefreshToken}
              />

            </div>
          </aside>

          {/* ── Split handle ─────────────────────────────────────────── */}
          <div className="pipeline-editor-split-handle" aria-hidden="true"></div>

          {/* ── Main pane ────────────────────────────────────────────── */}
          <section className="pipeline-editor-main">

            {/* ── Asset manager view ──────────────────────────────────── */}
            {isAssets && <AssetManager api={assetsApi} subfolder={assetsSubfolder} />}

            {/* ── Folder view ─────────────────────────────────────────── */}
            {isFolder && !isAssets && (
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
                    const folderLocked = isFolderPathLocked(folderRelPath);
                    return (
                      <div
                        key={`ffolder-${index}`}
                        className="pipeline-registry-row pipeline-registry-folder-row"
                      >
                        <Link href={f?.href ?? "#"} className="pipeline-registry-row-link">
                          <span className="shrink-0 flex items-center text-body-soft"><FolderIcon /></span>
                          <span className="pipeline-registry-row-name">{f?.name}/</span>
                        </Link>
                        <div className="flex items-center gap-1 shrink-0">
                          <Button
                            variant="ghost"
                            size="xs"
                            onClick={() => openMoveDialog({
                              name: f?.name ?? "folder",
                              fromPath: folderRelPath,
                              isFolder: true,
                              targetParent: currentPath,
                            })}
                          >
                            Move
                          </Button>
                          {renderDeleteAffordance({
                            locked: folderLocked,
                            title: `Delete folder ${f?.name}`,
                            onDelete: () => {
                              setPendingDelete({ path: folderRelPath, name: f?.name ?? "folder", isPipeline: false, isFolder: true, parentPath: currentPath });
                              setDeleteInput("");
                              setDeleteError(null);
                            },
                          })}
                        </div>
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
                      <span className="shrink-0 flex items-center text-body-soft"><FolderIcon /></span>
                      <span className="pipeline-registry-row-name">{f?.name}/</span>
                    </Link>
                  ))}

                  {/* Pipelines */}
                  {dynFolderPipelines.length > 0 ? (
                    <div className="pipeline-registry-section-head">Pipelines</div>
                  ) : null}
                  {dynFolderPipelines.map((item, index) => (
                    (() => {
                      const pipelineLocked = !!item?.is_locked;
                      return (
                        <div
                          key={`fpipeline-${index}`}
                          className="pipeline-registry-row"
                          data-pipeline-row=""
                          data-rel-path={item?.id ?? ""}
                        >
                          <Link href={item?.editor_href ?? "#"} className="pipeline-registry-row-link">
                            <span className="shrink-0 flex items-center text-body-soft"><PipelineIcon /></span>
                            <StatusDot isActive={item?.is_active} hasDraft={item?.has_draft} />
                            {pipelineLocked && <LockIcon className="w-3 h-3 text-dark-accent1 shrink-0" title="Locked — agents cannot access" />}
                            <span className="pipeline-registry-row-name">{item?.title || item?.name}</span>
                            <Badge variant="secondary">{item?.trigger_kind}</Badge>
                          </Link>
                          {renderDeleteAffordance({
                            locked: pipelineLocked,
                            title: `Delete ${item?.name ?? "pipeline"}`,
                            onDelete: () => {
                              setPendingDelete({ path: item?.id ?? "", name: item?.name ?? "", isPipeline: true });
                              setDeleteInput("");
                              setDeleteError(null);
                            },
                          })}
                        </div>
                      );
                    })()
                  ))}

                  {/* Template files */}
                  {dynFolderTemplates.length > 0 ? (
                    <div className="pipeline-registry-section-head">Files</div>
                  ) : null}
                  {dynFolderTemplates.map((file, index) => (
                    (() => {
                      const templateLocked = isFilePathLocked(file?.rel_path ?? "");
                      return (
                        <div
                          key={`ffile-${index}`}
                          className="pipeline-registry-row pipeline-registry-file-row"
                          data-pipeline-row=""
                          data-rel-path={file?.rel_path ?? ""}
                        >
                          <Link href={file?.editor_href ?? "#"} className="pipeline-registry-row-link">
                            <span className="shrink-0 flex items-center text-body-soft"><FileKindIcon name={file?.name ?? ""} /></span>
                            {templateLocked && <LockIcon className="w-3 h-3 text-dark-accent1 shrink-0" title="Locked — agents cannot access" />}
                            <span className="pipeline-registry-row-name">{file?.name}</span>
                          </Link>
                          <div className="flex items-center gap-1 shrink-0">
                            <Button
                              variant="ghost"
                              size="xs"
                              onClick={() => openMoveDialog({
                                name: file?.name ?? "file",
                                fromPath: file?.kind === "doc" ? String(file?.template_path ?? "").replace(/^docs\//, "") : file?.rel_path ?? "",
                                isFolder: false,
                                targetParent: file?.kind === "doc" ? currentPath : currentPath.replace(/^\//, "") || "/",
                              })}
                            >
                              Move
                            </Button>
                            {renderDeleteAffordance({
                              locked: templateLocked,
                              title: `Delete ${file?.name ?? "file"}`,
                              onDelete: () => {
                                setPendingDelete({ path: file?.rel_path ?? "", name: file?.name ?? "", isPipeline: false, parentPath: currentPath });
                                setDeleteInput("");
                                setDeleteError(null);
                              },
                            })}
                          </div>
                        </div>
                      );
                    })()
                  ))}

                  {(dynFolderNormalFolders.length + dynFolderSpecialFolders.length) === 0 && dynFolderPipelines.length === 0 && dynFolderTemplates.length === 0 ? (
                    <p className="p-6 text-center text-[0.78rem] text-body-soft">Empty folder. Use <strong>+ New</strong> to add pipelines.</p>
                  ) : null}
                </div>
              </div>

              </>
            )}

            {/* ── Template editor ─────────────────────────────────────── */}
            {isFile && (
              <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-[var(--zf-radius-panel)] border border-border bg-surface">
                <div className="pipeline-editor-toolbar border-b border-border-soft">
                  <div className="flex items-start justify-between gap-4">
                    <div className="pipeline-editor-toolbar-main">
                      <p className="pipeline-editor-title">{template?.name}</p>
                      <p className="pipeline-editor-subtitle">{template?.rel_path}</p>
                    </div>
                    <div className="flex min-w-0 shrink-0 items-center justify-end gap-2 overflow-x-auto">
                      <div className="flex shrink-0 items-center gap-3">
                        <span className="pipeline-editor-indicator">{templateSaveState}</span>
                        <span className="pipeline-editor-indicator">{template?.file_kind}</span>
                        <span className="pipeline-editor-indicator">
                          {selectedTemplateLocked ? "locked" : "editable"}
                        </span>
                        {isTsxTemplate ? (
                          <span className="pipeline-editor-indicator">
                            {previewActive ? "live" : "preview off"}
                          </span>
                        ) : null}
                      </div>
                      <div className="flex shrink-0 items-center gap-2">
                        <Button variant="outline" size="xs" onClick={handleSaveTemplate} disabled={selectedTemplateLocked}>
                          Save
                        </Button>
                        {isTsxTemplate && (
                          <Button
                            variant={previewActive ? "live" : "outline"}
                            size="xs"
                            onClick={handleTogglePreview}
                          >
                            {previewActive ? "● Live" : "Live Preview"}
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={handleToggleTemplateLock}
                          title={selectedTemplateLocked ? "Unlock (allow agent access)" : "Lock (block agent access)"}
                          aria-label={selectedTemplateLocked ? "Unlock template editor" : "Lock template editor"}
                          className={selectedTemplateLocked ? "text-dark-accent1" : "text-body hover:text-dark-accent1"}
                        >
                          {selectedTemplateLocked ? <LockIcon /> : <LockOpenIcon />}
                        </Button>
                        {!selectedTemplateLocked ? (
                          <Button
                            variant="destructive"
                            size="icon"
                            onClick={() => {
                              setPendingDelete({ path: template?.rel_path ?? "", name: template?.name ?? "", isPipeline: false });
                              setDeleteInput("");
                              setDeleteError(null);
                            }}
                            title="Delete template"
                            aria-label="Delete template"
                          >
                            <TrashIcon />
                          </Button>
                        ) : (
                          <span className="inline-flex items-center justify-center text-dark-accent1" title="Locked — cannot delete" aria-label="Locked item">
                            <LockIcon />
                          </span>
                        )}
                        <Link href={`${editorBase}?path=${currentPath}`} className="zf-btn zf-btn-ghost zf-btn-xs">✕ Close</Link>
                      </div>
                    </div>
                  </div>
                </div>
                <div className="pipeline-editor-template-host flex-1 min-h-0" ref={templateEditorHostRef} />
                <div className="pipeline-editor-foot">
                  <span className="pipeline-editor-foot-item">{template?.name}</span>
                  <span className="pipeline-editor-foot-item">{templateSaveState}</span>
                  <span className="pipeline-editor-foot-item">zeb/codemirror@0.1</span>
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
                  execute: editorApi?.execute ?? "",
                  hits: editorApi?.hits ?? "",
                  invocations: editorApi?.invocations ?? "",
                  nodes: editorApi?.nodes ?? "",
                  credentials: editorApi?.credentials ?? "",
                  templatesWorkspace: editorApi?.templates_workspace ?? "",
                  templateFile: editorApi?.template_file ?? "",
                  templateSave: editorApi?.template_save ?? "",
                  templateOutline: editorApi?.template_outline ?? "",
                }}
                selectedId={pipeline?.selected_id ?? ""}
                owner={owner}
                project={project}
                scopePath={currentPath}
                graphuiSrc={pipeline?.graphui?.runtime_src ?? ""}
                graphuiPackageLabel={pipeline?.graphui?.package_label ?? "Graph UI"}
                projectDefaultMaxInvocations={Number(pipeline?.logging_defaults?.max_invocations ?? 20)}
                projectDefaultTraceCapture={pipeline?.logging_defaults?.trace_capture}
                onDeleteClick={pipeline?.selected_meta?.is_locked ? undefined : () => {
                  const pName = String(pipeline?.selected_meta?.name
                    ?? (pipeline?.selected_id ?? "").split("/").pop()?.replace(".zf.json", "")
                    ?? "");
                  setPendingDelete({ path: pipeline?.selected_id ?? "", name: pName, isPipeline: true });
                  setDeleteInput("");
                  setDeleteError(null);
                }}
                onLockToggle={handleTogglePipelineLock}
              />
            )}

            {/* ── No-selection placeholder (folder mode with no folder content) ── */}
            {!isPipeline && !isFile && !isFolder && (
              <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-body-muted">
                <p className="text-sm font-medium text-body">Select a file to edit</p>
              </div>
            )}
          </section>

          {/* ── Rename (a move inside the same folder) ────────────────────── */}
          <ConfirmDialog
            open={!!renaming}
            title={`Rename ${renaming?.name ?? ""}`}
            message="The new name keeps it in the same folder."
            confirmLabel="Rename"
            confirmDisabled={!renameInput.trim() || renameInput.trim() === renaming?.name}
            onClose={() => setRenaming(null)}
            onConfirm={confirmRename}
          >
            <Input
              className="mt-3"
              value={renameInput}
              placeholder={renaming?.name ?? ""}
              onInput={(e) => setRenameInput(e.currentTarget.value)}
            />
          </ConfirmDialog>

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

          {pendingMove && (
            <div className="pipeline-delete-overlay">
              <div className="pipeline-delete-backdrop" onClick={() => { setPendingMove(null); setMoveError(null); }} />
              <div className="pipeline-delete-box">
                <p className="pipeline-delete-title">Move <strong>{pendingMove.name}</strong></p>
                <p className="pipeline-delete-warn">Enter the destination parent folder.</p>
                <input
                  type="text"
                  className="pipeline-delete-input"
                  placeholder="/"
                  value={moveTargetParent}
                  onInput={(e) => setMoveTargetParent(e.currentTarget.value)}
                />
                {moveError ? <p className="pipeline-delete-error">{moveError}</p> : null}
                <div className="pipeline-delete-actions">
                  <button
                    type="button"
                    className="zf-btn zf-btn-primary zf-btn-sm"
                    disabled={moving}
                    onClick={handleMoveConfirm}
                  >
                    {moving ? "Moving…" : "Move"}
                  </button>
                  <button
                    type="button"
                    className="zf-btn zf-btn-ghost zf-btn-sm"
                    onClick={() => { setPendingMove(null); setMoveError(null); }}
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
                <small className="pipeline-editor-field-help">Saved in <code>{currentPath === "/" ? "the repository root" : currentPath}</code></small>
              </label>
              {createError ? <p className="pipeline-editor-dialog-error">{createError}</p> : null}
              <div className="pipeline-editor-dialog-actions">
                <Button variant="outline" size="xs" type="button" onClick={() => { if (newDocDialogRef.current) newDocDialogRef.current.close(); }}>Cancel</Button>
                <Button size="xs" type="submit" disabled={creating}>{creating ? "Creating…" : "Create"}</Button>
              </div>
            </form>
          </dialog>

          <ScriptPromptDialog open={scriptPromptOpen} onClose={() => setScriptPromptOpen(false)} />

          {installOpen && (
            <RegistryInstallCatalog
              onClose={() => setInstallOpen(false)}
              installTab={installTab}
              setInstallTab={setInstallTab}
              catalogData={catalogData}
              hubPacks={hubPacks}
              packSearch={packSearch}
              setPackSearch={setPackSearch}
              hubReviewTargetFolder={hubReviewTargetFolder}
              setHubReviewTargetFolder={(value) => {
                setHubReviewTargetFolder(value);
                setHubReviewDirty(true);
              }}
              hubReviewDirty={hubReviewDirty}
              selectedComponents={selectedComponents}
              setSelectedComponents={(value) => {
                setSelectedComponents(value);
                setUiReviewDirty(true);
              }}
              installResult={installResult}
              installing={installing}
              onInstallSubmit={reviewUiInstall}
              onConfirmUiInstall={handleInstallSubmit}
              uiInstallReview={uiInstallReview}
              uiReviewDirty={uiReviewDirty}
              onRefreshUiReview={reviewUiInstall}
              onCancelUiReview={cancelUiReview}
              onAddPack={handleAddPack}
              hubInstallReview={hubInstallReview}
              onRefreshHubReview={() => {
                const item = pendingHubAdd?.item;
                if (item) void handleAddPack(item, hubReviewTargetFolder);
              }}
              onConfirmHubAdd={confirmHubAdd}
              onCancelHubAddReview={cancelHubAddReview}
            />
          )}

        </div>
      </div>
    </ProjectStudioShell>
  );
}
