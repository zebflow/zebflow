import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initPipelineRegistryBehavior } from "@/components/behavior/project-pipelines";
import { cx, Link } from "rwe";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import Checkbox from "@/components/ui/checkbox";

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

function LucideFolderIcon({ className = "w-5 h-5" }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
    </svg>
  );
}

export default function Page(input) {
  initPipelineRegistryBehavior();
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const registry = input?.registry ?? {};
  const registryApi = registry?.api ?? {};
  const breadcrumbs = Array.isArray(registry?.breadcrumbs) ? registry.breadcrumbs : [];
  const folders = Array.isArray(registry?.folders) ? registry.folders : [];
  const pipelines = Array.isArray(registry?.pipelines) ? registry.pipelines : [];

  return (
    <>
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
                  {breadcrumbs.map((crumb, index) => (
                    <span key={`${crumb?.path ?? "root"}-${index}`} className="project-inline-path-item">
                      {crumb?.show_divider ? <span className="project-inline-path-divider">/</span> : null}
                      <Link href={crumb?.path ?? "#"} className="project-inline-path-link">{crumb?.name ?? "/"}</Link>
                    </span>
                  ))}
                </div>
                <Button variant="outline" size="xs" data-new-folder-toggle="true">+ Folder</Button>
                <Button size="xs" data-new-pipeline-toggle="true">+ Pipeline</Button>
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

              {/* ── Entry grid ───────────────────────────────────────────── */}
              <section className="project-content-section">
                <div className="project-content-head">
                  <div>
                    <p className="project-content-title">{input?.page_title}</p>
                    <p className="project-content-copy">{input?.page_subtitle}</p>
                  </div>
                </div>
                <div className="project-content-body">
                  <div className="project-entry-grid">
                    {folders.map((folder, index) => (
                      <Link key={`${folder?.path ?? "folder"}-${index}`} href={folder?.path ?? "#"} className="project-entry-card">
                        <span className="project-entry-icon project-entry-icon-folder">
                          <LucideFolderIcon />
                        </span>
                        <div className="project-entry-content">
                          <span className="project-list-title">{folder?.name}</span>
                          <p className="project-card-meta">Folder</p>
                        </div>
                      </Link>
                    ))}

                    {pipelines.map((item, index) => (
                      <Link
                        key={`${item?.id ?? item?.name ?? "pipeline"}-${index}`}
                        href={item?.edit_href ?? "#"}
                        className="project-entry-card"
                        data-pipeline-row="true"
                        data-rel-path={item?.file_rel_path ?? ""}
                        data-pipeline-name={item?.name ?? ""}
                      >
                        <span className="project-entry-icon">
                          <i className="devicon-yaml-plain colored text-xl" aria-hidden="true" />
                        </span>
                        <div className="project-entry-content">
                          <div className="flex items-start justify-between gap-3">
                            <div className="min-w-0">
                              <p className="project-list-title">{item?.title}</p>
                              <p className="project-card-meta">{item?.name}</p>
                            </div>
                            <Badge variant="secondary">{item?.trigger_kind}</Badge>
                          </div>
                          <p className="project-card-copy">{item?.description}</p>
                          <p className="project-card-meta">{item?.file_rel_path}</p>
                        </div>
                      </Link>
                    ))}

                    {folders.length === 0 && pipelines.length === 0 ? (
                      <p className="pipeline-registry-empty">No pipelines here. Use <strong>+ Pipeline</strong> to create one.</p>
                    ) : null}
                  </div>
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

              {/* ── Git commit dialog ────────────────────────────────────── */}
              <div hidden data-git-commit-dialog="true" className="git-commit-overlay">
                <div className="git-commit-backdrop" data-git-commit-close="true" />
                <div className="git-commit-box">
                  <div className="git-commit-header">
                    <h3 className="git-commit-title">Commit Changes</h3>
                    <Button variant="ghost" size="icon" className="git-commit-close" data-git-commit-close="true" aria-label="Close">✕</Button>
                  </div>
                  <div className="git-commit-file-list" data-git-commit-file-list="true"></div>
                  <textarea className="git-commit-message" data-git-commit-message="true" placeholder="Commit message…" rows={3} />
                  <Checkbox label="Push after commit" data-git-commit-push="true" className="git-commit-push-row" />
                  <p hidden data-git-commit-error="true" className="git-commit-error" />
                  <div className="git-commit-actions">
                    <Button size="xs" data-git-commit-submit="true" disabled>Commit</Button>
                    <Button variant="outline" size="xs" data-git-commit-close="true">Cancel</Button>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </ProjectStudioShell>
    </>
  );
}
