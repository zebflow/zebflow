import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initPipelineEditorBehavior, registerPipelineLoadedCallback } from "@/components/behavior/pipeline-editor";
import { initPipelineRegistryBehavior } from "@/components/behavior/project-pipelines";
import WebhookRouteTree from "@/components/ui/webhook-route-tree";
import { cx, Link, usePageState, useEffect } from "rwe";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import Checkbox from "@/components/ui/checkbox";
import Badge from "@/components/ui/badge";

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

function LucideFolderIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="pipeline-editor-nav-icon" aria-hidden="true">
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
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
  initPipelineEditorBehavior();
  initPipelineRegistryBehavior();
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const registry = input?.registry ?? {};
  const registryApi = registry?.api ?? {};
  const editor = input?.editor ?? {};

  const [pipelineLoaded, setPipelineLoaded] = usePageState("pe_pipeline_loaded", !!(editor?.selected_id));
  useEffect(() => {
    registerPipelineLoadedCallback(setPipelineLoaded);
    return () => registerPipelineLoadedCallback(null);
  }, []);
  const editorApi = editor?.api ?? {};
  const registryBreadcrumbs = Array.isArray(registry?.breadcrumbs) ? registry.breadcrumbs : [];
  const registryFolders = Array.isArray(registry?.folders) ? registry.folders : [];
  const registryPipelines = Array.isArray(registry?.pipelines) ? registry.pipelines : [];
  const scopeHierarchy = Array.isArray(editor?.scope_hierarchy) ? editor.scope_hierarchy : [];
  const scopeFolders = Array.isArray(editor?.scope_folders) ? editor.scope_folders : [];
  const editorPipelines = Array.isArray(editor?.pipelines) ? editor.pipelines : [];
  const currentPath = String(editor?.scope_path ?? "/");
  const editorBase = String(scopeHierarchy[0]?.href ?? "").replace(/\?path=.*$/, "");
  const expandedFolders = expandFolderPaths(scopeFolders, editorBase);
  const directChildFolders = getDirectChildFolders(expandedFolders, currentPath);
  const pipelineItems = Array.isArray(input?.pipeline_items) ? input.pipeline_items : [];

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
                <Button variant="outline" size="xs" data-new-folder-toggle="true">+ Folder</Button>
                <Button size="xs" data-new-pipeline-toggle="true">+ Pipeline</Button>
                <Button variant="outline" size="xs" data-registry-commit="true">Commit</Button>
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

              {/* ── Pipeline / folder list ───────────────────────────────── */}
              <section className="project-content-section">
                <div className="pipeline-registry-list">
                  {registryFolders.map((folder, index) => (
                    <Link
                      key={`${folder?.path ?? "folder"}-${index}`}
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

                  {registryPipelines.map((item, index) => (
                    <div
                      key={`${item?.file_rel_path ?? item?.name ?? "pipeline"}-${index}`}
                      className="pipeline-registry-row"
                      data-pipeline-row="true"
                      data-rel-path={item?.file_rel_path ?? ""}
                      data-pipeline-name={item?.name ?? ""}
                    >
                      <StatusDot isActive={item?.is_active} hasDraft={item?.has_draft} />
                      <div className="pipeline-registry-row-body">
                        <span className="pipeline-registry-row-name">{item?.title || item?.name}</span>
                        <Badge variant="secondary">{item?.trigger_kind}</Badge>
                        {item?.git_status
                          ? <Badge variant="destructive" title={`Git status: ${item.git_status}`}>uncommitted</Badge>
                          : null}
                      </div>
                      <div className="pipeline-registry-row-actions">
                        <Link href={item?.edit_href ?? "#"} className="project-inline-chip">Edit</Link>
                        <Button
                          variant="destructive"
                          size="xs"
                          data-delete-pipeline="true"
                          data-pipeline-name={item?.name ?? ""}
                          data-rel-path={item?.file_rel_path ?? ""}
                        >Delete</Button>
                      </div>
                    </div>
                  ))}

                  {registryFolders.length === 0 && registryPipelines.length === 0 ? (
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
              className="pipeline-editor-shell"
              data-pipeline-editor={input?.is_editor ? "true" : "false"}
              data-editor-snap-grid="true"
              data-editor-selected-id={editor?.selected_id ?? ""}
              data-editor-api-by-id={editorApi?.by_id ?? ""}
              data-editor-api-definition={editorApi?.definition ?? ""}
              data-editor-api-activate={editorApi?.activate ?? ""}
              data-editor-api-deactivate={editorApi?.deactivate ?? ""}
              data-editor-api-hits={editorApi?.hits ?? ""}
              data-editor-api-nodes={editorApi?.nodes ?? ""}
              data-editor-api-credentials={editorApi?.credentials ?? ""}
              data-editor-api-templates-workspace={editorApi?.templates_workspace ?? ""}
              data-editor-api-template-file={editorApi?.template_file ?? ""}
              data-editor-api-template-save={editorApi?.template_save ?? ""}
              data-editor-owner={input?.owner ?? ""}
              data-editor-project={input?.project ?? ""}
              data-editor-scope-path={editor?.scope_path ?? "/"}
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
                          <i className="devicon-yaml-plain colored text-xs" aria-hidden="true" />
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
              </aside>

              <section className="pipeline-editor-main relative">
                {!pipelineLoaded && (
                  <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 bg-[var(--studio-bg)] text-[var(--studio-muted)]">
                    <svg viewBox="0 0 24 24" fill="none" className="w-10 h-10 opacity-30" aria-hidden="true">
                      <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" strokeWidth="1.5"/>
                      <path d="M3 9h18" stroke="currentColor" strokeWidth="1.5"/>
                      <circle cx="7" cy="6" r="1" fill="currentColor"/>
                      <circle cx="10" cy="6" r="1" fill="currentColor"/>
                      <circle cx="13" cy="6" r="1" fill="currentColor"/>
                      <path d="M8 14h8M8 17h5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
                    </svg>
                    <p className="text-sm font-medium text-[var(--studio-text)]">No pipeline selected</p>
                    <p className="text-xs opacity-60">Select a pipeline from the sidebar to start editing.</p>
                  </div>
                )}
                <div className="pipeline-editor-toolbar">
                  <div className="pipeline-editor-toolbar-main">
                    <p className="pipeline-editor-title" data-editor-selected-name="true">No pipeline selected</p>
                    <p className="pipeline-editor-subtitle" data-editor-selected-meta="true">Select a pipeline to edit graph + node config.</p>
                  </div>
                  <div className="pipeline-editor-toolbar-actions">
                    <span className="pipeline-editor-indicator pipeline-editor-indicator-trigger" data-editor-trigger-kind="true">trigger: -</span>
                    <span className="pipeline-editor-indicator pipeline-editor-indicator-lock" data-editor-lock-state="true">editable</span>
                    <span className="pipeline-editor-indicator pipeline-editor-indicator-draft" data-editor-draft-state="true">draft unknown</span>
                    <Button variant="outline" size="xs" data-editor-save="true">Save Draft</Button>
                    <Button size="xs" data-editor-activate="true">Activate</Button>
                    <Button variant="outline" size="xs" data-editor-deactivate="true">Deactivate</Button>
                  </div>
                </div>

                <div className="pipeline-editor-graph-wrap">
                  <div className="pipeline-editor-canvas-tools" data-editor-categories="true">
                    <button type="button" className="pipeline-editor-cat" data-editor-cat="trigger" title="Trigger">
                      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M5 12h14M12 5l7 7-7 7" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"/></svg>
                    </button>
                    <button type="button" className="pipeline-editor-cat" data-editor-cat="data" title="Data">
                      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><ellipse cx="12" cy="6" rx="7" ry="3" stroke="currentColor" strokeWidth="1.7"/><path d="M5 6v8c0 1.66 3.13 3 7 3s7-1.34 7-3V6" stroke="currentColor" strokeWidth="1.7"/></svg>
                    </button>
                    <button type="button" className="pipeline-editor-cat" data-editor-cat="logic" title="Logic">
                      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><circle cx="7" cy="7" r="2" stroke="currentColor" strokeWidth="1.7"/><circle cx="17" cy="17" r="2" stroke="currentColor" strokeWidth="1.7"/><path d="M9 7h3a4 4 0 014 4v4" stroke="currentColor" strokeWidth="1.7"/></svg>
                    </button>
                    <button type="button" className="pipeline-editor-cat" data-editor-cat="web" title="Web">
                      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.7"/><path d="M12 3c-2.5 3-4 5.5-4 9s1.5 6 4 9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round"/><path d="M12 3c2.5 3 4 5.5 4 9s-1.5 6-4 9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round"/><path d="M3 12h18" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round"/></svg>
                    </button>
                    <button type="button" className="pipeline-editor-cat" data-editor-cat="security" title="Security">
                      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M12 2l7 4v6c0 4.42-3.13 8.56-7 9.93C8.13 20.56 5 16.42 5 12V6l7-4z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/></svg>
                    </button>
                    <div className="pipeline-editor-cat-menu" data-editor-cat-menu="true"></div>
                  </div>
                  <div className="pipeline-editor-graph" data-pipeline-graph-root="true"></div>
                </div>

                <div className="pipeline-editor-foot">
                  <span className="pipeline-editor-foot-item">{editor?.graphui?.package_label ?? "Graph UI"}</span>
                  <span className="pipeline-editor-foot-item" data-editor-hit-success="true">Success: 0</span>
                  <span className="pipeline-editor-foot-item" data-editor-hit-failed="true">Failed: 0</span>
                  <span className="pipeline-editor-foot-item" data-editor-hit-error="true">Latest error: -</span>
                </div>
              </section>

              {/* ── Git commit dialog (shown after pipeline save) ─────── */}
              <div hidden data-editor-git-commit-dialog="true" className="git-commit-overlay">
                <div className="git-commit-backdrop" data-editor-git-commit-close="true" />
                <div className="git-commit-box">
                  <div className="git-commit-header">
                    <h3 className="git-commit-title">Commit Pipeline Changes</h3>
                    <Button variant="ghost" size="icon" className="git-commit-close" data-editor-git-commit-close="true" aria-label="Close">✕</Button>
                  </div>
                  <div className="git-commit-file-list" data-editor-git-commit-file-list="true">
                    {/* populated by pipeline-editor.ts after save */}
                  </div>
                  <textarea
                    className="git-commit-message"
                    data-editor-git-commit-message="true"
                    placeholder="Commit message…"
                    rows={3}
                  />
                  <Checkbox label="Push after commit" data-editor-git-commit-push="true" className="git-commit-push-row" />
                  <p hidden data-editor-git-commit-error="true" className="git-commit-error" />
                  <div className="git-commit-actions">
                    <Button size="xs" data-editor-git-commit-submit="true" disabled>Commit</Button>
                    <Button variant="outline" size="xs" data-editor-git-commit-close="true">Skip</Button>
                  </div>
                </div>
              </div>

              <dialog className="pipeline-editor-dialog" data-editor-new-dialog="true">
                <form method="dialog" className="pipeline-editor-dialog-form" data-editor-new-form="true">
                  <h3 className="pipeline-editor-dialog-title">Create Pipeline</h3>
                  <label className="pipeline-editor-field">
                    <span>Trigger</span>
                    <select name="trigger_kind" required>
                      <option value="webhook">Webhook</option>
                      <option value="schedule">Schedule</option>
                      <option value="manual">Manual</option>
                      <option value="function">Function</option>
                    </select>
                  </label>
                  <label className="pipeline-editor-field">
                    <span>Name</span>
                    <input name="name" type="text" placeholder="my-pipeline" required />
                  </label>
                  <label className="pipeline-editor-field">
                    <span>Folder Path</span>
                    <input name="virtual_path" type="text" placeholder="/blog/admin" defaultValue="/" />
                  </label>
                  <label className="pipeline-editor-field">
                    <span>Title</span>
                    <input name="title" type="text" placeholder="My Pipeline" />
                  </label>
                  <label className="pipeline-editor-field">
                    <span>Description</span>
                    <textarea name="description" rows="3" placeholder="Describe pipeline"></textarea>
                  </label>
                  <div className="pipeline-editor-dialog-actions">
                    <Button variant="outline" size="xs" data-editor-new-cancel="true">Cancel</Button>
                    <Button size="xs" type="submit">Create</Button>
                  </div>
                </form>
              </dialog>

              <dialog className="pipeline-editor-dialog" data-editor-node-dialog="true">
                <form method="dialog" className="pipeline-editor-dialog-form" data-editor-node-form="true">
                  <h3 className="pipeline-editor-dialog-title" data-editor-node-title="true">Edit Node</h3>
                  <p className="pipeline-editor-subtitle" data-editor-node-copy="true"></p>
                  <div className="pipeline-editor-node-fields" data-editor-node-fields="true"></div>
                  <div className="pipeline-editor-dialog-actions">
                    <Button variant="outline" size="xs" data-editor-node-cancel="true">Cancel</Button>
                    <Button size="xs" type="submit">Apply</Button>
                  </div>
                </form>
              </dialog>
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
