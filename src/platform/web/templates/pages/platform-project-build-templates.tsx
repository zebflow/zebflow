import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initTemplateEditorBehavior } from "@/components/behavior/template-editor";
import Sonner from "@/components/ui/sonner";
import TemplateFolderTree from "@/components/ui/template-folder-tree";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    links: [
      { rel: "stylesheet", href: "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css" },
    ],
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
  initTemplateEditorBehavior();
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const workspace = input?.workspace ?? {};
  const workspaceApi = workspace?.api ?? {};
  const selectedFile = workspace?.selected_file ?? {};
  const workspaceItems = Array.isArray(workspace?.items) ? workspace.items : [];
  return (
<Page>
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
          <a href={navLinks.build_templates ?? "#"} className={`project-tab-link ${navClasses.build_templates || ""}`}>Templates</a>
          <a href={navLinks.build_assets ?? "#"} className={`project-tab-link ${navClasses.build_assets || ""}`}>Assets</a>
          <a href={navLinks.build_docs ?? "#"} className={`project-tab-link ${navClasses.build_docs || ""}`}>Docs</a>
        </nav>

        <section className="project-workspace-body">
          <div
            className="template-workspace"
            data-template-workspace="true"
            data-template-api-workspace={workspaceApi.workspace ?? ""}
            data-template-api-file={workspaceApi.file ?? ""}
            data-template-api-save={workspaceApi.save ?? ""}
            data-template-api-create={workspaceApi.create ?? ""}
            data-template-api-move={workspaceApi.move ?? ""}
            data-template-api-delete={workspaceApi.delete ?? ""}
            data-template-api-git-status={workspaceApi.git_status ?? ""}
            data-template-api-diagnostics={workspaceApi.diagnostics ?? ""}
            data-template-selected-file={selectedFile.rel_path ?? ""}
          >
            <aside className="template-sidebar" data-split-target="true">
              <div className="template-sidebar-toolbar">
                <button type="button" className="template-sidebar-icon is-active" data-template-pane-trigger="files" title="Files">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M4 6h7l2 2h7v10H4z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                  </svg>
                </button>
                <button type="button" className="template-sidebar-icon" data-template-pane-trigger="search" title="Search">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <circle cx="11" cy="11" r="6" stroke="currentColor" stroke-width="1.7"/>
                    <path d="M16 16l4 4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                  </svg>
                </button>
                <button type="button" className="template-sidebar-icon" data-template-pane-trigger="git" title="Git">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <circle cx="6" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <circle cx="18" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <circle cx="12" cy="18" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <path d="M8 7l3 9M16 7l-3 9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                  </svg>
                </button>

                <span className="template-sidebar-toolbar-spacer"></span>

                <button type="button" className="template-sidebar-icon" title="New Folder" data-template-new-folder="true">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M4 7h6l2 2h8v8H4z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                    <path d="M12 11v4M10 13h4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                  </svg>
                </button>

                <details className="template-create-menu">
                  <summary className="template-sidebar-icon" title="New">
                    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                      <path d="M12 5v14M5 12h14" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                    </svg>
                  </summary>
                  <div className="template-create-menu-panel">
                    <button type="button" className="template-create-item" data-template-create-kind="page">Page</button>
                    <button type="button" className="template-create-item" data-template-create-kind="component">Component</button>
                    <button type="button" className="template-create-item" data-template-create-kind="script">Script</button>
                  </div>
                </details>
              </div>

              <section className="template-sidebar-pane is-active" data-template-pane="files">
                <div className="template-tree" data-template-tree="true" data-template-root-drop="true">
                  <TemplateFolderTree
                    items={workspaceItems}
                    selectedFile={selectedFile.rel_path ?? ""}
                    selectedFolder=""
                  />
                </div>
              </section>

              <section className="template-sidebar-pane" data-template-pane="search">
                <div className="template-mode-search">
                  <div className="template-search-input-wrap">
                    <input
                      type="search"
                      className="template-search-input"
                      placeholder="Search templates"
                      data-template-search-input="true"
                    />
                  </div>
                  <div className="template-search-results" data-template-search-results="true"></div>
                </div>
              </section>

              <section className="template-sidebar-pane" data-template-pane="git">
                <div className="template-git-status" data-template-git-status="true"></div>
              </section>
            </aside>

            <div className="template-split-handle" data-template-split-handle="true" aria-hidden="true"></div>

            <section className="template-editor-pane">
              <Sonner />
              <div className="template-editor-tabs">
                <div className="template-editor-tab is-active" data-template-editor-tab="true">
                  <span className="template-editor-tab-label">{selectedFile.name}</span>
                  <span className="template-editor-tab-kind">{selectedFile.file_kind}</span>
                </div>
                <div className="template-editor-tab-actions">
                  <button type="button" className="template-editor-action" data-template-save="true">Save</button>
                  <button type="button" className="template-editor-action is-danger" data-template-delete="true" data-template-delete-protected={selectedFile.is_protected ? "true" : "false"}>Delete</button>
                </div>
              </div>

              <div className="template-editor-surface">
                <div className="template-editor-host" data-template-editor-host="true"></div>
                <textarea className="template-editor-source" data-template-editor-source="true" spellCheck={false}>{selectedFile.content}</textarea>
              </div>

              <div className="template-editor-statusbar">
                <div className="template-editor-status-icon" title={selectedFile.rel_path ?? ""} data-template-current-file="true">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                    <path d="M14 4v4h4" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                  </svg>
                  <span className="template-editor-status-value" data-template-current-file-value="true">{selectedFile.name}</span>
                </div>
                <div className="template-editor-status-icon" title="zeb/codemirror@0.1">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M8 6h8M8 10h8M8 14h5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                    <path d="M6 4h12v16H6z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                  </svg>
                  <span className="template-editor-status-value">CM</span>
                </div>
                <div className="template-editor-status-icon" title="zeb/interact@0.1">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M8 7l3 9M16 7l-3 9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                    <circle cx="6" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <circle cx="18" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <circle cx="12" cy="18" r="2" stroke="currentColor" stroke-width="1.7"/>
                  </svg>
                  <span className="template-editor-status-value">IN</span>
                </div>
                <div className="template-editor-status-icon" title="zeb/stateutil@0.1">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M6 12h12M12 6v12" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                  </svg>
                  <span className="template-editor-status-value">ST</span>
                </div>
                <div className="template-editor-status-spacer"></div>
                <div className="template-editor-status-icon" title="Save state">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M6 4h9l3 3v13H6z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                    <path d="M9 4v5h6" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                  </svg>
                  <span className="template-editor-status-value" data-template-save-state="true">Clean</span>
                </div>
                <div className="template-editor-status-icon" title="Git status">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <circle cx="6" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <circle cx="18" cy="6" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <circle cx="12" cy="18" r="2" stroke="currentColor" stroke-width="1.7"/>
                    <path d="M8 7l3 9M16 7l-3 9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                  </svg>
                  <span className="template-editor-status-value" data-template-git-state="true">Synced</span>
                </div>
                <div className="template-editor-status-icon" title="Compile state">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M12 8v5M12 17h.01" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                    <path d="M10.3 4.8L3.8 16a2 2 0 001.73 3h13a2 2 0 001.73-3L13.73 4.8a2 2 0 00-3.46 0z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                  </svg>
                  <span className="template-editor-status-value" data-template-compile-state="true">Unknown</span>
                </div>
                <div className="template-editor-status-icon" title="Editor state">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <circle cx="12" cy="12" r="8" stroke="currentColor" stroke-width="1.7"/>
                    <path d="M12 8v4l2.5 2.5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                  <span className="template-editor-status-value" data-template-status="true">Booting</span>
                </div>
              </div>
            </section>
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
