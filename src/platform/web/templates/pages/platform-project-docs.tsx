import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initDocsBehavior } from "@/components/behavior/project-docs";
import Sonner from "@/components/ui/sonner";

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


export default function Page(input) {
  initDocsBehavior();
  const navLinks = input?.nav?.links ?? {};
  const navClasses = input?.nav?.classes ?? {};
  const docs = input?.docs ?? {};
  const docsApi = docs?.api ?? {};
  const docItems = Array.isArray(docs?.items) ? docs.items : [];
  const selectedPath = docs?.selected_path ?? "";
  const selectedContent = docs?.selected_content ?? "";

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
            className="docs-workspace"
            data-docs-workspace="true"
            data-docs-api-list={docsApi.list ?? ""}
            data-docs-api-read={docsApi.read ?? ""}
            data-docs-api-create={docsApi.create ?? ""}
            data-docs-api-agent-list={docsApi.agent_list ?? ""}
            data-docs-api-agent-read={docsApi.agent_read ?? ""}
            data-docs-api-agent-save={docsApi.agent_save ?? ""}
            data-docs-selected-path={selectedPath}
          >
            <aside className="docs-sidebar" data-split-target="true">
              <div className="docs-sidebar-tabs">
                <button type="button" className="docs-tab-btn is-active" data-docs-tab="user" title="User docs (app/docs)">
                  Docs
                </button>
                <button type="button" className="docs-tab-btn" data-docs-tab="agent" title="Agent context docs">
                  Agent
                </button>
              </div>

              {/* User Docs Panel */}
              <div className="docs-sidebar-panel is-active" data-docs-panel="user">
                <div className="docs-sidebar-toolbar">
                  <span className="docs-sidebar-toolbar-label">User Docs</span>
                  <button type="button" className="docs-sidebar-icon" title="New doc" data-docs-new="true">
                    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                      <path d="M12 5v14M5 12h14" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                    </svg>
                  </button>
                </div>
                <div className="docs-file-list" data-docs-file-list="true">
                  {docItems.map((doc, index) => (
                    <a
                      key={`${doc?.path ?? "doc"}-${index}`}
                      href={`?file=${doc?.path ?? ""}`}
                      className={`docs-file-item${(doc?.path ?? "") === selectedPath ? " is-active" : ""}`}
                      data-docs-file={doc?.path ?? ""}
                    >
                      <svg viewBox="0 0 24 24" fill="none" className="w-3 h-3 shrink-0">
                        <path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                      </svg>
                      <span>{doc?.name ?? ""}</span>
                    </a>
                  ))}
                </div>
              </div>

              {/* Agent Docs Panel */}
              <div className="docs-sidebar-panel" data-docs-panel="agent">
                <div className="docs-sidebar-toolbar">
                  <span className="docs-sidebar-toolbar-label">Agent Docs</span>
                </div>
                <div className="docs-file-list" data-docs-agent-list="true">
                  <a className="docs-file-item" data-docs-agent-file="AGENTS.md" href="#">
                    <svg viewBox="0 0 24 24" fill="none" className="w-3 h-3 shrink-0">
                      <path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                    </svg>
                    <span>AGENTS.md</span>
                  </a>
                  <a className="docs-file-item" data-docs-agent-file="SOUL.md" href="#">
                    <svg viewBox="0 0 24 24" fill="none" className="w-3 h-3 shrink-0">
                      <path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                    </svg>
                    <span>SOUL.md</span>
                  </a>
                  <a className="docs-file-item" data-docs-agent-file="MEMORY.md" href="#">
                    <svg viewBox="0 0 24 24" fill="none" className="w-3 h-3 shrink-0">
                      <path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                    </svg>
                    <span>MEMORY.md</span>
                    <span className="docs-badge-readonly">agent</span>
                  </a>
                </div>
                <div className="docs-agent-hint">
                  AGENTS.md and SOUL.md are editable. MEMORY.md is updated automatically by the assistant.
                </div>
              </div>
            </aside>

            <div className="docs-split-handle" data-docs-split-handle="true" aria-hidden="true"></div>

            <section className="docs-editor-pane">
              <Sonner />
              <div className="docs-editor-tabs">
                <div className="docs-editor-tab is-active" data-docs-editor-tab="true">
                  <span className="docs-editor-tab-label" data-docs-current-file-label="true">{selectedPath || "Select a file"}</span>
                </div>
                <div className="docs-editor-tab-actions">
                  <button type="button" className="docs-editor-action" data-docs-save="true" style="display:none">Save</button>
                  <button type="button" className="docs-editor-action is-danger" data-docs-delete="true" style="display:none">Delete</button>
                </div>
              </div>

              <div className="docs-editor-surface">
                <div className="docs-editor-host" data-docs-editor-host="true"></div>
                <textarea className="docs-editor-source" data-docs-editor-source="true" spellCheck={false}>{selectedContent}</textarea>
              </div>

              <div className="docs-editor-statusbar">
                <div className="docs-status-icon" data-docs-current-file-status="true">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M7 4h7l4 4v12H7z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                    <path d="M14 4v4h4" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
                  </svg>
                  <span data-docs-current-file-value="true">{selectedPath || "(none)"}</span>
                </div>
                <div className="docs-status-spacer"></div>
                <div className="docs-status-icon">
                  <span data-docs-save-state="true">Clean</span>
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
