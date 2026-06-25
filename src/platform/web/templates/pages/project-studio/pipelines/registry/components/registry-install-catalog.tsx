import { cx } from "zeb";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import Input from "@/components/ui/input";

const ESSENTIALS = [
  "button", "input", "textarea", "label", "checkbox", "badge", "card", "dialog", "select", "tabs", "separator", "alert",
];

type CatalogEntry = { name: string; installed?: boolean; category?: string };
type HubPackEntry = {
  package_id: string;
  asset_kind: string;
  latest_version?: string;
  publisher_id?: string;
  publisher_display_name?: string;
  repository_title?: string;
  title?: string;
  description?: string;
  visibility?: string;
  source?: string;
  repository_id?: string;
};

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
  hubPacks,
  packSearch,
  setPackSearch,
  hubInstallMode,
  setHubInstallMode,
  onAddPack,
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
  hubPacks: HubPackEntry[];
  packSearch: string;
  setPackSearch: (value: string) => void;
  hubInstallMode: string;
  setHubInstallMode: (value: string) => void;
  onAddPack: (item: HubPackEntry, installMode: string) => void;
}) {
  const normalizedQuery = String(packSearch || "").trim().toLowerCase();
  const filteredPacks = hubPacks.filter((item) => {
    if (installTab === "pipelines" && !String(item.asset_kind || "").includes("pipeline")) return false;
    if (installTab === "templates" && !String(item.asset_kind || "").includes("template")) return false;
    if (installTab === "packs" && String(item.asset_kind || "") === "template_bundle") return true;
    if (installTab === "packs" && String(item.asset_kind || "").includes("pipeline")) return true;
    if (installTab === "packs" && !["project_bundle", "folder_bundle", "component_pack", "sekejap_pack", "mapserver_pack"].includes(String(item.asset_kind || ""))) {
      return false;
    }
    if (!normalizedQuery) return true;
    const haystack = [
      item.package_id,
      item.asset_kind,
      item.title,
      item.description,
      item.publisher_display_name,
      item.publisher_id,
      item.repository_title,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return haystack.includes(normalizedQuery);
  });

  return (
    <div className="git-commit-overlay">
      <div className="git-commit-backdrop" onClick={onClose} />
      <div className="git-commit-box git-commit-box--install-catalog">
        <div className="git-commit-header shrink-0">
          <h3 className="git-commit-title">Add+</h3>
          <Button variant="ghost" size="icon" className="git-commit-close" onClick={onClose} aria-label="Close">✕</Button>
        </div>
        <div className="git-install-catalog-stack">
          <div className="git-install-catalog-tabs">
            {(["packs", "pipelines", "templates", "ui"] as const).map(tab => (
              <button key={tab} type="button"
                className={cx("pipeline-registry-filter-tab", installTab === tab ? "is-active" : "")}
                onClick={() => setInstallTab(tab)}>
                {tab === "ui" ? "UI" : tab === "pipelines" ? "Pipelines" : tab === "templates" ? "Templates" : "Packs"}
              </button>
            ))}
          </div>
          {installTab === "ui" ? (
            <div className="install-catalog-tab-panel">
              <p className="text-xs text-body-soft m-0">
                Select components to add into <code>shared/ui/</code>.
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
              <div className="space-y-3">
                <p className="text-xs text-body-soft m-0">
                  Browse hub packs and choose how they enter this project workspace.
                </p>
                <label className="pipeline-editor-field">
                  <span>Mode</span>
                  <select
                    value={hubInstallMode}
                    onChange={(e) => setHubInstallMode((e?.currentTarget as HTMLSelectElement)?.value || "add_to_current_project")}
                  >
                    <option value="add_to_current_project">Add to current project</option>
                    <option value="clone_as_folder">Clone as folder</option>
                  </select>
                </label>
                <Input
                  value={packSearch}
                  onInput={(e) => setPackSearch((e?.currentTarget as HTMLInputElement)?.value || "")}
                  placeholder={`Search ${installTab}...`}
                />
                <div className="max-h-[440px] overflow-auto rounded-md border border-ui-border bg-ui-bg-muted/20">
                  {filteredPacks.length ? filteredPacks.map((item) => (
                    <div key={`${item.repository_id || "local"}:${item.package_id}:${item.latest_version || ""}`} className="flex items-start gap-3 border-b border-ui-border px-3 py-2 last:border-b-0">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium text-ui-text">{item.title || item.package_id}</span>
                          <span className="rounded-full border border-ui-border px-2 py-0.5 text-[10px] uppercase tracking-wide text-ui-text-soft">{item.asset_kind}</span>
                        </div>
                        <div className="mt-1 text-xs text-ui-text-soft">
                          {item.package_id} · {item.latest_version || "-"} · {item.publisher_display_name || item.publisher_id || "-"} · {item.repository_title || "Local"}
                        </div>
                        {item.description ? (
                          <p className="mt-1 text-xs text-ui-text-soft">{item.description}</p>
                        ) : null}
                      </div>
                      <Button type="button" size="xs" variant="ghost" onClick={() => onAddPack(item, hubInstallMode)}>
                        {hubInstallMode === "clone_as_folder" ? "Clone" : "Add"}
                      </Button>
                    </div>
                  )) : (
                    <div className="px-3 py-4 text-xs text-ui-text-soft">No matching packs.</div>
                  )}
                </div>
                {installResult ? <p className="text-xs text-body-soft m-0 shrink-0">{installResult}</p> : null}
              </div>
            </div>
          )}
        </div>
        <div className="git-commit-actions shrink-0">
          {installTab === "ui" ? (
          <Button size="xs" onClick={onInstallSubmit} disabled={installing}>
            {installing ? "Adding…" : "Add Selected"}
          </Button>
          ) : null}
          <Button variant="outline" size="xs" type="button" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}
