import ProjectStudioShell from "@/components/layout/project-studio-shell";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    scripts: [{ type: "module", src: "/assets/platform/project-settings.mjs" }],
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
      currentMenu="Settings"
      owner="{input.owner}"
      project="{input.project}"
    >
      <div className="project-workspace">
        <nav className="project-tab-strip">
          <a zFor="item in input.settings_tabs" href="{item.href}" className="project-tab-link {item.classes}">{item.label}</a>
        </nav>

        <section className="project-workspace-body">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">{input.page_title}</p>
                  <p className="project-content-copy">{input.page_subtitle}</p>
                </div>
              </div>
            </section>

            <section zShow="input.tab_flags.general" className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-2">
                  <a zFor="item in input.cards_general" href="{item.href}" className="project-card block">
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <h3 className="project-card-title">{item.title}</h3>
                        <p className="project-card-copy">{item.description}</p>
                      </div>
                      <span zShow="item.tag" className="project-inline-chip">{item.tag}</span>
                    </div>
                  </a>
                </div>
              </div>
            </section>

            <section zShow="input.tab_flags.policy" className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-2">
                  <a zFor="item in input.cards_policy" href="{item.href}" className="project-card block">
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <h3 className="project-card-title">{item.title}</h3>
                        <p className="project-card-copy">{item.description}</p>
                      </div>
                      <span zShow="item.tag" className="project-inline-chip">{item.tag}</span>
                    </div>
                  </a>
                </div>
              </div>
            </section>

            <section zShow="input.tab_flags.automatons" className="project-content-section">
              <div className="project-content-body">
                <article className="project-settings-panel"
                  data-assistant-settings="true"
                  data-api-config="{input.assistant.api.config}"
                >
                  <header className="project-settings-panel-head">
                    <div>
                      <h3 className="project-card-title">Project Assistant</h3>
                      <p className="project-card-copy">Bind credential profiles for assistant reasoning tiers.</p>
                    </div>
                    <span className="project-inline-chip">Automaton</span>
                  </header>

                  <form className="project-settings-form" data-assistant-settings-form="true">
                    <label className="pipeline-editor-field">
                      <span>High Model</span>
                      <select name="llm_high_credential_id" data-assistant-high="true">
                        <option value="">None</option>
                        <option zFor="item in input.assistant.credentials" value="{item.credential_id}">{item.title} · {item.credential_id}</option>
                      </select>
                      <small className="pipeline-editor-field-help">Planning and decomposition model.</small>
                    </label>

                    <label className="pipeline-editor-field">
                      <span>General Model</span>
                      <select name="llm_general_credential_id" data-assistant-general="true">
                        <option value="">None</option>
                        <option zFor="item in input.assistant.credentials" value="{item.credential_id}">{item.title} · {item.credential_id}</option>
                      </select>
                      <small className="pipeline-editor-field-help">Default model for regular project chat requests.</small>
                    </label>

                    <label className="pipeline-editor-field">
                      <span>Max Steps</span>
                      <input name="max_steps" type="number" min="1" max="1000" value="{input.assistant.config.max_steps}" />
                      <small className="pipeline-editor-field-help">Upper bound for future multi-step agent execution.</small>
                    </label>

                    <label className="pipeline-editor-field">
                      <span>Max Replans</span>
                      <input name="max_replans" type="number" min="0" max="64" value="{input.assistant.config.max_replans}" />
                      <small className="pipeline-editor-field-help">Maximum replanning attempts before stopping.</small>
                    </label>

                    <label className="project-settings-checkbox">
                      <input name="enabled" type="checkbox" value="true" checked="{input.assistant.config.enabled}" />
                      <span>Enable assistant for this project</span>
                    </label>

                    <div className="project-settings-actions">
                      <button type="submit" className="project-inline-chip project-inline-chip-accent" data-assistant-save="true">Save Assistant Config</button>
                      <span className="project-settings-status" data-assistant-status="true">Ready.</span>
                    </div>
                  </form>
                </article>

                <article className="project-settings-panel">
                  <header className="project-settings-panel-head">
                    <div>
                      <h3 className="project-card-title">MCP Session</h3>
                      <p className="project-card-copy">Remote control channel for external agents.</p>
                    </div>
                    <span className="project-inline-chip">{input.mcp.status_label}</span>
                  </header>
                  <div className="project-settings-inline-list">
                    <p className="project-card-copy">Allowed capabilities:</p>
                    <span zFor="item in input.mcp.capabilities" className="project-inline-chip">{item}</span>
                  </div>
                </article>
              </div>
            </section>

            <section zShow="input.tab_flags.libraries" className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-2">
                  <a zFor="item in input.cards_libraries" href="{item.href}" className="project-card block">
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <h3 className="project-card-title">{item.title}</h3>
                        <p className="project-card-copy">{item.description}</p>
                      </div>
                      <span zShow="item.tag" className="project-inline-chip">{item.tag}</span>
                    </div>
                  </a>
                </div>
              </div>
            </section>

            <section zShow="input.tab_flags.nodes" className="project-content-section">
              <div className="project-content-body">
                <div className="project-card-grid cols-2">
                  <a zFor="item in input.cards_nodes" href="{item.href}" className="project-card block">
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <h3 className="project-card-title">{item.title}</h3>
                        <p className="project-card-copy">{item.description}</p>
                      </div>
                      <span zShow="item.tag" className="project-inline-chip">{item.tag}</span>
                    </div>
                  </a>
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
