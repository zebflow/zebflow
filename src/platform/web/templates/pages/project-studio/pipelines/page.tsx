import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { initPipelineRegistryBehavior } from "@/pages/project-studio/pipelines/pipelines-behavior";
import { initInstallCatalogBehavior } from "@/pages/project-studio/pipelines/install-catalog-behavior";
import WebhookRouteTree from "@/components/ui/webhook-route-tree";
import { cx, Link, usePageState, useNavigate } from "zeb";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import Checkbox from "@/components/ui/checkbox";
import Badge from "@/components/ui/badge";
export const page = {
  head: {
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

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
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
  initInstallCatalogBehavior();
  const spaNav = useNavigate();
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const registry = input?.registry ?? {};
  const registryApi = registry?.api ?? {};
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
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={navLinks.pipelines_registry ?? "#"} active={!!navClasses.pipeline_registry}>Registry</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_webhooks ?? "#"} active={!!navClasses.pipeline_webhooks}>Webhooks</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_schedules ?? "#"} active={!!navClasses.pipeline_schedules}>Schedules</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_manual ?? "#"} active={!!navClasses.pipeline_manual}>Manual</StudioTabLink>
          <StudioTabLink href={navLinks.pipelines_functions ?? "#"} active={!!navClasses.pipeline_functions}>Functions</StudioTabLink>
        </StudioTabNav>

        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg">
          {input?.is_registry ? (
            <div
              className="flex-1 min-h-0 flex flex-col"
              data-pipeline-registry="true"
              data-owner={input?.owner ?? ""}
              data-project={input?.project ?? ""}
              data-api-delete={registryApi?.delete ?? ""}
              data-api-delete-template={registryApi?.delete_template ?? ""}
              data-api-git-status={registryApi?.git_status ?? ""}
              data-api-git-commit={registryApi?.git_commit ?? ""}
            >
              {/* ── Toolbar ─────────────────────────────────────────────── */}
              <div className="flex items-center gap-3 px-[0.875rem] py-[0.625rem] border-b border-border bg-surface">
                <div className="flex flex-wrap items-center gap-[0.35rem] text-[0.72rem] text-body-soft">
                  <span className="font-mono uppercase tracking-[0.12em]">Path</span>
                  {registryBreadcrumbs.map((crumb, index) => (
                    <span key={`${crumb?.path ?? "root"}-${index}`} className="inline-flex items-center gap-[0.35rem]">
                      {crumb?.show_divider ? <span className="text-border">/</span> : null}
                      <Link href={crumb?.path ?? "#"} className="project-inline-path-link text-body">{crumb?.name ?? "/"}</Link>
                    </span>
                  ))}
                </div>
                <div className="flex items-center gap-1.5 shrink-0">
                  <Button variant="outline" size="xs" data-new-folder-toggle="true">+ Folder</Button>
                  <Button size="xs" data-new-pipeline-toggle="true">+ Pipeline</Button>
                  <Button variant="outline" size="xs" data-new-template-toggle="true">+ Template</Button>
                  <Button variant="outline" size="xs" data-install-catalog-open="true">⬇ Install</Button>
                </div>
              </div>

              {/* ── Filter tabs ──────────────────────────────────────────── */}
              <div className="flex gap-1 px-3 py-[0.375rem] border-b border-border bg-surface">
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
              <div hidden data-new-pipeline-form="true" className="flex items-center gap-2 px-3 py-2 border-t border-border-soft flex-wrap">
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
              <div hidden data-new-folder-form="true" className="flex items-center gap-2 px-3 py-2 border-t border-border-soft flex-wrap">
                <Input name="folder_name" type="text" placeholder="folder-name" className="pipeline-registry-inline-input" />
                <Button size="xs" data-new-folder-submit="true">Create Folder</Button>
                <Button variant="outline" size="xs" data-new-folder-cancel="true">Cancel</Button>
              </div>

              {/* ── Inline: new template form ────────────────────────────── */}
              <div hidden data-new-template-form="true" className="flex items-center gap-2 px-3 py-2 border-t border-border-soft flex-wrap">
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
                <div className="flex flex-col py-2 px-3 gap-1">

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
                      <span className="shrink-0 flex items-center text-body-soft">
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
                      <span className="shrink-0 flex items-center text-body-soft">
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
                        <span className="shrink-0 flex items-center text-body-soft"><PipelineIcon /></span>
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
                        <span className="shrink-0 flex items-center text-body-soft">
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
                    <p className="p-6 text-center text-[0.78rem] text-body-soft">No pipelines here. Use <strong>+ Pipeline</strong> to create one.</p>
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

              {/* ── Install Catalog modal ────────────────────────────────── */}
              <div hidden data-install-catalog-dialog="true" className="git-commit-overlay">
                <div className="git-commit-backdrop" data-install-catalog-close="true" />
                <div className="git-commit-box git-commit-box--install-catalog">
                  <div className="git-commit-header shrink-0">
                    <h3 className="git-commit-title">Install UI Components</h3>
                    <Button variant="ghost" size="icon" className="git-commit-close" data-install-catalog-close="true" aria-label="Close">✕</Button>
                  </div>

                  <div className="git-install-catalog-stack">
                    {/* Tab bar */}
                    <div className="git-install-catalog-tabs">
                      <button type="button" data-install-tab-btn="ui" data-install-tab-active="true" className="pipeline-registry-filter-tab is-active">UI Kit</button>
                      <button type="button" data-install-tab-btn="pipelines" className="pipeline-registry-filter-tab">Pipelines</button>
                      <button type="button" data-install-tab-btn="scripts" className="pipeline-registry-filter-tab">Scripts</button>
                    </div>

                    {/* UI Kit tab content */}
                    <div data-install-tab-content="ui" className="install-catalog-tab-panel">
                      <p style={{ fontSize: "12px", color: "var(--color-ui-text-muted)", margin: 0 }}>
                        Select components to install into <code>shared/ui/</code>. Installs as Zeb React TSX files.
                      </p>
                      <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
                        <button type="button" data-install-select-all="true" className="pipeline-registry-filter-tab">Select All</button>
                        <button type="button" data-install-select-none="true" className="pipeline-registry-filter-tab">None</button>
                        <button type="button" data-install-select-essentials="true" className="pipeline-registry-filter-tab">Essentials</button>
                      </div>
                      <div
                        data-install-component-list="true"
                        className="git-install-component-list-host"
                      >
                        {/* Populated by project-install.ts behavior */}
                      </div>
                      <p data-install-result="true" hidden style={{ fontSize: "12px", color: "var(--color-ui-text-muted)", margin: 0 }} />
                    </div>

                    {/* Pipelines tab content (future) */}
                    <div data-install-tab-content="pipelines" hidden className="install-catalog-tab-panel">
                      <p style={{ fontSize: "12px", color: "var(--color-ui-text-muted)", margin: 0 }}>Pipeline templates coming soon.</p>
                    </div>

                    {/* Scripts tab content (future) */}
                    <div data-install-tab-content="scripts" hidden className="install-catalog-tab-panel">
                      <p style={{ fontSize: "12px", color: "var(--color-ui-text-muted)", margin: 0 }}>Script templates coming soon.</p>
                    </div>
                  </div>

                  <div className="git-commit-actions shrink-0">
                    <Button size="xs" data-install-submit="true">Install Selected</Button>
                    <Button variant="outline" size="xs" data-install-catalog-close="true">Cancel</Button>
                  </div>
                </div>
              </div>
            </div>
          ) : null}

          {input?.is_non_registry && !input?.is_editor && !input?.is_webhooks ? (
            <div className="flex-1 min-h-0 overflow-auto flex flex-col">
              <div className="shrink-0 flex items-start justify-between gap-3 px-[0.875rem] py-[0.625rem] border-b border-border bg-surface">
                <div>
                  <p className="text-[0.68rem] uppercase tracking-[0.08em] text-body-soft">{input?.page_title}</p>
                  {input?.page_subtitle ? (
                    <p className="text-[0.72rem] text-body-soft mt-[0.2rem] leading-snug">{input?.page_subtitle}</p>
                  ) : null}
                </div>
                <Badge variant="secondary">{pipelineItems.length}</Badge>
              </div>
              <div className="flex flex-col py-2 px-3 gap-1">
                {pipelineItems.map((item, index) => (
                  <Link
                    key={`${item?.name ?? "pipeline"}-${index}`}
                    href={item?.editor_href ?? "#"}
                    className="pipeline-registry-row"
                  >
                    <span className="shrink-0 flex items-center text-body-soft"><PipelineIcon className="w-4 h-4" /></span>
                    <span className="flex-1 min-w-0 flex flex-col gap-[0.15rem]">
                      <span className="flex items-center gap-2 flex-wrap">
                        <span className="pipeline-registry-row-name">{item?.title || item?.name}</span>
                        {item?.virtual_path && item?.virtual_path !== "/" ? (
                          <span className="text-[0.65rem] text-body-soft font-mono">{item?.virtual_path}</span>
                        ) : null}
                      </span>
                      {item?.description ? (
                        <span className="text-[0.72rem] text-body-soft leading-snug">{item?.description}</span>
                      ) : null}
                    </span>
                  </Link>
                ))}
                {pipelineItems.length === 0 ? (
                  <p className="p-6 text-center text-[0.78rem] text-body-soft">
                    No {(input?.page_title as string)?.toLowerCase() ?? "pipelines"} yet. Create one in the Registry tab.
                  </p>
                ) : null}
              </div>
            </div>
          ) : null}

          {input?.is_webhooks ? (
            <div className="flex-1 min-h-0 overflow-auto flex flex-col">
              <div className="shrink-0 flex items-start justify-between gap-3 px-[0.875rem] py-[0.625rem] border-b border-border bg-surface">
                <div>
                  <p className="text-[0.68rem] uppercase tracking-[0.08em] text-body-soft">{input?.page_title}</p>
                  {input?.page_subtitle ? (
                    <p className="text-[0.72rem] text-body-soft mt-[0.2rem] leading-snug">{input?.page_subtitle}</p>
                  ) : null}
                </div>
                <Badge variant="secondary">{pipelineItems.length}</Badge>
              </div>
              <div className="flex-1 min-h-0 overflow-auto px-[0.875rem] py-3">
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
