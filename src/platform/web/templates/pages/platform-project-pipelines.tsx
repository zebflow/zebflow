import ProjectStudioShell from "@/components/layout/project-studio-shell";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    scripts: [
      {
        type: "module",
        src: "/assets/platform/pipeline-editor.mjs"
      },
      {
        type: "module",
        src: "/assets/platform/pipeline-webhook-tree.mjs"
      }
    ]
  },
  html: {
    lang: "en",
  },
  body: {
    className: "h-screen overflow-hidden bg-slate-950 text-slate-100 font-sans",
  },
  navigation: "history",
};

export const app = {};

export default function Page(input) {
  return (
<Page>
    <ProjectStudioShell
      projectHref="{input.project_href}"
      projectLabel="{input.title}"
      currentMenu="{input.current_menu}"
      owner="{input.owner}"
      project="{input.project}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a href="{input.nav.links.pipelines_registry}" className="project-tab-link {input.nav.classes.pipeline_registry}">Registry</a>
          <a href="{input.nav.links.pipelines_webhooks}" className="project-tab-link {input.nav.classes.pipeline_webhooks}">Webhooks</a>
          <a href="{input.nav.links.pipelines_schedules}" className="project-tab-link {input.nav.classes.pipeline_schedules}">Schedules</a>
          <a href="{input.nav.links.pipelines_manual}" className="project-tab-link {input.nav.classes.pipeline_manual}">Manual</a>
          <a href="{input.nav.links.pipelines_functions}" className="project-tab-link {input.nav.classes.pipeline_functions}">Functions</a>
        </nav>

        <section className="project-workspace-body">
          <div zShow="input.is_registry" className="project-registry-shell">
            <div className="project-surface-toolbar">
              <div className="project-inline-path">
                <span className="project-inline-path-label">Path</span>
                <span zFor="crumb in input.registry.breadcrumbs" className="project-inline-path-item">
                  <span zShow="crumb.show_divider" className="project-inline-path-divider">/</span>
                  <a href="{crumb.path}" className="project-inline-path-link">{crumb.name}</a>
                </span>
              </div>
              <a href="{input.registry.editor_href}" className="project-inline-chip project-inline-chip-accent">Open Editor</a>
              <details className="project-action-menu">
                <summary className="project-inline-chip project-inline-chip-accent">+New</summary>
                <div className="project-action-menu-panel">
                  <a href="{input.registry.editor_href}" className="project-action-menu-item">Pipeline (Editor)</a>
                  <a href="{input.nav.links.pipelines_webhooks}" className="project-action-menu-item">Webhook</a>
                  <a href="{input.nav.links.pipelines_schedules}" className="project-action-menu-item">Schedule</a>
                  <a href="{input.nav.links.pipelines_manual}" className="project-action-menu-item">Manual</a>
                  <a href="{input.nav.links.pipelines_functions}" className="project-action-menu-item">Function</a>
                </div>
              </details>
            </div>

            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">{input.page_title}</p>
                  <p className="project-content-copy">{input.page_subtitle}</p>
                </div>
              </div>
              <div className="project-content-body">
                <div className="project-entry-grid">
                  <a zFor="folder in input.registry.folders" href="{folder.path}" className="project-entry-card">
                    <span className="project-entry-icon">
                      <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
                        <path d="M3 7.5A1.5 1.5 0 014.5 6h4l1.5 2h9A1.5 1.5 0 0120.5 9.5v7A1.5 1.5 0 0119 18H4.5A1.5 1.5 0 013 16.5v-9z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                      </svg>
                    </span>
                    <div className="project-entry-content">
                      <span className="project-list-title">{folder.name}</span>
                      <p className="project-card-meta">Folder</p>
                    </div>
                  </a>

                  <a zFor="item in input.registry.pipelines" href="{item.edit_href}" className="project-entry-card">
                    <span className="project-entry-icon">
                      <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
                        <circle cx="6" cy="12" r="2" stroke="currentColor" stroke-width="1.7"/>
                        <circle cx="18" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                        <circle cx="18" cy="18" r="2" stroke="currentColor" stroke-width="1.7"/>
                        <path d="M8 12h4l4-6M12 12h4l-4 6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
                      </svg>
                    </span>
                    <div className="project-entry-content">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <p className="project-list-title">{item.title}</p>
                          <p className="project-card-meta">{item.name}</p>
                        </div>
                        <span className="project-inline-chip">{item.trigger_kind}</span>
                      </div>
                      <p className="project-card-copy">{item.description}</p>
                      <p className="project-card-meta">{item.file_rel_path}</p>
                    </div>
                  </a>
                </div>
              </div>
            </section>
          </div>

          <div zShow="input.is_editor" className="pipeline-editor-shell"
            data-pipeline-editor="{input.is_editor}"
            data-editor-snap-grid="true"
            data-editor-selected-id="{input.editor.selected_id}"
            data-editor-api-by-id="{input.editor.api.by_id}"
            data-editor-api-definition="{input.editor.api.definition}"
            data-editor-api-activate="{input.editor.api.activate}"
            data-editor-api-deactivate="{input.editor.api.deactivate}"
            data-editor-api-hits="{input.editor.api.hits}"
            data-editor-api-nodes="{input.editor.api.nodes}"
            data-editor-api-credentials="{input.editor.api.credentials}"
            data-editor-api-templates-workspace="{input.editor.api.templates_workspace}"
            data-editor-api-template-file="{input.editor.api.template_file}"
            data-editor-api-template-save="{input.editor.api.template_save}"
            data-editor-owner="{input.owner}"
            data-editor-project="{input.project}"
            data-editor-scope-path="{input.editor.scope_path}"
          >
            <aside className="pipeline-editor-sidebar">
              <div className="pipeline-editor-sidebar-head">
                <p className="pipeline-editor-title">Pipelines</p>
                <button type="button" className="project-inline-chip project-inline-chip-accent" data-editor-new-open="true">+ New</button>
              </div>

              <details className="pipeline-editor-folder-explorer">
                <summary className="pipeline-editor-folder-summary">
                  Folder: <span>{input.editor.scope_path}</span>
                </summary>
                <div className="pipeline-editor-folder-panel">
                  <p className="pipeline-editor-folder-label">Parents</p>
                  <a zFor="item in input.editor.scope_hierarchy" href="{item.href}" className="pipeline-editor-folder-link">{item.virtual_path}</a>
                  <p className="pipeline-editor-folder-label">Folders</p>
                  <a zFor="item in input.editor.scope_folders" href="{item.href}" className="pipeline-editor-folder-link">{item.virtual_path} ({item.count})</a>
                </div>
              </details>

              <div className="pipeline-editor-list" data-editor-pipeline-list="true">
                <a zFor="item in input.editor.pipelines" href="{item.editor_href}" className="pipeline-editor-item" data-editor-pipeline-id="{item.id}">
                  <div className="pipeline-editor-item-head">
                    <span className="pipeline-editor-item-name">{item.name}</span>
                    <span className="pipeline-editor-item-status">{item.status_label} <span zShow="item.is_locked">| locked</span></span>
                  </div>
                  <p className="pipeline-editor-item-meta">{item.virtual_path}</p>
                </a>
              </div>
            </aside>

            <section className="pipeline-editor-main">
              <div className="pipeline-editor-toolbar">
                <div className="pipeline-editor-toolbar-main">
                  <p className="pipeline-editor-title" data-editor-selected-name="true">No pipeline selected</p>
                  <p className="pipeline-editor-subtitle" data-editor-selected-meta="true">Select a pipeline to edit graph + node config.</p>
                </div>
                <div className="pipeline-editor-toolbar-actions">
                  <span className="project-inline-chip" data-editor-trigger-kind="true">trigger: -</span>
                  <span className="project-inline-chip" data-editor-lock-state="true">editable</span>
                  <span className="project-inline-chip" data-editor-draft-state="true">draft unknown</span>
                  <button type="button" className="project-inline-chip" data-editor-save="true">Save Draft</button>
                  <button type="button" className="project-inline-chip project-inline-chip-accent" data-editor-activate="true">Activate</button>
                  <button type="button" className="project-inline-chip" data-editor-deactivate="true">Deactivate</button>
                </div>
              </div>

              <div className="pipeline-editor-graph-wrap">
                <div className="pipeline-editor-canvas-tools" data-editor-categories="true">
                  <button type="button" className="pipeline-editor-cat" data-editor-cat="trigger" title="Trigger">
                    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M5 12h14M12 5l7 7-7 7" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
                  </button>
                  <button type="button" className="pipeline-editor-cat" data-editor-cat="data" title="Data">
                    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><ellipse cx="12" cy="6" rx="7" ry="3" stroke="currentColor" stroke-width="1.7"/><path d="M5 6v8c0 1.66 3.13 3 7 3s7-1.34 7-3V6" stroke="currentColor" stroke-width="1.7"/></svg>
                  </button>
                  <button type="button" className="pipeline-editor-cat" data-editor-cat="logic" title="Logic">
                    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><circle cx="7" cy="7" r="2" stroke="currentColor" stroke-width="1.7"/><circle cx="17" cy="17" r="2" stroke="currentColor" stroke-width="1.7"/><path d="M9 7h3a4 4 0 014 4v4" stroke="currentColor" stroke-width="1.7"/></svg>
                  </button>
                  <button type="button" className="pipeline-editor-cat" data-editor-cat="render" title="Render">
                    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M4 5h16v10H4z" stroke="currentColor" stroke-width="1.7"/><path d="M9 19h6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
                  </button>
                  <div className="pipeline-editor-cat-menu" data-editor-cat-menu="true"></div>
                </div>
                <div className="pipeline-editor-graph" data-pipeline-graph-root="true"></div>
              </div>

              <div className="pipeline-editor-foot">
                <span className="pipeline-editor-foot-item">{input.editor.graphui.package_label}</span>
                <span className="pipeline-editor-foot-item" data-editor-hit-success="true">Success: 0</span>
                <span className="pipeline-editor-foot-item" data-editor-hit-failed="true">Failed: 0</span>
                <span className="pipeline-editor-foot-item" data-editor-hit-error="true">Latest error: -</span>
              </div>
            </section>

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
                  <input name="virtual_path" type="text" placeholder="/blog/admin" value="/" />
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
                  <button type="button" data-editor-new-cancel="true">Cancel</button>
                  <button type="submit">Create</button>
                </div>
              </form>
            </dialog>

            <dialog className="pipeline-editor-dialog" data-editor-node-dialog="true">
              <form method="dialog" className="pipeline-editor-dialog-form" data-editor-node-form="true">
                <h3 className="pipeline-editor-dialog-title" data-editor-node-title="true">Edit Node</h3>
                <p className="pipeline-editor-subtitle" data-editor-node-copy="true"></p>
                <div className="pipeline-editor-node-fields" data-editor-node-fields="true"></div>
                <div className="pipeline-editor-dialog-actions">
                  <button type="button" data-editor-node-cancel="true">Cancel</button>
                  <button type="submit">Apply</button>
                </div>
              </form>
            </dialog>
          </div>

          <div zShow="input.is_non_registry && !input.is_editor && !input.is_webhooks" className="project-flat-list">
            <div className="project-surface-panel-head">{input.page_title}</div>
            <div className="project-list">
              <article zFor="item in input.pipeline_items" className="project-list-item">
                <p className="project-list-title">{item.name}</p>
                <p className="project-card-copy">{item.description}</p>
              </article>
            </div>
          </div>

          <div zShow="input.is_webhooks" className="project-flat-list">
            <div className="project-surface-panel-head">{input.page_title}</div>
            <div className="project-webhook-tree" data-webhook-tree="true">
              <div className="project-webhook-tree-root" data-webhook-tree-root="true"></div>
              <div hidden>
                <a
                  zFor="item in input.pipeline_items"
                  data-webhook-item="true"
                  data-webhook-route="{item.webhook_path}"
                  data-webhook-method="{item.webhook_method}"
                  data-webhook-title="{item.title}"
                  data-webhook-name="{item.name}"
                  data-webhook-kind="{item.trigger_kind}"
                  href="{item.editor_href}"
                >{item.name}</a>
              </div>
            </div>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
