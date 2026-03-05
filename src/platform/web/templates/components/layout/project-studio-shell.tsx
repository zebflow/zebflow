import PlatformSidebar from "@/components/platform-sidebar";
import { initProjectShellBehavior } from "@/components/behavior/project-shell";

export const app = {};

export default function ProjectStudioShell(props) {
  initProjectShellBehavior();
  const nav = props?.nav ?? {};

  return (
<div className="project-studio-shell">
    <input id="project-theme-toggle" type="checkbox" className="project-theme-toggle-input" />

    <div className="project-studio-frame">
      <PlatformSidebar nav={nav} />

      <main className="project-shell-main">
        <header className="project-shell-header">
          <div className="project-shell-header-row">
            <div className="project-shell-breadcrumb">
              <a href="/home" className="project-shell-breadcrumb-home" aria-label="Go to home">
                <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5">
                  <path d="M4 10.5L12 4l8 6.5V20H4z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round"/>
                </svg>
              </a>
              <span className="project-shell-separator">/</span>
              <a href={props?.projectHref ?? "#"} className="project-shell-breadcrumb-link">{props?.projectLabel ?? "Project"}</a>
              <span className="project-shell-separator">/</span>
              <span className="project-shell-breadcrumb-current" data-rwe-breadcrumb>{props?.currentMenu ?? "Workspace"}</span>
            </div>

            <div className="project-shell-tools">
              <label htmlFor="project-theme-toggle" className="project-shell-tool-button" title="Toggle theme">
                <span className="project-shell-tool-icon project-shell-theme-dark">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M20 15.2A8 8 0 118.8 4 6.5 6.5 0 0020 15.2z" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
                  </svg>
                </span>
                <span className="project-shell-tool-icon project-shell-theme-light">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <circle cx="12" cy="12" r="4" stroke="currentColor" strokeWidth="1.8"/>
                    <path d="M12 2v2.5M12 19.5V22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M2 12h2.5M19.5 12H22M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"/>
                  </svg>
                </span>
              </label>

              <details className="project-shell-chat" data-owner={props?.owner ?? ""} data-project={props?.project ?? ""}>
                <summary className="project-shell-tool-button" title="Open assistant">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M8 10h8M8 14h5M7 5h10a3 3 0 013 3v6a3 3 0 01-3 3h-4l-4 3v-3H7a3 3 0 01-3-3V8a3 3 0 013-3z" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
                  </svg>
                </summary>

                <div className="project-shell-chat-panel">
                  <div className="project-shell-chat-head">
                    <p className="project-shell-chat-title">Zebflow Assistant</p>
                    <p className="project-shell-chat-subtitle">Project-aware help for pipelines, templates, and schema.</p>
                  </div>

                  <div className="project-shell-chat-body">
                    <div className="project-shell-chat-thread" data-assistant-thread="true"></div>

                    <div className="project-shell-chat-meta">
                      <label className="project-shell-chat-toggle-label">
                        <input type="checkbox" data-assistant-use-high="true" />
                        <span>Use high model</span>
                      </label>
                      <span className="project-shell-chat-status" data-assistant-status="true">Ready</span>
                    </div>

                    <form className="project-shell-chat-form" data-assistant-form="true">
                      <textarea
                        className="project-shell-chat-input"
                        rows="3"
                        placeholder="Ask about this project..."
                        data-assistant-input="true"
                      ></textarea>
                      <button type="submit" className="project-shell-chat-send" data-assistant-send="true">Send</button>
                    </form>
                  </div>
                </div>
              </details>

              <details className="project-shell-session" data-owner={props?.owner ?? ""} data-project={props?.project ?? ""}>
                <summary className="project-shell-tool-button" title="Remote control session">
                  <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
                    <path d="M8 6h8M6 10h12M9 14h6M11 18h2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"/>
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
                        <input type="checkbox" value="pipelines.read" defaultChecked />
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
                      <input type="text" className="project-shell-session-token-input" readOnly placeholder="Enable session to generate token" />
                      <button type="button" className="project-shell-session-copy-button">Copy</button>
                      <p className="project-shell-session-help">Add in Cursor: URL above + Authorization: Bearer TOKEN</p>
                    </div>

                    <div className="project-shell-session-block project-shell-session-url-block">
                      <p className="project-shell-session-label">MCP URL:</p>
                      <input type="text" className="project-shell-session-url-input" readOnly placeholder="Enable session to get URL" />
                    </div>
                  </div>
                </div>
              </details>
            </div>
          </div>
        </header>

        <section className="project-shell-workspace" data-rwe-outlet>
          {props?.children}
        </section>
      </main>
    </div>
    <script src="/assets/platform/rwe-router.js"></script>
  </div>
  );
}
