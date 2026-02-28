export const app = {};

export default function PlatformSidebar(input) {
  return (
<aside className="platform-sidebar-shell fixed left-0 top-0 bottom-0 z-50">
  <input id="platform-sidebar-toggle" type="checkbox" className="platform-sidebar-toggle-input" />

  <div className="platform-sidebar-panel">
    <div className="platform-sidebar-header">
      <a href="/home" className="platform-sidebar-brand">
        <img src="/assets/branding/logo.svg" alt="Zebflow logo" className="w-9 h-9 shrink-0" />
        <span className="platform-sidebar-label">
          <span className="platform-sidebar-brand-title block text-base font-black tracking-tight">ZEBFLOW</span>
          <span className="platform-sidebar-brand-subtitle block text-[10px] font-mono uppercase tracking-[0.18em]">Project</span>
        </span>
      </a>

      <label for="platform-sidebar-toggle" className="platform-sidebar-toggle" aria-label="Toggle sidebar">
        <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
          <path d="M8 5l8 7-8 7" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </label>
    </div>

    <nav className="platform-sidebar-nav">
      <details name="platform-sidebar-groups" data-group="pipelines" className="platform-sidebar-group">
        <summary className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.pipelines}">
          <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
            <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
              <circle cx="7" cy="7" r="2.2" stroke="currentColor" stroke-width="1.6"/>
              <circle cx="17" cy="7" r="2.2" stroke="currentColor" stroke-width="1.6"/>
              <circle cx="12" cy="17" r="2.2" stroke="currentColor" stroke-width="1.6"/>
              <path d="M9.2 8.4l1.9 5.2M14.8 8.4l-1.9 5.2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
            </svg>
          </span>
          <span className="platform-sidebar-label">Pipelines</span>
          <span className="platform-sidebar-group-chevron">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M7 10l5 5 5-5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
        </summary>
        <div className="platform-sidebar-submenu">
          <a href="{input.nav.links.pipelines_registry}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.pipeline_registry}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M5 7h14M5 12h14M5 17h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></span>
            <span className="platform-sidebar-label">Registry</span>
          </a>
          <a href="{input.nav.links.pipelines_webhooks}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.pipeline_webhooks}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M8 8h8v8H8zM6 12H4m16 0h-2M12 6V4m0 16v-2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></span>
            <span className="platform-sidebar-label">Webhooks</span>
          </a>
          <a href="{input.nav.links.pipelines_schedules}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.pipeline_schedules}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><circle cx="12" cy="12" r="7" stroke="currentColor" stroke-width="1.6"/><path d="M12 8v4l3 2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></span>
            <span className="platform-sidebar-label">Schedules</span>
          </a>
          <a href="{input.nav.links.pipelines_functions}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.pipeline_functions}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M7 7h10M10 17l4-10M7 17h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></span>
            <span className="platform-sidebar-label">Functions</span>
          </a>
        </div>
      </details>

      <details name="platform-sidebar-groups" data-group="build" className="platform-sidebar-group">
        <summary className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.build}">
          <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
            <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
              <path d="M5 6h6v6H5zM13 6h6v6h-6zM5 14h6v4H5zM13 14h6v4h-6z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>
            </svg>
          </span>
          <span className="platform-sidebar-label">Build</span>
          <span className="platform-sidebar-group-chevron">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M7 10l5 5 5-5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
        </summary>
        <div className="platform-sidebar-submenu">
          <a href="{input.nav.links.build_templates}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.build_templates}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M6 5h8l4 4v10H6zM14 5v4h4" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg></span>
            <span className="platform-sidebar-label">Templates</span>
          </a>
          <a href="{input.nav.links.build_assets}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.build_assets}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M5 7h14v10H5zM8 13l2-2 2 2 3-3 2 3" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg></span>
            <span className="platform-sidebar-label">Assets</span>
          </a>
          <a href="{input.nav.links.build_schema}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.build_schema}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M7 6h10M7 12h10M7 18h6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></span>
            <span className="platform-sidebar-label">Schema</span>
          </a>
        </div>
      </details>

      <a href="{input.nav.links.dashboard}" className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.dashboard}">
        <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
            <path d="M4 13h6v7H4zM14 4h6v16h-6z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
          </svg>
        </span>
        <span className="platform-sidebar-label">Dashboard</span>
      </a>

      <a href="{input.nav.links.credentials}" className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.credentials}">
        <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
            <path d="M12 14a3 3 0 100-6 3 3 0 000 6zM6 10V8a6 6 0 1112 0v2M5 10h14v9H5z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </span>
        <span className="platform-sidebar-label">Credentials</span>
      </a>

      <details name="platform-sidebar-groups" data-group="tables" className="platform-sidebar-group">
        <summary className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.tables}">
          <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
            <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
              <ellipse cx="12" cy="6" rx="7" ry="3" stroke="currentColor" stroke-width="1.8"/>
              <path d="M5 6v8c0 1.7 3.1 3 7 3s7-1.3 7-3V6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
            </svg>
          </span>
          <span className="platform-sidebar-label">Tables</span>
          <span className="platform-sidebar-group-chevron">
            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
              <path d="M7 10l5 5 5-5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
        </summary>
        <div className="platform-sidebar-submenu">
          <a href="{input.nav.links.tables_connections}" className="platform-sidebar-subitem flex items-center gap-2 px-2 py-1.5 rounded-md text-xs font-mono uppercase tracking-[0.14em] {input.nav.classes.table_connections}">
            <span className="inline-flex items-center justify-center w-4 h-4 shrink-0"><svg viewBox="0 0 24 24" fill="none" className="w-4 h-4"><path d="M7 7h10v10H7zM12 7v10M7 12h10" stroke="currentColor" stroke-width="1.6"/></svg></span>
            <span className="platform-sidebar-label">Connections</span>
          </a>
        </div>
      </details>

      <a href="{input.nav.links.files}" className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.files}">
        <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
            <path d="M4 6h6l2 2h8v10H4z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
          </svg>
        </span>
        <span className="platform-sidebar-label">Files</span>
      </a>

      <a href="{input.nav.links.todo}" className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.todo}">
        <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
            <path d="M8 6h8M8 12h8M8 18h8M4 6h.01M4 12h.01M4 18h.01" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
          </svg>
        </span>
        <span className="platform-sidebar-label">Todo</span>
      </a>

      <a href="{input.nav.links.settings}" className="platform-sidebar-main group flex items-center gap-3 px-3 py-2 rounded-md text-sm {input.nav.classes.settings}">
        <span className="inline-flex items-center justify-center w-5 h-5 shrink-0">
          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
            <path d="M12 15.5a3.5 3.5 0 100-7 3.5 3.5 0 000 7z" stroke="currentColor" stroke-width="1.8"/>
            <path d="M19 12a7 7 0 01-.1 1.1l1.8 1.4-1.8 3.1-2.2-.8a7.3 7.3 0 01-1.9 1.1l-.3 2.3h-3.6l-.3-2.3a7.3 7.3 0 01-1.9-1.1l-2.2.8-1.8-3.1 1.8-1.4A7 7 0 015 12c0-.4 0-.8.1-1.1L3.3 9.5l1.8-3.1 2.2.8c.6-.5 1.2-.9 1.9-1.1l.3-2.3h3.6l.3 2.3c.7.2 1.3.6 1.9 1.1l2.2-.8 1.8 3.1-1.8 1.4c.1.3.1.7.1 1.1z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>
          </svg>
        </span>
        <span className="platform-sidebar-label">Settings</span>
      </a>
    </nav>
  </div>
</aside>
  );
}
