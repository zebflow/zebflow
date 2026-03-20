import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initPipelineRegistryBehavior } from "@/components/behavior/project-pipelines";
import WebhookRouteTree from "@/components/ui/webhook-route-tree";
import { cx, Link, usePageState, useEffect, useState, useRef } from "zeb";
import { useSplitPane } from "zeb/use";
import { loadEditorRuntime } from "@/components/behavior/template-editor-runtime";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import Checkbox from "@/components/ui/checkbox";
import Badge from "@/components/ui/badge";
import PipelineEditor from "@/components/pipeline-editor/index";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
    links: [
      { rel: "stylesheet", href: "/assets/libraries/zeb/icons/0.1/runtime/devicons.css" },
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

function LucideFolderIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="pipeline-editor-nav-icon" aria-hidden="true">
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
    </svg>
  );
}

function PipelineIcon({ className = "w-4 h-4" }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" className={className} aria-hidden="true">
      <circle cx="7" cy="7" r="2.2" stroke="currentColor" strokeWidth="1.6"/>
      <circle cx="17" cy="7" r="2.2" stroke="currentColor" strokeWidth="1.6"/>
      <circle cx="12" cy="17" r="2.2" stroke="currentColor" strokeWidth="1.6"/>
      <path d="M9.2 8.4l1.9 5.2M14.8 8.4l-1.9 5.2" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round"/>
    </svg>
  );
}

function FileKindIcon({ name = "" }) {
  const ext = (name.split(".").pop() ?? "").toLowerCase();
  if (ext === "tsx" || ext === "jsx") {
    return <i className="devicon-react-original colored text-[0.95rem] leading-none" aria-hidden="true" />;
  }
  if (ext === "ts") {
    return <i className="devicon-typescript-plain colored text-[0.95rem] leading-none" aria-hidden="true" />;
  }
  if (ext === "css" || ext === "scss") {
    return <i className="devicon-css3-plain colored text-[0.95rem] leading-none" aria-hidden="true" />;
  }
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round"/>
      <path d="M14 2v6h6" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round"/>
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5" aria-hidden="true">
      <path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  );
}

function pipelineNavLastSegment(virtualPath) {
  const parts = String(virtualPath || "").split("/").filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "/";
}

function expandFolderPaths(scopeFolders, editorBase) {
  const pathMap = new Map();
  for (const f of scopeFolders) {
    const vp = String(f?.virtual_path ?? "");
    if (!vp || vp === "/") continue;
    if (!pathMap.has(vp)) {
      pathMap.set(vp, { virtual_path: vp, count: 0, href: f?.href ?? `${editorBase}?path=${vp}` });
    }
    pathMap.get(vp).count += (f?.count ?? 0);
    // Derive all intermediate ancestor paths and accumulate counts
    const parts = vp.split("/").filter(Boolean);
    for (let i = 1; i < parts.length; i++) {
      const ancestor = "/" + parts.slice(0, i).join("/");
      if (!pathMap.has(ancestor)) {
        pathMap.set(ancestor, { virtual_path: ancestor, count: 0, href: `${editorBase}?path=${ancestor}` });
      }
      pathMap.get(ancestor).count += (f?.count ?? 0);
    }
  }
  return Array.from(pathMap.values()).sort((a, b) => a.virtual_path.localeCompare(b.virtual_path));
}

function getDirectChildFolders(allFolders, currentPath) {
  const normalized = String(currentPath || "/");
  return allFolders.filter((f) => {
    const vp = String(f?.virtual_path ?? "");
    if (vp === normalized) return false;
    const lastSlash = vp.lastIndexOf("/");
    const parent = lastSlash <= 0 ? "/" : vp.slice(0, lastSlash);
    return parent === normalized;
  });
}

function StatusDot({ isActive, hasDraft }) {
  const cls = isActive && !hasDraft
    ? "pipeline-status-dot dot-active"
    : hasDraft
      ? "pipeline-status-dot dot-draft"
      : "pipeline-status-dot dot-inactive";
  const title = isActive && !hasDraft ? "Active" : hasDraft ? "Draft" : "Inactive";
  return <span className={cls} title={title} />;
}

export default function Page(input) {
  initPipelineRegistryBehavior();
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const registry = input?.registry ?? {};
  const registryApi = registry?.api ?? {};
  const editor = input?.editor ?? {};

  const pipelineEditorRef = useSplitPane({
    handleSelector: ".pipeline-editor-split-handle",
    variable: "--pipeline-editor-sidebar-width",
    min: 220,
    max: 480,
  });
  const editorApi = editor?.api ?? {};
  const registryBreadcrumbs = Array.isArray(registry?.breadcrumbs) ? registry.breadcrumbs : [];
  const registryFolders = Array.isArray(registry?.folders) ? registry.folders : [];
  const registryPipelines = Array.isArray(registry?.pipelines) ? registry.pipelines : [];
  const registryFiles = Array.isArray(registry?.files) ? registry.files : [];
  const [regFilter, setRegFilter] = usePageState("reg_filter", "all");
  const normalFolders = registryFolders.filter((f) => !f?.is_special);
  const SPECIAL_ORDER = ["docs", "styles", "assets"];
  const specialFolders = [...registryFolders.filter((f) => f?.is_special)].sort(
    (a, b) => SPECIAL_ORDER.indexOf(a?.name) - SPECIAL_ORDER.indexOf(b?.name)
  );
  const scopeHierarchy = Array.isArray(editor?.scope_hierarchy) ? editor.scope_hierarchy : [];
  const scopeFolders = Array.isArray(editor?.scope_folders) ? editor.scope_folders : [];
  const editorPipelines = Array.isArray(editor?.pipelines) ? editor.pipelines : [];
  const currentPath = String(editor?.scope_path ?? "/");
  const editorBase = String(scopeHierarchy[0]?.href ?? "").replace(/\?path=.*$/, "");
  const expandedFolders = expandFolderPaths(scopeFolders, editorBase);
  const directChildFolders = getDirectChildFolders(expandedFolders, currentPath);
  const pipelineItems = Array.isArray(input?.pipeline_items) ? input.pipeline_items : [];
  const editorTemplateFiles = Array.isArray(editor?.template_files) ? editor.template_files : [];

  // ── Template editor state ──────────────────────────────────────────────
  const [selectedTemplateFile, setSelectedTemplateFile] = useState(null);
  const [templateSaveState, setTemplateSaveState] = useState("Saved");
  const templateEditorHostRef = useRef(null);
  const templateEditorViewRef = useRef(null);
  const templateRuntimeRef = useRef(null);

  async function requestJson(url, options: any = {}) {
    const response = await fetch(url, {
      headers: {
        Accept: "application/json",
        ...(options.body ? { "Content-Type": "application/json" } : {}),
      },
      ...options,
    });
    if (response.status === 401) { if (typeof window !== "undefined") window.location.href = "/login"; return null; }
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

  async function handleSelectTemplateFile(file) {
    setSelectedTemplateFile(file);
    setTemplateSaveState("Loading…");
    try {
      let rt = templateRuntimeRef.current;
      if (!rt) {
        rt = await loadEditorRuntime();
        templateRuntimeRef.current = rt;
      }
      const fileData = await requestJson(`${editorApi.template_file}?path=${encodeURIComponent(file.template_path)}`);
      mountTemplateEditor(fileData?.content ?? "", fileData?.file_kind ?? "script", rt);
      setTemplateSaveState("Saved");
    } catch (err) {
      setTemplateSaveState("Error");
      console.error("[EDITOR] template open failed", err);
    }
  }

  async function handleSaveTemplate() {
    if (!selectedTemplateFile || !templateEditorViewRef.current) return;
    setTemplateSaveState("Saving…");
    try {
      const content = templateEditorViewRef.current.state.doc.toString();
      await requestJson(editorApi.template_save, {
        method: "PUT",
        body: JSON.stringify({ rel_path: selectedTemplateFile.template_path, content }),
      });
      setTemplateSaveState("Saved");
      if (typeof window !== "undefined") window.dispatchEvent(new CustomEvent("zf:repo:changed"));
    } catch (err) {
      setTemplateSaveState("Error");
    }
  }

  function handleCloseTemplate() {
    setSelectedTemplateFile(null);
    setTemplateSaveState("Saved");
    if (templateEditorViewRef.current) {
      templateEditorViewRef.current.destroy();
      templateEditorViewRef.current = null;
    }
  }

  return (
<Page>
    <ProjectStudioShell
      projectHref={input?.project_href}
      projectLabel={input?.title}
      currentMenu={input?.current_menu}
      owner={input?.owner}
      project={input?.project}
      nav={input?.nav}
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <Link href={navLinks.pipelines_registry ?? "#"} className={cx("project-tab-link", navClasses.pipeline_registry)}>Registry</Link>
          <Link href={navLinks.pipelines_webhooks ?? "#"} className={cx("project-tab-link", navClasses.pipeline_webhooks)}>Webhooks</Link>
          <Link href={navLinks.pipelines_schedules ?? "#"} className={cx("project-tab-link", navClasses.pipeline_schedules)}>Schedules</Link>
          <Link href={navLinks.pipelines_manual ?? "#"} className={cx("project-tab-link", navClasses.pipeline_manual)}>Manual</Link>
          <Link href={navLinks.pipelines_functions ?? "#"} className={cx("project-tab-link", navClasses.pipeline_functions)}>Functions</Link>
        </nav>

        <section className="project-workspace-body">
          {input?.is_registry ? (
            <div
              className="project-registry-shell"
              data-pipeline-registry="true"
              data-owner={input?.owner ?? ""}
              data-project={input?.project ?? ""}
              data-api-delete={registryApi?.delete ?? ""}
              data-api-delete-template={registryApi?.delete_template ?? ""}
              data-api-git-status={registryApi?.git_status ?? ""}
              data-api-git-commit={registryApi?.git_commit ?? ""}
            >
              {/* ── Toolbar ─────────────────────────────────────────────── */}
              <div className="project-surface-toolbar">
                <div className="project-inline-path">
                  <span className="project-inline-path-label">Path</span>
                  {registryBreadcrumbs.map((crumb, index) => (
                    <span key={`${crumb?.path ?? "root"}-${index}`} className="project-inline-path-item">
                      {crumb?.show_divider ? <span className="project-inline-path-divider">/</span> : null}
                      <Link href={crumb?.path ?? "#"} className="project-inline-path-link">{crumb?.name ?? "/"}</Link>
                    </span>
                  ))}
                </div>
                <div className="flex items-center gap-1.5 shrink-0">
                  <Button variant="outline" size="xs" data-new-folder-toggle="true">+ Folder</Button>
                  <Button size="xs" data-new-pipeline-toggle="true">+ Pipeline</Button>
                  <Button variant="outline" size="xs" data-new-template-toggle="true">+ Template</Button>
                </div>
              </div>

              {/* ── Filter tabs ──────────────────────────────────────────── */}
              <div className="pipeline-registry-filters">
                {(["all", "pipelines", "templates", "scripts"] as const).map((f) => (
                  <button
                    key={f}
                    type="button"
                    onClick={() => setRegFilter(f)}
                    className={cx("pipeline-registry-filter-tab", regFilter === f ? "is-active" : "")}
                  >
                    {f === "all" ? "All" : f === "pipelines" ? "Pipelines" : f === "templates" ? "Templates" : "Scripts"}
                  </button>
                ))}
              </div>

              {/* ── Inline: new pipeline form ────────────────────────────── */}
              <div hidden data-new-pipeline-form="true" className="pipeline-registry-inline-form">
                <Input name="name" type="text" placeholder="pipeline-name" className="pipeline-registry-inline-input" />
                <Input name="title" type="text" placeholder="Title (optional)" className="pipeline-registry-inline-input" />
                <Select name="trigger_kind" className="pipeline-registry-inline-select">
                  <option value="webhook">Webhook</option>
                  <option value="schedule">Schedule</option>
                  <option value="manual">Manual</option>
                  <option value="function">Function</option>
                </Select>
                <Button size="xs" data-new-pipeline-submit="true">Create & Open</Button>
                <Button variant="outline" size="xs" data-new-pipeline-cancel="true">Cancel</Button>
              </div>

              {/* ── Inline: new folder form ──────────────────────────────── */}
              <div hidden data-new-folder-form="true" className="pipeline-registry-inline-form">
                <Input name="folder_name" type="text" placeholder="folder-name" className="pipeline-registry-inline-input" />
                <Button size="xs" data-new-folder-submit="true">Create Folder</Button>
                <Button variant="outline" size="xs" data-new-folder-cancel="true">Cancel</Button>
              </div>

              {/* ── Inline: new template form ────────────────────────────── */}
              <div hidden data-new-template-form="true" className="pipeline-registry-inline-form">
                <Input name="template_name" type="text" placeholder="template-name" className="pipeline-registry-inline-input" />
                <Select name="template_kind" className="pipeline-registry-inline-select">
                  <option value="page">Page (.tsx)</option>
                  <option value="component">Component (.tsx)</option>
                  <option value="script">Script (.ts)</option>
                </Select>
                <Button size="xs" data-new-template-submit="true">Create & Edit</Button>
                <Button variant="outline" size="xs" data-new-template-cancel="true">Cancel</Button>
              </div>

              {/* ── Registry list ────────────────────────────────────────── */}
              <section className="project-content-section">
                <div className="pipeline-registry-list">

                  {/* Subfolders */}
                  {(normalFolders.length > 0 || specialFolders.length > 0) ? (
                    <div className="pipeline-registry-section-head">Subfolders</div>
                  ) : null}

                  {normalFolders.map((folder, index) => (
                    <Link
                      key={`folder-${folder?.name ?? index}`}
                      href={folder?.path ?? "#"}
                      className="pipeline-registry-row pipeline-registry-folder-row"
                    >
                      <span className="pipeline-registry-row-icon">
                        <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true">
                          <path d="M3 7.5A1.5 1.5 0 014.5 6h4l1.5 2h9A1.5 1.5 0 0120.5 9.5v7A1.5 1.5 0 0119 18H4.5A1.5 1.5 0 013 16.5v-9z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/>
                        </svg>
                      </span>
                      <span className="pipeline-registry-row-name">{folder?.name}/</span>
                    </Link>
                  ))}

                  {specialFolders.length > 0 ? (
                    <div className="pipeline-registry-special-sep" aria-hidden="true" />
                  ) : null}
                  {specialFolders.map((folder, index) => (
                    <Link
                      key={`special-${folder?.name ?? index}`}
                      href={folder?.path ?? "#"}
                      className={cx("pipeline-registry-row pipeline-registry-folder-row pipeline-registry-special-folder", folder?.name === "docs" ? "registry-folder-docs" : folder?.name === "assets" ? "registry-folder-assets" : folder?.name === "styles" ? "registry-folder-styles" : "")}
                    >
                      <span className="pipeline-registry-row-icon">
                        <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true">
                          <path d="M3 7.5A1.5 1.5 0 014.5 6h4l1.5 2h9A1.5 1.5 0 0120.5 9.5v7A1.5 1.5 0 0119 18H4.5A1.5 1.5 0 013 16.5v-9z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/>
                        </svg>
                      </span>
                      <span className="pipeline-registry-row-name">{folder?.name}/</span>
                    </Link>
                  ))}

                  {/* Pipelines */}
                  {(regFilter === "all" || regFilter === "pipelines") && registryPipelines.length > 0 ? (
                    <div className="pipeline-registry-section-head">Pipelines</div>
                  ) : null}

                  {(regFilter === "all" || regFilter === "pipelines") && registryPipelines.map((item, index) => (
                    <div
                      key={`pipeline-${item?.file_rel_path ?? index}`}
                      className="pipeline-registry-row"
                      data-pipeline-row=""
                      data-rel-path={item?.file_rel_path ?? ""}
                    >
                      <Link href={item?.edit_href ?? "#"} className="pipeline-registry-row-link">
                        <span className="pipeline-registry-row-icon"><PipelineIcon /></span>
                        <StatusDot isActive={item?.is_active} hasDraft={item?.has_draft} />
                        <span className="pipeline-registry-row-name">{item?.title || item?.name}</span>
                        <Badge variant="secondary">{item?.trigger_kind}</Badge>
                        {item?.git_status
                          ? <Badge variant="destructive" title={`Git: ${item.git_status}`}>uncommitted</Badge>
                          : null}
                      </Link>
                      <button type="button" className="pipeline-registry-row-del"
                        data-delete-pipeline="" data-rel-path={item?.file_rel_path ?? ""}
                        data-pipeline-name={item?.name ?? ""} title={`Delete ${item?.name ?? "pipeline"}`}>
                        <TrashIcon />
                      </button>
                    </div>
                  ))}

                  {/* Templates */}
                  {(regFilter === "all" || regFilter === "templates" || regFilter === "scripts") && registryFiles.filter((f) => {
                    if (regFilter === "templates") return f?.kind === "template";
                    if (regFilter === "scripts") return f?.kind !== "template";
                    return true;
                  }).length > 0 ? (
                    <div className="pipeline-registry-section-head">Templates</div>
                  ) : null}

                  {(regFilter === "all" || regFilter === "templates" || regFilter === "scripts") && registryFiles.filter((f) => {
                    if (regFilter === "templates") return f?.kind === "template";
                    if (regFilter === "scripts") return f?.kind !== "template";
                    return true;
                  }).map((file, index) => (
                    <div
                      key={`file-${file?.rel_path ?? index}`}
                      className="pipeline-registry-row pipeline-registry-file-row"
                      data-pipeline-row=""
                      data-rel-path={file?.rel_path ?? ""}
                    >
                      <Link href={file?.edit_href ?? "#"} className="pipeline-registry-row-link">
                        <span className="pipeline-registry-row-icon">
                          <FileKindIcon name={file?.name ?? ""} />
                        </span>
                        <span className="pipeline-registry-row-name">{file?.name}</span>
                        {file?.git_status
                          ? <Badge variant="destructive" title={`Git: ${file.git_status}`}>uncommitted</Badge>
                          : null}
                      </Link>
                      <button type="button" className="pipeline-registry-row-del"
                        data-delete-pipeline="" data-rel-path={file?.rel_path ?? ""}
                        data-pipeline-name={file?.name ?? ""} title={`Delete ${file?.name ?? "file"}`}>
                        <TrashIcon />
                      </button>
                    </div>
                  ))}

                  {registryFolders.length === 0 && registryPipelines.length === 0 && registryFiles.length === 0 ? (
                    <p className="pipeline-registry-empty">No pipelines here. Use <strong>+ Pipeline</strong> to create one.</p>
                  ) : null}
                </div>
              </section>

              {/* ── Delete confirm dialog ────────────────────────────────── */}
              <div hidden data-delete-pipeline-dialog="true" className="pipeline-delete-overlay">
                <div className="pipeline-delete-backdrop" data-delete-cancel-btn="true" />
                <div className="pipeline-delete-box">
                  <h3 className="pipeline-delete-title">Delete Pipeline</h3>
                  <p className="pipeline-delete-copy">Type the pipeline name to confirm:</p>
                  <strong className="pipeline-delete-name" data-delete-pipeline-name="true"></strong>
                  <Input type="text" data-delete-confirm-input="true" className="pipeline-delete-input" autoComplete="off" placeholder="type name to confirm" />
                  <div className="pipeline-delete-actions">
                    <Button variant="destructive" size="xs" data-delete-confirm-btn="true" disabled>Delete</Button>
                    <Button variant="outline" size="xs" data-delete-cancel-btn="true">Cancel</Button>
                  </div>
                </div>
              </div>

              {/* ── Git commit dialog (file list populated by behavior) ───── */}
              <div hidden data-git-commit-dialog="true" className="git-commit-overlay">
                <div className="git-commit-backdrop" data-git-commit-close="true" />
                <div className="git-commit-box">
                  <div className="git-commit-header">
                    <h3 className="git-commit-title">Commit Changes</h3>
                    <Button variant="ghost" size="icon" className="git-commit-close" data-git-commit-close="true" aria-label="Close">✕</Button>
                  </div>
                  <div className="git-commit-file-list" data-git-commit-file-list="true">
                    {/* populated by initPipelineRegistryBehavior */}
                  </div>
                  <textarea
                    className="git-commit-message"
                    data-git-commit-message="true"
                    placeholder="Commit message…"
                    rows={3}
                  />
                  <Checkbox label="Push after commit" data-git-commit-push="true" className="git-commit-push-row" />
                  <p hidden data-git-commit-error="true" className="git-commit-error" />
                  <div className="git-commit-actions">
                    <Button size="xs" data-git-commit-submit="true" disabled>Commit</Button>
                    <Button variant="outline" size="xs" data-git-commit-close="true">Cancel</Button>
                  </div>
                </div>
              </div>
            </div>
          ) : null}

          {input?.is_editor ? (
            <div
              ref={pipelineEditorRef}
              className="pipeline-editor-shell"
            >
              <aside className="pipeline-editor-sidebar">
                <div className="pipeline-editor-sidebar-head">
                  <p className="pipeline-editor-title">Pipelines</p>
                  <Button size="xs" data-editor-new-open="true">+ New</Button>
                </div>

                {/* ── Folder navigator ─────────────────────────────── */}
                <div className="pipeline-editor-folder-nav">
                  <div className="pipeline-editor-folder-crumbs">
                    {scopeHierarchy.map((seg, index) => (
                      <span key={`crumb-${index}`} className="pipeline-editor-folder-crumb">
                        {index > 0 ? <span className="pipeline-editor-crumb-sep">/</span> : null}
                        <Link href={seg?.href ?? "#"} className="pipeline-editor-crumb-link">{seg?.name}</Link>
                      </span>
                    ))}
                  </div>
                  {directChildFolders.map((folder, index) => (
                    <Link
                      key={`child-folder-${index}`}
                      href={folder?.href ?? "#"}
                      className="pipeline-editor-nav-row"
                    >
                      <LucideFolderIcon />
                      <span className="pipeline-editor-nav-label">{pipelineNavLastSegment(folder?.virtual_path)}/</span>
                      <span className="pipeline-editor-nav-count">{folder?.count ?? 0}</span>
                    </Link>
                  ))}
                </div>

                <div className="pipeline-editor-list" data-editor-pipeline-list="true">
                  {editorPipelines.map((item, index) => (
                    <Link key={`${item?.id ?? "pipeline"}-${index}`} href={item?.editor_href ?? "#"} className="pipeline-editor-item" data-editor-pipeline-id={item?.id ?? ""}>
                      <div className="pipeline-editor-item-head">
                        <div className="flex items-center gap-1.5">
                          <PipelineIcon className="w-3.5 h-3.5 text-[var(--studio-accent)]" />
                          <StatusDot isActive={item?.is_active} hasDraft={item?.has_draft} />
                          <span className="pipeline-editor-item-name">{item?.name}</span>
                        </div>
                        <span className="pipeline-editor-item-status">
                          {item?.status_label}{item?.is_locked ? " | locked" : ""}
                        </span>
                      </div>
                      <p className="pipeline-editor-item-meta">{item?.virtual_path}</p>
                    </Link>
                  ))}
                </div>

                {editorTemplateFiles.length > 0 && (
                  <>
                    <div className="pipeline-editor-section-head">Templates</div>
                    <div className="pipeline-editor-list">
                      {editorTemplateFiles.map((file, index) => (
                        <button
                          key={`tpl-${file?.rel_path ?? index}`}
                          type="button"
                          className={cx("pipeline-editor-item", selectedTemplateFile?.template_path === file?.template_path ? "is-selected" : "")}
                          onClick={() => handleSelectTemplateFile(file)}
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
                        </button>
                      ))}
                    </div>
                  </>
                )}
              </aside>

              <div className="pipeline-editor-split-handle" aria-hidden="true"></div>

              <section className="pipeline-editor-main">

                {/* ── Template editor — proper flex child, shown instead of pipeline graph ── */}
                {selectedTemplateFile && (
                  <div className="flex flex-col flex-1 min-h-0">
                    <div className="pipeline-editor-toolbar">
                      <div className="pipeline-editor-toolbar-main">
                        <p className="pipeline-editor-title">{selectedTemplateFile?.name}</p>
                        <p className="pipeline-editor-subtitle">{selectedTemplateFile?.rel_path}</p>
                      </div>
                      <div className="pipeline-editor-toolbar-actions">
                        <span className="pipeline-editor-indicator">{templateSaveState}</span>
                        <span className="pipeline-editor-indicator">{selectedTemplateFile?.kind}</span>
                        <Button variant="outline" size="xs" onClick={handleSaveTemplate}>Save</Button>
                        <Button variant="ghost" size="xs" onClick={handleCloseTemplate}>✕ Close</Button>
                      </div>
                    </div>
                    <div className="pipeline-editor-template-host" ref={templateEditorHostRef} />
                    <div className="pipeline-editor-foot">
                      <span className="pipeline-editor-foot-item">{selectedTemplateFile?.name}</span>
                      <span className="pipeline-editor-foot-item">{templateSaveState}</span>
                      <span className="pipeline-editor-foot-item">zeb/codemirror@0.1</span>
                    </div>
                  </div>
                )}

                {/* ── Pipeline editor ─────────────────────────────────────── */}
                {!selectedTemplateFile && (
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
                    selectedId={editor?.selected_id ?? ""}
                    owner={input?.owner ?? ""}
                    project={input?.project ?? ""}
                    scopePath={editor?.scope_path ?? "/"}
                    graphuiSrc={editor?.graphui?.runtime_src ?? ""}
                    graphuiPackageLabel={editor?.graphui?.package_label ?? "Graph UI"}
                  />
                )}
              </section>
            </div>
          ) : null}

          {input?.is_non_registry && !input?.is_editor && !input?.is_webhooks ? (
            <div className="project-flat-list">
              <div className="project-surface-panel-head">{input?.page_title}</div>
              <div className="project-list">
                {pipelineItems.map((item, index) => (
                  <article key={`${item?.name ?? "pipeline"}-${index}`} className="project-list-item">
                    <p className="project-list-title">{item?.name}</p>
                    <p className="project-card-copy">{item?.description}</p>
                  </article>
                ))}
              </div>
            </div>
          ) : null}

          {input?.is_webhooks ? (
            <div className="project-flat-list">
              <div className="project-surface-panel-head">{input?.page_title}</div>
              <div className="project-webhook-tree">
                <WebhookRouteTree items={pipelineItems} />
              </div>
            </div>
          ) : null}
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
