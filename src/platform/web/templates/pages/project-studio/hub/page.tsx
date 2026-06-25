import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { useEffect, useState } from "zeb";

export const page = {
  head: {
    links: [{ rel: "stylesheet", href: "/assets/platform/db-suite.css" }],
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

const SOURCE_TYPES = [
  { value: "pipeline_with_dependencies", label: "Pipeline with dependencies", note: "Exports the pipeline plus referenced templates and local imports." },
  { value: "template_with_dependencies", label: "Template with dependencies", note: "Exports the selected TSX/TS/CSS file plus local imports." },
  { value: "folder_files", label: "Folder files", note: "Exports everything recursively under one folder." },
  { value: "project_files", label: "Project files", note: "Exports the full repo workspace." },
];

function describeStatus(value) {
  if (value == null) return "Unknown status";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value.map((item) => describeStatus(item)).filter(Boolean).join(", ");
  }
  if (typeof value === "object") {
    const direct = value.message || value.error || value.detail || value.reason;
    if (direct && direct !== value) return describeStatus(direct);
    try {
      return JSON.stringify(value, null, 2);
    } catch (_) {
      return String(value);
    }
  }
  return String(value);
}

function requestJson(url, options = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (response) => {
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      throw new Error(
        describeStatus(payload?.error || payload?.message || `${response.status} ${response.statusText}`),
      );
    }
    return payload;
  });
}

function fmtTs(ts) {
  const n = Number(ts || 0);
  if (!n) return "-";
  const dt = new Date(n * 1000);
  if (Number.isNaN(dt.getTime())) return "-";
  return dt.toISOString().slice(0, 19).replace("T", " ");
}

function slugify(input) {
  return String(input || "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80);
}

function sourceLabel(value) {
  return SOURCE_TYPES.find((item) => item.value === value)?.label || value;
}

export default function Page(input) {
  const tabs = Array.isArray(input?.hub_tabs) ? input.hub_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const api = input?.hub_api ?? {};
  const [packs, setPacks] = useState(Array.isArray(input?.assets) ? input.assets : []);
  const [myPacks, setMyPacks] = useState(Array.isArray(input?.my_assets) ? input.my_assets : []);
  const [publishSources, setPublishSources] = useState(Array.isArray(input?.publish_sources) ? input.publish_sources : []);
  const [publishPreview, setPublishPreview] = useState(null);
  const [status, setStatus] = useState("");
  const [publishForm, setPublishForm] = useState({
    source_type: "pipeline_with_dependencies",
    source_ref: (input?.publish_sources?.[0]?.source_ref) || "",
    package_id: "",
    version: "0.1.0",
    publisher_token: "",
    title: "",
    description: "",
    visibility: "private",
    tags_csv: "",
  });

  function showStatus(value) {
    setStatus(describeStatus(value));
  }

  useEffect(() => {
    if (!status) return;
    const timer = setTimeout(() => setStatus(""), 5000);
    return () => clearTimeout(timer);
  }, [status]);

  async function refresh() {
    const tasks = [requestJson(api.assets), requestJson(api.my_assets)];
    const [assetsRes, myRes] = await Promise.all(tasks);
    setPacks(Array.isArray(assetsRes?.items) ? assetsRes.items : []);
    setMyPacks(Array.isArray(myRes?.items) ? myRes.items : []);
  }

  async function refreshPublishSources(sourceType) {
    const params = new URLSearchParams({ source_type: sourceType });
    const res = await requestJson(`${api.publish_sources}?${params.toString()}`);
    const items = Array.isArray(res?.items) ? res.items : [];
    setPublishSources(items);
    setPublishForm((prev) => {
      const nextSourceRef = items.some((item) => item.source_ref === prev.source_ref) ? prev.source_ref : (items[0]?.source_ref || "");
      const selected = items.find((item) => item.source_ref === nextSourceRef) || items[0];
      return {
        ...prev,
        source_type: sourceType,
        source_ref: nextSourceRef,
        package_id: prev.package_id || slugify(selected?.name || ""),
        title: prev.title || selected?.name || "",
        description: prev.description || selected?.description || "",
      };
    });
  }

  async function refreshPreview(sourceType, sourceRef) {
    if (!sourceRef) {
      setPublishPreview(null);
      return;
    }
    const params = new URLSearchParams({ source_type: sourceType, source_ref: sourceRef });
    const res = await requestJson(`${api.publish_preview}?${params.toString()}`);
    setPublishPreview(res?.preview || null);
  }

  useEffect(() => {
    refresh().catch(() => {});
  }, []);

  useEffect(() => {
    refreshPublishSources(publishForm.source_type).catch(() => {});
  }, [publishForm.source_type]);

  useEffect(() => {
    refreshPreview(publishForm.source_type, publishForm.source_ref).catch(() => {});
  }, [publishForm.source_type, publishForm.source_ref]);

  async function publishAsset(event) {
    event?.preventDefault?.();
    showStatus("Publishing asset...");
    try {
      const payload = await requestJson(api.publish_asset, {
        method: "POST",
        body: JSON.stringify({
          source_type: publishForm.source_type,
          source_ref: publishForm.source_ref,
          package_id: publishForm.package_id,
          version: publishForm.version,
          title: publishForm.title,
          description: publishForm.description,
          publisher_token: publishForm.publisher_token,
          visibility: publishForm.visibility,
          tags: String(publishForm.tags_csv || "").split(",").map((s) => s.trim()).filter(Boolean),
        }),
      });
      await refresh();
      showStatus(`Published ${payload?.package?.package_id || publishForm.package_id}@${payload?.version?.version || publishForm.version}`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function addAsset(item) {
    const packageId = item?.package_id;
    const version = item?.latest_version;
    showStatus(`Adding ${packageId}@${version} to project...`);
    try {
      const url = item?.source === "remote"
        ? `${api.repositories}/${encodeURIComponent(item.repository_id)}/packs/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`
        : `${api.assets}/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`;
      const payload = await requestJson(url, {
        method: "POST",
        body: JSON.stringify({ install_mode: "add_to_current_project" }),
      });
      const result = payload?.result || {};
      showStatus(`Added ${result.files_written || 0} file(s) into ${result.install_root || "project"} workspace`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  const selectedSource = publishSources.find((item) => item.source_ref === publishForm.source_ref) || null;
  const selectedType = SOURCE_TYPES.find((item) => item.value === publishForm.source_type) || SOURCE_TYPES[0];

  return (
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu="Hub"
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          <StudioTabNav>
            {tabs.map((item, index) => (
              <StudioTabLink key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} active={item?.classes === "is-active"}>
                {item?.label}
              </StudioTabLink>
            ))}
          </StudioTabNav>

          <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg">
            <div className="project-content-wrap">
              <section className="project-content-section">
                <div className="project-content-head">
                  <div>
                    <p className="project-content-title">Project Hub</p>
                    <p className="project-content-copy">Install hub packages into this project, or publish from this project with a scoped publisher token. Hub service enablement and office placement live in Home &gt; Hub.</p>
                  </div>
                  <Button type="button" variant="outline" onClick={() => refresh().then(() => showStatus("Refreshed")).catch((err) => showStatus(err?.message || err))}>
                    Refresh
                  </Button>
                </div>
              </section>

              <section className="project-content-section">
                <div className="project-content-body space-y-4">
                  {status ? (
                    <div className="rounded-xl border border-ui-border bg-ui-bg-muted/30 px-4 py-3 text-sm text-ui-text-soft">
                      <span className="font-medium text-ui-text">Status:</span> {status}
                    </div>
                  ) : null}

                  {tabFlags?.packs ? (
                    <StudioTable>
                      <StudioThead>
                        <tr>
                          <StudioTh>Package</StudioTh>
                          <StudioTh>Kind</StudioTh>
                          <StudioTh>Version</StudioTh>
                          <StudioTh>Publisher</StudioTh>
                          <StudioTh>Repository</StudioTh>
                          <StudioTh>Visibility</StudioTh>
                          <StudioTh>Action</StudioTh>
                        </tr>
                      </StudioThead>
                      <tbody>
                        {packs.map((item, index) => (
                          <tr key={`${item?.package_id ?? "asset"}-${index}`}>
                            <StudioTd>{item?.package_id}</StudioTd>
                            <StudioTd>{item?.asset_kind}</StudioTd>
                            <StudioTd>{item?.latest_version || "-"}</StudioTd>
                            <StudioTd>{item?.publisher_display_name || item?.publisher_id || "-"}</StudioTd>
                            <StudioTd>{item?.repository_title || "Local"}</StudioTd>
                            <StudioTd>{item?.visibility}</StudioTd>
                            <StudioTd>
                              <Button type="button" variant="ghost" size="sm" onClick={() => addAsset(item)}>
                                Add
                              </Button>
                            </StudioTd>
                          </tr>
                        ))}
                        {!packs.length ? (
                          <tr><StudioTd colSpan={7}>No hub packages yet.</StudioTd></tr>
                        ) : null}
                      </tbody>
                    </StudioTable>
                  ) : null}

                  {tabFlags?.my_packs ? (
                    <StudioTable>
                      <StudioThead>
                        <tr>
                          <StudioTh>Package</StudioTh>
                          <StudioTh>Title</StudioTh>
                          <StudioTh>Kind</StudioTh>
                          <StudioTh>Version</StudioTh>
                          <StudioTh>Visibility</StudioTh>
                          <StudioTh>Updated</StudioTh>
                        </tr>
                      </StudioThead>
                      <tbody>
                        {myPacks.map((item, index) => (
                          <tr key={`${item?.package_id ?? "mine"}-${index}`}>
                            <StudioTd>{item?.package_id}</StudioTd>
                            <StudioTd>{item?.title}</StudioTd>
                            <StudioTd>{item?.asset_kind}</StudioTd>
                            <StudioTd>{item?.latest_version || "-"}</StudioTd>
                            <StudioTd>{item?.visibility}</StudioTd>
                            <StudioTd>{fmtTs(item?.updated_at)}</StudioTd>
                          </tr>
                        ))}
                        {!myPacks.length ? (
                          <tr><StudioTd colSpan={6}>You have not published any packs yet.</StudioTd></tr>
                        ) : null}
                      </tbody>
                    </StudioTable>
                  ) : null}

                  {tabFlags?.publish ? (
                    <form className="space-y-4" onSubmit={publishAsset}>
                      <div className="rounded-xl border border-ui-border bg-ui-bg p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">1. Choose Source Type</p>
                          <p className="text-sm text-ui-text-soft">Pick the export scope first, then choose the specific item and review the final file tree.</p>
                        </div>
                        <Field label="Source type">
                          <select
                            className="w-full rounded-md border border-ui-border bg-ui-bg px-3 py-2"
                            value={publishForm.source_type}
                            onChange={(e) => setPublishForm((prev) => ({ ...prev, source_type: e.target.value, source_ref: "" }))}
                          >
                            {SOURCE_TYPES.map((item) => (
                              <option key={item.value} value={item.value}>{item.label}</option>
                            ))}
                          </select>
                        </Field>
                        <div className="rounded-lg border border-ui-border bg-ui-bg-muted/30 px-3 py-2 text-sm text-ui-text-soft">
                          <span className="font-medium text-ui-text">{selectedType.label}:</span> {selectedType.note}
                        </div>
                      </div>

                      <div className="rounded-xl border border-ui-border bg-ui-bg p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">2. Select Item</p>
                          <p className="text-sm text-ui-text-soft">Only name, description, and path are shown here. The actual export set is resolved in the preview step below.</p>
                        </div>
                        <Field label="Selected item">
                          <select
                            className="w-full rounded-md border border-ui-border bg-ui-bg px-3 py-2"
                            value={publishForm.source_ref}
                            onChange={(e) => setPublishForm((prev) => ({ ...prev, source_ref: e.target.value }))}
                          >
                            <option value="">Select item</option>
                            {publishSources.map((item, index) => (
                              <option key={`${item?.source_ref ?? "src"}-${index}`} value={item?.source_ref}>
                                {item?.name} · {item?.path}
                              </option>
                            ))}
                          </select>
                        </Field>
                        {selectedSource ? (
                          <div className="grid gap-3 md:grid-cols-3 rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3 text-sm">
                            <div>
                              <div className="text-ui-text-soft uppercase tracking-wider text-[11px]">Name</div>
                              <div className="mt-1 text-ui-text">{selectedSource.name}</div>
                            </div>
                            <div>
                              <div className="text-ui-text-soft uppercase tracking-wider text-[11px]">Description</div>
                              <div className="mt-1 text-ui-text">{selectedSource.description}</div>
                            </div>
                            <div>
                              <div className="text-ui-text-soft uppercase tracking-wider text-[11px]">Path</div>
                              <code className="mt-1 block text-xs text-ui-text">{selectedSource.path}</code>
                            </div>
                          </div>
                        ) : null}
                      </div>

                      <div className="rounded-xl border border-ui-border bg-ui-bg p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">3. Export Tree Preview</p>
                          <p className="text-sm text-ui-text-soft">This is the exact file set that will be packed and published.</p>
                        </div>
                        {publishPreview ? (
                          <>
                            <div className="grid gap-3 md:grid-cols-4 rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3 text-sm">
                              <div>
                                <div className="text-ui-text-soft uppercase tracking-wider text-[11px]">Asset kind</div>
                                <div className="mt-1 text-ui-text">{publishPreview.asset_kind}</div>
                              </div>
                              <div>
                                <div className="text-ui-text-soft uppercase tracking-wider text-[11px]">Source</div>
                                <div className="mt-1 text-ui-text">{sourceLabel(publishPreview.source_type)}</div>
                              </div>
                              <div>
                                <div className="text-ui-text-soft uppercase tracking-wider text-[11px]">Files</div>
                                <div className="mt-1 text-ui-text">{publishPreview.total_files}</div>
                              </div>
                              <div>
                                <div className="text-ui-text-soft uppercase tracking-wider text-[11px]">Bytes</div>
                                <div className="mt-1 text-ui-text">{publishPreview.total_bytes}</div>
                              </div>
                            </div>

                            <div className="rounded-lg border border-ui-border bg-ui-bg-muted/20">
                              <div className="border-b border-ui-border px-3 py-2 text-xs font-mono uppercase tracking-widest text-ui-text-soft">Resolved Files</div>
                              <div className="max-h-80 overflow-auto divide-y divide-ui-border/60">
                                {publishPreview.entries.map((entry, index) => (
                                  <div key={`${entry.rel_path}-${index}`} className="grid gap-2 px-3 py-2 md:grid-cols-[minmax(0,1fr)_140px_90px]">
                                    <div className="min-w-0">
                                      <code className="block truncate text-xs text-ui-text">{entry.rel_path}</code>
                                      <div className="mt-1 text-[11px] text-ui-text-soft">{entry.reason}</div>
                                    </div>
                                    <div className="text-xs text-ui-text-soft">{entry.kind}</div>
                                    <div className="text-right text-xs text-ui-text-soft">{entry.size_bytes}</div>
                                  </div>
                                ))}
                              </div>
                            </div>

                            {publishPreview.warnings?.length ? (
                              <div className="rounded-lg border border-amber-400/30 bg-amber-500/10 px-3 py-3">
                                <p className="text-sm font-medium text-amber-100">Warnings</p>
                                <ul className="mt-2 space-y-1 text-xs text-amber-50">
                                  {publishPreview.warnings.map((item, index) => (
                                    <li key={`${item}-${index}`}>{item}</li>
                                  ))}
                                </ul>
                              </div>
                            ) : null}
                          </>
                        ) : (
                          <div className="rounded-lg border border-dashed border-ui-border px-4 py-6 text-sm text-ui-text-soft">
                            Select a source item to generate the export tree.
                          </div>
                        )}
                      </div>

                      <div className="rounded-xl border border-ui-border bg-ui-bg p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">4. Publish Package</p>
                          <p className="text-sm text-ui-text-soft">Use a publisher token issued from Home &gt; Hub. This project cannot create publishers, tokens, or hub sources.</p>
                        </div>
                        <Field label="Publisher Token">
                          <Input type="password" value={publishForm.publisher_token} onInput={(e) => setPublishForm((prev) => ({ ...prev, publisher_token: e.target.value }))} placeholder="zfmt_..." />
                        </Field>
                        <div className="grid gap-3 md:grid-cols-2">
                          <Field label="Package ID">
                            <Input value={publishForm.package_id} onInput={(e) => setPublishForm((prev) => ({ ...prev, package_id: e.target.value }))} placeholder="ev-charging-demo" />
                          </Field>
                          <Field label="Version">
                            <Input value={publishForm.version} onInput={(e) => setPublishForm((prev) => ({ ...prev, version: e.target.value }))} placeholder="0.1.0" />
                          </Field>
                        </div>
                        <Field label="Title">
                          <Input value={publishForm.title} onInput={(e) => setPublishForm((prev) => ({ ...prev, title: e.target.value }))} placeholder="EV Charging Demo Pipeline" />
                        </Field>
                        <Field label="Description">
                          <Input value={publishForm.description} onInput={(e) => setPublishForm((prev) => ({ ...prev, description: e.target.value }))} placeholder="Short summary" />
                        </Field>
                        <div className="grid gap-3 md:grid-cols-2">
                          <Field label="Visibility">
                            <select className="w-full rounded-md border border-ui-border bg-ui-bg px-3 py-2" value={publishForm.visibility} onChange={(e) => setPublishForm((prev) => ({ ...prev, visibility: e.target.value }))}>
                              <option value="private">private</option>
                              <option value="public">public</option>
                              <option value="unlisted">unlisted</option>
                            </select>
                          </Field>
                          <Field label="Tags">
                            <Input value={publishForm.tags_csv} onInput={(e) => setPublishForm((prev) => ({ ...prev, tags_csv: e.target.value }))} placeholder="ev, mobility, demo" />
                          </Field>
                        </div>
                        <div>
                          <Button type="submit" disabled={!publishForm.source_ref || !publishPreview?.entries?.length || !publishForm.publisher_token}>Publish Pack</Button>
                        </div>
                      </div>
                    </form>
                  ) : null}

                </div>
              </section>
            </div>
          </section>
        </div>
      </ProjectStudioShell>
  );
}
