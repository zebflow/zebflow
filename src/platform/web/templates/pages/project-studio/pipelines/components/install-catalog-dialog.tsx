import Button from "@/components/ui/button";

/**
 * The install catalog modal. Its markup is static: `initInstallCatalogBehavior`
 * opens it, fills the component list and submits, all through the
 * `data-install-*` attributes below.
 */
export default function InstallCatalogDialog() {
  return (
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
            <p style={{ fontSize: "12px", color: "var(--muted-foreground)", margin: 0 }}>
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
            <p data-install-result="true" hidden style={{ fontSize: "12px", color: "var(--muted-foreground)", margin: 0 }} />
          </div>

          {/* Pipelines tab content (future) */}
          <div data-install-tab-content="pipelines" hidden className="install-catalog-tab-panel">
            <p style={{ fontSize: "12px", color: "var(--muted-foreground)", margin: 0 }}>Pipeline templates coming soon.</p>
          </div>

          {/* Scripts tab content (future) */}
          <div data-install-tab-content="scripts" hidden className="install-catalog-tab-panel">
            <p style={{ fontSize: "12px", color: "var(--muted-foreground)", margin: 0 }}>Script templates coming soon.</p>
          </div>
        </div>

        <div className="git-commit-actions shrink-0">
          <Button size="xs" data-install-submit="true">Install Selected</Button>
          <Button variant="outline" size="xs" data-install-catalog-close="true">Cancel</Button>
        </div>
      </div>
    </div>
  );
}
