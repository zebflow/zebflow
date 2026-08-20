import { cx, useState } from "zeb";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import Input from "@/components/ui/input";
import SqlReport from "@/components/package-review/sql-report";
import { ViolationNotice, WarningNotice } from "@/components/package-review/findings";

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
  tags?: string[];
  image_url?: string;
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
  hubReviewTargetFolder,
  setHubReviewTargetFolder,
  hubReviewDirty,
  onAddPack,
  hubInstallReview,
  onRefreshHubReview,
  onConfirmHubAdd,
  onCancelHubAddReview,
  uiInstallReview,
  uiReviewDirty,
  onRefreshUiReview,
  onConfirmUiInstall,
  onCancelUiReview,
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
  hubReviewTargetFolder: string;
  setHubReviewTargetFolder: (value: string) => void;
  hubReviewDirty: boolean;
  onAddPack: (item: HubPackEntry, targetFolder?: string) => void;
  hubInstallReview: any;
  onRefreshHubReview: () => void;
  onConfirmHubAdd: () => void;
  onCancelHubAddReview: () => void;
  uiInstallReview: any;
  uiReviewDirty: boolean;
  onRefreshUiReview: () => void;
  onConfirmUiInstall: () => void;
  onCancelUiReview: () => void;
}) {
  const [kindFilter, setKindFilter] = useState("all");
  const [tagFilter, setTagFilter] = useState("all");
  const normalizedQuery = String(packSearch || "").trim().toLowerCase();
  const browsablePackages = hubPacks.filter((item) => String(item.asset_kind || "") !== "project_bundle");
  const packageTags = Array.from(new Set(
    browsablePackages.flatMap((item) => Array.isArray(item.tags) ? item.tags : []).map((tag) => String(tag || "").trim()).filter(Boolean)
  )).sort();
  const filteredPacks = hubPacks.filter((item) => {
    const kind = String(item.asset_kind || "");
    if (kind === "project_bundle") return false;
    if (kindFilter === "pipeline" && kind !== "pipeline_bundle") return false;
    if (kindFilter === "template" && kind !== "template_bundle") return false;
    if (kindFilter === "folder" && kind !== "folder_bundle") return false;
    if (kindFilter === "node" && kind !== "node_bundle") return false;
    if (tagFilter !== "all") {
      const tags = Array.isArray(item.tags) ? item.tags.map((tag) => String(tag)) : [];
      if (!tags.includes(tagFilter)) return false;
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
      ...(Array.isArray(item.tags) ? item.tags : []),
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return haystack.includes(normalizedQuery);
  });
  const kindChips = [
    { value: "all", label: "All" },
    { value: "pipeline", label: "Pipelines" },
    { value: "template", label: "Templates" },
    { value: "folder", label: "Folders" },
    { value: "node", label: "Nodes" },
  ];

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
            {(["packs", "ui"] as const).map(tab => (
              <button key={tab} type="button"
                className={cx("pipeline-registry-filter-tab", installTab === tab ? "is-active" : "")}
                onClick={() => setInstallTab(tab)}>
                {tab === "ui" ? "UI" : "Browse"}
              </button>
            ))}
          </div>
          {installTab === "ui" ? (
            <div className="install-catalog-tab-panel">
              {uiInstallReview ? (
                <div className="space-y-3 rounded-md border border-ui-border bg-ui-bg-muted/30 p-3">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <p className="m-0 text-sm font-semibold text-ui-text">Built-in UI Policy Review</p>
                      <p className="m-0 mt-1 text-xs text-ui-text-soft">
                        {uiInstallReview.components?.length || 0} component(s) · {uiInstallReview.asset_kind} · risk {uiInstallReview.risk_level}
                      </p>
                    </div>
                    <span className="rounded-full border border-ui-border px-2 py-0.5 text-[10px] uppercase tracking-wide text-ui-text-soft">
                      Add
                    </span>
                  </div>
                  <div className="rounded-md border border-ui-border bg-ui-bg px-2 py-1.5 text-xs text-ui-text-soft">
                    Files will be added under <code>{uiInstallReview.install_root || "pipelines/shared/ui"}</code>.
                  </div>
                  <div className="grid gap-2 text-xs md:grid-cols-2">
                    <ReviewList title="Components" items={uiInstallReview.components} />
                    <ReviewList title="Files added" items={uiInstallReview.files_added} />
                    <ReviewList title="Files skipped" items={uiInstallReview.files_skipped} />
                    <ReviewList title="Files overwritten" items={uiInstallReview.files_overwritten} danger />
                    <ReviewList title="External URLs" items={uiInstallReview.external_urls} danger />
                    <ReviewList title="Database effects" items={uiInstallReview.database_effects} danger />
                    <ReviewList title="Filesystem effects" items={uiInstallReview.filesystem_effects} />
                    <ReviewList title="Outbound connections" items={uiInstallReview.network_effects} danger />
                    <ReviewList title="Runs supplied code" items={uiInstallReview.code_execution} danger />
                  </div>
                  <WarningNotice items={uiInstallReview.warnings} />
                  <div className="flex justify-end gap-2">
                    <Button type="button" size="xs" variant="outline" onClick={onCancelUiReview}>Back</Button>
                    {uiReviewDirty ? (
                      <Button type="button" size="xs" variant="outline" disabled={installing} onClick={onRefreshUiReview}>
                        {installing ? "Reviewing…" : "Update Review"}
                      </Button>
                    ) : null}
                    <Button type="button" size="xs" variant="primary" disabled={installing || uiReviewDirty} onClick={onConfirmUiInstall}>
                      {installing ? "Adding…" : "Confirm Add"}
                    </Button>
                  </div>
                </div>
              ) : (
                <>
                  <p className="text-xs text-body-soft m-0">
                    Select components to add into <code>shared/ui/</code>. Review runs before files are written.
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
                </>
              )}
              {installResult ? <p className="text-xs text-body-soft m-0 shrink-0">{installResult}</p> : null}
            </div>
          ) : (
            <div className="install-catalog-tab-panel">
              {hubInstallReview ? (
                <div className="space-y-3 rounded-md border border-ui-border bg-ui-bg-muted/30 p-3">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <p className="m-0 text-sm font-semibold text-ui-text">Package Policy Review</p>
                      <p className="m-0 mt-1 text-xs text-ui-text-soft">
                        {hubInstallReview.package_id}@{hubInstallReview.version} · {hubInstallReview.asset_kind} · risk {hubInstallReview.risk_level}
                      </p>
                    </div>
                    <span className="rounded-full border border-ui-border px-2 py-0.5 text-[10px] uppercase tracking-wide text-ui-text-soft">
                      Add
                    </span>
                  </div>
                  <label className="pipeline-editor-field">
                    <span>Destination folder</span>
                    <Input
                      value={hubReviewTargetFolder}
                      onInput={(e) => setHubReviewTargetFolder((e?.currentTarget as HTMLInputElement)?.value || "")}
                      placeholder="/functions/blogging/new-asset-folder"
                    />
                  </label>
                  <div className="rounded-md border border-ui-border bg-ui-bg px-2 py-1.5 text-xs text-ui-text-soft">
                    Files will be added under <code>{hubInstallReview.install_root || "."}</code>. Edit the folder and refresh the review before confirming.
                  </div>
                  <div className="grid gap-2 text-xs md:grid-cols-2">
                    <ReviewList title="Files added" items={hubInstallReview.files_added} />
                    <ReviewList title="Files overwritten" items={hubInstallReview.files_overwritten} danger />
                    <ReviewList title="Pipelines" items={hubInstallReview.pipelines_registered} />
                    <ReviewList title="Nodes" items={hubInstallReview.nodes_used} />
                    <ReviewList title="Credentials" items={hubInstallReview.credentials_required} danger />
                    <ReviewList title="External URLs" items={hubInstallReview.external_urls} danger />
                    <ReviewList title="Database effects" items={hubInstallReview.database_effects} danger />
                    <ReviewList title="Filesystem effects" items={hubInstallReview.filesystem_effects} />
                    <ReviewList title="Outbound connections" items={hubInstallReview.network_effects} danger />
                    <ReviewList title="Runs supplied code" items={hubInstallReview.code_execution} danger />
                    <ReviewList title="Public endpoints" items={hubInstallReview.public_endpoints} danger />
                    <ReviewList title="Schedules" items={hubInstallReview.schedules} danger />
                    <ReviewList title="Large files" items={hubInstallReview.large_files} />
                    <ReviewList title="Seed/demo data" items={hubInstallReview.seed_data} />
                  </div>
                  <SqlReport
                    reports={hubInstallReview.database_initialization}
                    emptyNote="This package carries no install-time SQL. Nothing is replayed into any store."
                  />
                  <ViolationNotice items={hubInstallReview.violations} subject="This package" />
                  <WarningNotice items={hubInstallReview.warnings} />
                  <div className="flex justify-end gap-2">
                    <Button type="button" size="xs" variant="outline" onClick={onCancelHubAddReview}>Back</Button>
                    {hubReviewDirty ? (
                      <Button type="button" size="xs" variant="outline" disabled={installing} onClick={onRefreshHubReview}>
                        {installing ? "Reviewing…" : "Update Review"}
                      </Button>
                    ) : null}
                    <Button type="button" size="xs" variant="primary" disabled={installing || hubReviewDirty || hubInstallReview.installable === false} onClick={onConfirmHubAdd}>
                      {installing ? "Adding…" : "Confirm Add"}
                    </Button>
                  </div>
                </div>
              ) : (
              <div className="space-y-3">
                <p className="text-xs text-body-soft m-0">
                  Browse Hub packages and add them into this project workspace. Project bundles are intentionally excluded from Add+.
                </p>
                <div className="flex flex-wrap gap-2">
                  {kindChips.map((chip) => (
                    <button
                      key={chip.value}
                      type="button"
                      className={cx("pipeline-registry-filter-tab", kindFilter === chip.value ? "is-active" : "")}
                      onClick={() => setKindFilter(chip.value)}
                    >
                      {chip.label}
                    </button>
                  ))}
                </div>
                {packageTags.length ? (
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      className={cx("pipeline-registry-filter-tab", tagFilter === "all" ? "is-active" : "")}
                      onClick={() => setTagFilter("all")}
                    >
                      All tags
                    </button>
                    {packageTags.slice(0, 16).map((tag) => (
                      <button
                        key={tag}
                        type="button"
                        className={cx("pipeline-registry-filter-tab", tagFilter === tag ? "is-active" : "")}
                        onClick={() => setTagFilter(tag)}
                      >
                        {tag}
                      </button>
                    ))}
                  </div>
                ) : null}
                <Input
                  value={packSearch}
                  onInput={(e) => setPackSearch((e?.currentTarget as HTMLInputElement)?.value || "")}
                  placeholder="Search packages, publishers, kinds, or tags..."
                />
                <div className="max-h-[440px] overflow-auto rounded-md border border-ui-border bg-ui-bg-muted/20">
                  {filteredPacks.length ? filteredPacks.map((item) => (
                    <div key={`${item.repository_id || "local"}:${item.package_id}:${item.latest_version || ""}`} className="flex items-start gap-3 border-b border-ui-border px-3 py-2 last:border-b-0">
                      {item.image_url ? <img src={item.image_url} alt="" className="h-14 w-20 shrink-0 rounded-md border border-ui-border object-cover" /> : null}
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium text-ui-text">{item.title || item.package_id}</span>
                          <span className="rounded-full border border-ui-border px-2 py-0.5 text-[10px] uppercase tracking-wide text-ui-text-soft">{item.asset_kind}</span>
                        </div>
                        {Array.isArray(item.tags) && item.tags.length ? (
                          <div className="mt-1 flex flex-wrap gap-1">
                            {item.tags.slice(0, 6).map((tag) => (
                              <span key={tag} className="rounded-full border border-ui-border bg-ui-bg px-2 py-0.5 text-[10px] text-ui-text-soft">{tag}</span>
                            ))}
                          </div>
                        ) : null}
                        <div className="mt-1 text-xs text-ui-text-soft">
                          {item.package_id} · {item.latest_version || "-"} · {item.publisher_display_name || item.publisher_id || "-"} · {item.repository_title || "Local"}
                        </div>
                        {item.description ? (
                          <p className="mt-1 text-xs text-ui-text-soft">{item.description}</p>
                        ) : null}
                      </div>
                      <Button type="button" size="xs" variant="ghost" onClick={() => onAddPack(item)}>
                        Add
                      </Button>
                    </div>
                  )) : (
                    <div className="px-3 py-4 text-xs text-ui-text-soft">No matching packages.</div>
                  )}
                </div>
                {installResult ? <p className="text-xs text-body-soft m-0 shrink-0">{installResult}</p> : null}
              </div>
              )}
            </div>
          )}
        </div>
        <div className="git-commit-actions shrink-0">
          {installTab === "ui" && !uiInstallReview ? (
          <Button size="xs" onClick={onInstallSubmit} disabled={installing}>
            {installing ? "Reviewing…" : "Review Selected"}
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

function ReviewList({ title, items, danger = false }: { title: string; items: any[]; danger?: boolean }) {
  const values = Array.isArray(items) ? items.filter(Boolean).slice(0, 8) : [];
  const remaining = Array.isArray(items) ? Math.max(0, items.length - values.length) : 0;
  return (
    <div className={cx("rounded-md border px-2 py-1.5", danger && values.length ? "border-amber-300 bg-amber-50/60" : "border-ui-border bg-ui-bg")}>
      <p className="m-0 text-[10px] font-semibold uppercase tracking-wide text-ui-text-soft">{title}</p>
      {values.length ? (
        <ul className="m-0 mt-1 space-y-0.5 p-0 list-none text-ui-text-soft">
          {values.map((item, index) => <li key={`${title}-${index}`} className="truncate">{String(item)}</li>)}
          {remaining ? <li className="text-ui-text-soft">+{remaining} more</li> : null}
        </ul>
      ) : (
        <p className="m-0 mt-1 text-ui-text-soft">None</p>
      )}
    </div>
  );
}
