import { cx } from "zeb";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";

const ESSENTIALS = [
  "button", "input", "textarea", "label", "checkbox", "badge", "card", "dialog", "select", "tabs", "separator", "alert",
];

type CatalogEntry = { name: string; installed?: boolean; category?: string };

export function RegistryInstallCatalog({
  onClose,
  installTab,
  setInstallTab,
  catalogData,
  selectedComponents,
  setSelectedComponents,
  installResult,
  installing,
  onInstallSubmit,
}: {
  onClose: () => void;
  installTab: string;
  setInstallTab: (t: string) => void;
  catalogData: CatalogEntry[];
  selectedComponents: Set<string>;
  setSelectedComponents: (s: Set<string> | ((prev: Set<string>) => Set<string>)) => void;
  installResult: string | null;
  installing: boolean;
  onInstallSubmit: () => void;
}) {
  return (
    <div className="git-commit-overlay">
      <div className="git-commit-backdrop" onClick={onClose} />
      <div className="git-commit-box git-commit-box--install-catalog">
        <div className="git-commit-header shrink-0">
          <h3 className="git-commit-title">Install UI Components</h3>
          <Button variant="ghost" size="icon" className="git-commit-close" onClick={onClose} aria-label="Close">✕</Button>
        </div>
        <div className="git-install-catalog-stack">
          <div className="git-install-catalog-tabs">
            {(["ui", "pipelines", "scripts"] as const).map(tab => (
              <button key={tab} type="button"
                className={cx("pipeline-registry-filter-tab", installTab === tab ? "is-active" : "")}
                onClick={() => setInstallTab(tab)}>
                {tab === "ui" ? "UI Kit" : tab === "pipelines" ? "Pipelines" : "Scripts"}
              </button>
            ))}
          </div>
          {installTab === "ui" ? (
            <div className="install-catalog-tab-panel">
              <p className="text-xs text-body-soft m-0">
                Select components to install into <code>shared/ui/</code>.
              </p>
              <div className="flex flex-wrap gap-2 shrink-0">
                <button type="button" className="pipeline-registry-filter-tab"
                  onClick={() => setSelectedComponents(new Set(catalogData.map((c) => c.name)))}>Select All</button>
                <button type="button" className="pipeline-registry-filter-tab"
                  onClick={() => setSelectedComponents(new Set())}>None</button>
                <button type="button" className="pipeline-registry-filter-tab"
                  onClick={() => setSelectedComponents(new Set(ESSENTIALS))}>Essentials</button>
              </div>
              <div className="git-install-component-list-host">
                {catalogData.map((comp) => (
                  <label key={comp.name} className="flex items-center gap-1.5 px-2 py-1.5 rounded-md cursor-pointer text-xs bg-surface-2">
                    <Checkbox
                      checked={selectedComponents.has(comp.name)}
                      onChange={(checked: boolean) => {
                        setSelectedComponents((prev) => {
                          const next = new Set(prev);
                          checked ? next.add(comp.name) : next.delete(comp.name);
                          return next;
                        });
                      }}
                    />
                    <span className="flex-1">{comp.name}</span>
                    {comp.installed && <span className="text-green-500 text-[10px]">✓</span>}
                    <span className="text-[10px] text-body-soft capitalize">{comp.category}</span>
                  </label>
                ))}
              </div>
              {installResult ? <p className="text-xs text-body-soft m-0 shrink-0">{installResult}</p> : null}
            </div>
          ) : (
            <div className="install-catalog-tab-panel">
              <p className="text-xs text-body-soft m-0">Coming soon.</p>
            </div>
          )}
        </div>
        <div className="git-commit-actions shrink-0">
          <Button size="xs" onClick={onInstallSubmit} disabled={installing}>
            {installing ? "Installing…" : "Install Selected"}
          </Button>
          <Button variant="outline" size="xs" type="button" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}
