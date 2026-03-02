import PlatformSidebar from "@/components/platform-sidebar";

export const app = {};

export default function ProjectStudioShell(props) {
  return (
<div className="project-studio-shell">
    <input id="project-theme-toggle" type="checkbox" className="project-theme-toggle-input" />
    <script src="/assets/platform/session-manager.mjs" type="module"></script>

    <div className="project-studio-frame">
      <PlatformSidebar />

      <main className="project-shell-main">
        <header className="project-shell-header">
          <div className="project-shell-header-row">
            <div className="project-shell-breadcrumb">
              <a href="/home" className="project-shell-breadcrumb-home" aria-label="Go to home">
                <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5">
                  <path d="M4 10.5L12 4l8 6.5V20H4z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
                </svg>
              </a>
              <span className="project-shell-separator">/</span>
              <a href="{props.projectHref}" className="project-shell-breadcrumb-link">{props.projectLabel}</a>
              <span className="project-shell-separator">/</span>
              <span className="project-shell-breadcrumb-current">{props.currentMenu}</span>
            </div>

            <div className="project-shell-tools">
              <label for="project-theme-toggle" className="project-shell-tool-button" title="Toggle theme">
                <span className="project-shell-tool-icon project-shell-theme-dark">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M20 15.2A8 8 0 118.8 4 6.5 6.5 0 0020 15.2z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </span>
                <span className="project-shell-tool-icon project-shell-theme-light">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <circle cx="12" cy="12" r="4" stroke="currentColor" stroke-width="1.8"/>
                    <path d="M12 2v2.5M12 19.5V22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M2 12h2.5M19.5 12H22M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
                  </svg>
                </span>
              </label>

              <details className="project-shell-chat">
                <summary className="project-shell-tool-button" title="Open assistant">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M8 10h8M8 14h5M7 5h10a3 3 0 013 3v6a3 3 0 01-3 3h-4l-4 3v-3H7a3 3 0 01-3-3V8a3 3 0 013-3z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </summary>

                <div className="project-shell-chat-panel">
                  <div className="project-shell-chat-head">
                    <p className="project-shell-chat-title">Zebflow Assistant</p>
                    <p className="project-shell-chat-subtitle">Project-aware help for pipelines, templates, and schema.</p>
                  </div>

                  <div className="project-shell-chat-body">
                    <div className="project-shell-chat-block">
                      <p className="project-shell-chat-label">Suggested</p>
                      <ul className="project-shell-chat-list">
                        <li>Inspect the current workspace</li>
                        <li>Explain the selected route or pipeline</li>
                        <li>Draft the next template or schema step</li>
                      </ul>
                    </div>

                    <div className="project-shell-chat-block">
                      <p className="project-shell-chat-label">Runtime</p>
                      <p className="project-shell-chat-copy">
                        This launcher is platform-wide. The actual assistant runtime can be swapped later without changing the shell contract.
                      </p>
                    </div>
                  </div>
                </div>
              </details>

              <details className="project-shell-session" data-owner="{props.owner}" data-project="{props.project}">
                <summary className="project-shell-tool-button" title="Remote control session">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M8 6h8M6 10h12M9 14h6M11 18h2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
                  </svg>
                  <span className="project-shell-session-label">Session</span>
                </summary>

                <div className="project-shell-session-panel">
                  <div className="project-shell-session-head">
                    <p className="project-shell-session-title">MCP Session</p>
                    <p className="project-shell-session-subtitle">Enable per-project remote control for LLM agents (Cursor, etc.)</p>
                  </div>

                  <div className="project-shell-session-body">
                    <div className="project-shell-session-block">
                      <label className="project-shell-session-toggle-label">
                        <input type="checkbox" className="project-shell-session-toggle" />
                        <span>Enable MCP session</span>
                      </label>
                    </div>

                    <div className="project-shell-session-block project-shell-session-operations">
                      <p className="project-shell-session-label">Allowed capabilities:</p>
                      <label className="project-shell-session-checkbox-label">
                        <input type="checkbox" value="pipelines.read" checked />
                        <span>Pipelines Read</span>
                      </label>
                      <label className="project-shell-session-checkbox-label">
                        <input type="checkbox" value="pipelines.write" />
                        <span>Pipelines Write</span>
                      </label>
                      <label className="project-shell-session-checkbox-label">
                        <input type="checkbox" value="pipelines.execute" />
                        <span>Pipelines Execute</span>
                      </label>
                      <label className="project-shell-session-checkbox-label">
                        <input type="checkbox" value="templates.read" />
                        <span>Templates Read</span>
                      </label>
                      <label className="project-shell-session-checkbox-label">
                        <input type="checkbox" value="templates.write" />
                        <span>Templates Write</span>
                      </label>
                      <label className="project-shell-session-checkbox-label">
                        <input type="checkbox" value="credentials.read" />
                        <span>Credentials Read</span>
                      </label>
                      <label className="project-shell-session-checkbox-label">
                        <input type="checkbox" value="tables.read" />
                        <span>Tables Read</span>
                      </label>
                    </div>

                    <div className="project-shell-session-block project-shell-session-token-block">
                      <p className="project-shell-session-label">Token:</p>
                      <input type="text" className="project-shell-session-token-input" readonly placeholder="Enable session to generate token" />
                      <button type="button" className="project-shell-session-copy-button">Copy</button>
                      <p className="project-shell-session-help">Add in Cursor: URL above + Authorization: Bearer TOKEN</p>
                    </div>

                    <div className="project-shell-session-block project-shell-session-url-block">
                      <p className="project-shell-session-label">MCP URL:</p>
                      <input type="text" className="project-shell-session-url-input" readonly placeholder="Enable session to get URL" />
                    </div>
                  </div>
                </div>
              </details>
            </div>
          </div>
        </header>

        <section className="project-shell-workspace">
          {props.children}
        </section>
      </main>
    </div>
  </div>
  );
}
