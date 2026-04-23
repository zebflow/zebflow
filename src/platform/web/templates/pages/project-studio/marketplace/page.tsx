import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { useEffect, useState } from "zeb";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
    links: [{ rel: "stylesheet", href: "/assets/platform/db-suite.css" }],
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

const SOURCE_TYPES = [
  { value: "pipeline_with_dependencies", label: "Pipeline with dependencies", note: "Exports the pipeline plus referenced templates and local imports." },
  { value: "template_with_dependencies", label: "Template with dependencies", note: "Exports the selected TSX/TS/CSS file plus local imports." },
  { value: "folder_files", label: "Folder files", note: "Exports everything recursively under one folder." },
  { value: "project_files", label: "Project files", note: "Exports the full repo workspace." },
];

const DEFAULT_TOKEN_SCOPES = {
  read: true,
  publish: true,
  manage: false,
};

const DEFAULT_EXTERNAL_REPO = {
  repository_id: "zebflow-com",
  title: "Zebflow Marketplace",
  base_url: "https://marketplace.zebflow.com/api",
  remote_owner: "marketplace",
  remote_project: "default",
  read_token: "",
  enabled: true,
};

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
  const tabs = Array.isArray(input?.marketplace_tabs) ? input.marketplace_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const api = input?.marketplace_api ?? {};
  const producer = input?.marketplace_producer ?? {};
  const [packs, setPacks] = useState(Array.isArray(input?.assets) ? input.assets : []);
  const [myPacks, setMyPacks] = useState(Array.isArray(input?.my_assets) ? input.my_assets : []);
  const [tokens, setTokens] = useState(Array.isArray(input?.tokens) ? input.tokens : []);
  const [publishers, setPublishers] = useState(Array.isArray(input?.publishers) ? input.publishers : []);
  const [repositories, setRepositories] = useState(Array.isArray(input?.repositories) ? input.repositories : []);
  const [publishSources, setPublishSources] = useState(Array.isArray(input?.publish_sources) ? input.publish_sources : []);
  const [publishPreview, setPublishPreview] = useState(null);
  const [status, setStatus] = useState("");
  const [issuedToken, setIssuedToken] = useState("");
  const [publishForm, setPublishForm] = useState({
    source_type: "pipeline_with_dependencies",
    source_ref: (input?.publish_sources?.[0]?.source_ref) || "",
    package_id: "",
    version: "0.1.0",
    publisher_id: "",
    title: "",
    description: "",
    visibility: "private",
    tags_csv: "",
  });
  const [tokenForm, setTokenForm] = useState({
    publisher_id: "",
    title: "",
    expires_at: "",
    scopes: DEFAULT_TOKEN_SCOPES,
  });
  const [repoForm, setRepoForm] = useState({
    repository_id: "",
    title: "",
    base_url: "",
    remote_owner: "",
    remote_project: "",
    read_token: "",
    enabled: true,
  });
  const [publisherForm, setPublisherForm] = useState({
    publisher_id: "",
    display_name: "",
    publisher_url: "",
    email: "",
    description: "",
    icon_url: "",
    website_url: "",
    enabled: true,
  });
  const [producerForm, setProducerForm] = useState({
    project_name: input?.project || "",
    password: "",
    enabled: !!producer?.enabled,
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
    const tasks = [requestJson(api.assets), requestJson(api.repositories)];
    if (producer?.enabled) {
      tasks.push(requestJson(api.my_assets), requestJson(api.tokens), requestJson(api.publishers));
    } else {
      tasks.push(Promise.resolve({ items: [] }), Promise.resolve({ items: [] }), Promise.resolve({ items: [] }));
    }
    const [assetsRes, repoRes, myRes, tokenRes, publisherRes] = await Promise.all(tasks);
    setPacks(Array.isArray(assetsRes?.items) ? assetsRes.items : []);
    setMyPacks(Array.isArray(myRes?.items) ? myRes.items : []);
    setTokens(Array.isArray(tokenRes?.items) ? tokenRes.items : []);
    setPublishers(Array.isArray(publisherRes?.items) ? publisherRes.items : []);
    setRepositories(Array.isArray(repoRes?.items) ? repoRes.items : []);
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
    if (!publishers.length) return;
    const first = publishers[0];
    setPublishForm((prev) => (prev.publisher_id ? prev : { ...prev, publisher_id: first.publisher_id }));
    setTokenForm((prev) => (prev.publisher_id ? prev : { ...prev, publisher_id: first.publisher_id }));
  }, [publishers]);

  useEffect(() => {
    refreshPublishSources(publishForm.source_type).catch(() => {});
  }, [publishForm.source_type]);

  useEffect(() => {
    refreshPreview(publishForm.source_type, publishForm.source_ref).catch(() => {});
  }, [publishForm.source_type, publishForm.source_ref]);

  async function publishAsset(event) {
    event?.preventDefault?.();
    showStatus("Publishing asset...");
    setIssuedToken("");
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
          publisher_id: publishForm.publisher_id,
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
      const payload = await requestJson(url, { method: "POST" });
      const result = payload?.result || {};
      showStatus(`Added ${result.files_written || 0} file(s) into ${result.install_root || "project"} workspace`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function createToken(event) {
    event?.preventDefault?.();
    showStatus("Creating token...");
    setIssuedToken("");
    try {
      const scopes = [];
      if (tokenForm.scopes.read) scopes.push("marketplace:read");
      if (tokenForm.scopes.publish) scopes.push("marketplace:publish");
      if (tokenForm.scopes.manage) scopes.push("marketplace:manage");
      const payload = await requestJson(api.tokens, {
        method: "POST",
        body: JSON.stringify({
          publisher_id: tokenForm.publisher_id,
          title: tokenForm.title,
          scopes,
          expires_at: tokenForm.expires_at ? Math.floor(new Date(tokenForm.expires_at).getTime() / 1000) : null,
        }),
      });
      await refresh();
      setIssuedToken(String(payload?.token_value || ""));
      showStatus(`Created token ${payload?.token?.token_id || ""}`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function savePublisher(event) {
    event?.preventDefault?.();
    showStatus("Saving publisher...");
    try {
      await requestJson(api.publishers, {
        method: "POST",
        body: JSON.stringify(publisherForm),
      });
      await refresh();
      setPublisherForm({
        publisher_id: "",
        display_name: "",
        publisher_url: "",
        email: "",
        description: "",
        icon_url: "",
        website_url: "",
        enabled: true,
      });
      showStatus(`Saved publisher ${publisherForm.publisher_id}`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function deletePublisher(publisherId) {
    showStatus(`Removing publisher ${publisherId}...`);
    try {
      await requestJson(`${api.publishers}/${encodeURIComponent(publisherId)}`, { method: "DELETE" });
      await refresh();
      showStatus(`Removed publisher ${publisherId}`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function saveProducerMode(event) {
    event?.preventDefault?.();
    showStatus(producerForm.enabled ? "Enabling producer mode..." : "Disabling producer mode...");
    try {
      const payload = await requestJson(api.producer, {
        method: "POST",
        body: JSON.stringify(producerForm),
      });
      const nextEnabled = !!payload?.marketplace?.producer_enabled;
      setProducerForm((prev) => ({ ...prev, enabled: nextEnabled, password: "" }));
      showStatus(nextEnabled ? "Marketplace producer mode enabled" : "Marketplace producer mode disabled");
      location.reload();
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function revokeToken(tokenId) {
    showStatus(`Revoking ${tokenId}...`);
    try {
      await requestJson(`${api.tokens}/${encodeURIComponent(tokenId)}`, { method: "DELETE" });
      await refresh();
      showStatus(`Revoked ${tokenId}`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function saveRepository(event) {
    event?.preventDefault?.();
    showStatus("Saving repository...");
    try {
      await requestJson(api.repositories, {
        method: "POST",
        body: JSON.stringify(repoForm),
      });
      await refresh();
      showStatus(`Saved repository ${repoForm.repository_id}`);
      setRepoForm({
        repository_id: "",
        title: "",
        base_url: "",
        remote_owner: "",
        remote_project: "",
        read_token: "",
        enabled: true,
      });
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  function applyExternalRepoPreset() {
    setRepoForm({ ...DEFAULT_EXTERNAL_REPO });
    showStatus("Loaded Zebflow Marketplace preset");
  }

  async function deleteRepository(repositoryId) {
    showStatus(`Removing repository ${repositoryId}...`);
    try {
      await requestJson(`${api.repositories}/${encodeURIComponent(repositoryId)}`, { method: "DELETE" });
      await refresh();
      showStatus(`Removed repository ${repositoryId}`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  const selectedSource = publishSources.find((item) => item.source_ref === publishForm.source_ref) || null;
  const selectedType = SOURCE_TYPES.find((item) => item.value === publishForm.source_type) || SOURCE_TYPES[0];
  const selectedPublisher = publishers.find((item) => item.publisher_id === publishForm.publisher_id) || null;
  const selectedTokenPublisher = publishers.find((item) => item.publisher_id === tokenForm.publisher_id) || null;

  return (
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu="Marketplace"
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
                    <p className="project-content-title">Pack Marketplace</p>
                    <p className="project-content-copy">Share reusable pipelines, templates, folders, and project packs through the embedded Zebflow marketplace. Packs are added into your project workspace so you can inspect and edit them freely.</p>
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

                  {issuedToken ? (
                    <div className="rounded-xl border border-emerald-400/40 bg-emerald-500/10 px-4 py-3">
                      <p className="text-sm font-medium text-emerald-100">Copy this token now</p>
                      <code className="mt-2 block break-all text-xs text-emerald-50">{issuedToken}</code>
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
                            <StudioTd>{item?.publisher_display_name || item?.publisher_id || item?.publisher_owner}</StudioTd>
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
                          <tr><StudioTd colSpan={7}>No marketplace packs yet.</StudioTd></tr>
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
                          <p className="text-sm text-ui-text-soft">This publishes the resolved asset pack into the embedded marketplace registry.</p>
                        </div>
                        <Field label="Publisher">
                          <select className="w-full rounded-md border border-ui-border bg-ui-bg px-3 py-2" value={publishForm.publisher_id} onChange={(e) => setPublishForm((prev) => ({ ...prev, publisher_id: e.target.value }))}>
                            <option value="">Select publisher</option>
                            {publishers.filter((item) => item?.enabled !== false).map((item) => (
                              <option key={item.publisher_id} value={item.publisher_id}>{item.display_name || item.publisher_id}</option>
                            ))}
                          </select>
                        </Field>
                        {selectedPublisher ? (
                          <div className="rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3 text-sm">
                            <div className="font-medium text-ui-text">{selectedPublisher.display_name || selectedPublisher.publisher_id}</div>
                            <div className="mt-1 text-ui-text-soft">{selectedPublisher.publisher_url || "-"}</div>
                            <div className="mt-1 text-ui-text-soft">{selectedPublisher.email || "-"}</div>
                          </div>
                        ) : null}
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
                          <Button type="submit" disabled={!publishForm.source_ref || !publishPreview?.entries?.length || !publishForm.publisher_id}>Publish Pack</Button>
                        </div>
                      </div>
                    </form>
                  ) : null}

                  {tabFlags?.tokens ? (
                    <div className="grid gap-4 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
                      <form className="rounded-xl border border-ui-border bg-ui-bg p-4 space-y-4" onSubmit={createToken}>
                        <div>
                          <p className="project-content-subtitle">Create Marketplace Token</p>
                          <p className="text-sm text-ui-text-soft">Use scoped tokens for publish or read access against this Zebflow marketplace authority.</p>
                        </div>
                        <Field label="Publisher">
                          <select className="w-full rounded-md border border-ui-border bg-ui-bg px-3 py-2" value={tokenForm.publisher_id} onChange={(e) => setTokenForm((prev) => ({ ...prev, publisher_id: e.target.value }))}>
                            <option value="">Select publisher</option>
                            {publishers.filter((item) => item?.enabled !== false).map((item) => (
                              <option key={item.publisher_id} value={item.publisher_id}>{item.display_name || item.publisher_id}</option>
                            ))}
                          </select>
                        </Field>
                        {selectedTokenPublisher ? (
                          <div className="rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3 text-sm">
                            <div className="font-medium text-ui-text">{selectedTokenPublisher.display_name || selectedTokenPublisher.publisher_id}</div>
                            <div className="mt-1 text-ui-text-soft">{selectedTokenPublisher.publisher_url || "-"}</div>
                            <div className="mt-1 text-ui-text-soft">{selectedTokenPublisher.email || "-"}</div>
                          </div>
                        ) : null}
                        <Field label="Title">
                          <Input value={tokenForm.title} onInput={(e) => setTokenForm((prev) => ({ ...prev, title: e.target.value }))} placeholder="CI Publisher" />
                        </Field>
                        <Field label="Expires At">
                          <Input type="datetime-local" value={tokenForm.expires_at} onInput={(e) => setTokenForm((prev) => ({ ...prev, expires_at: e.target.value }))} />
                        </Field>
                        <div className="space-y-2">
                          <p className="text-sm font-medium text-ui-text">Scopes</p>
                          {[
                            ["read", "marketplace:read"],
                            ["publish", "marketplace:publish"],
                            ["manage", "marketplace:manage"],
                          ].map(([key, label]) => (
                            <label key={key} className="flex items-center gap-2 text-sm text-ui-text-soft">
                              <input
                                type="checkbox"
                                checked={!!tokenForm.scopes[key]}
                                onChange={(e) =>
                                  setTokenForm((prev) => ({
                                    ...prev,
                                    scopes: { ...prev.scopes, [key]: e.target.checked },
                                  }))
                                }
                              />
                              <span>{label}</span>
                            </label>
                          ))}
                        </div>
                        <div>
                          <Button type="submit" disabled={!tokenForm.publisher_id}>Create Token</Button>
                        </div>
                      </form>

                      <div className="rounded-xl border border-ui-border bg-ui-bg p-4">
                        <div className="mb-4">
                          <p className="project-content-subtitle">Issued Tokens</p>
                        </div>
                        <StudioTable>
                          <StudioThead>
                            <tr>
                              <StudioTh>Token</StudioTh>
                              <StudioTh>Publisher</StudioTh>
                              <StudioTh>Scopes</StudioTh>
                              <StudioTh>Expires</StudioTh>
                              <StudioTh>Status</StudioTh>
                              <StudioTh>Action</StudioTh>
                            </tr>
                          </StudioThead>
                          <tbody>
                            {tokens.map((item, index) => (
                              <tr key={`${item?.token_id ?? "token"}-${index}`}>
                                <StudioTd>{item?.token_id}</StudioTd>
                                <StudioTd>{item?.publisher_display_name || item?.publisher_id || "-"}</StudioTd>
                                <StudioTd>{Array.isArray(item?.scopes) ? item.scopes.join(", ") : "-"}</StudioTd>
                                <StudioTd>{fmtTs(item?.expires_at)}</StudioTd>
                                <StudioTd>{item?.revoked_at ? "revoked" : "active"}</StudioTd>
                                <StudioTd>
                                  {!item?.revoked_at ? (
                                    <Button type="button" variant="ghost" size="sm" onClick={() => revokeToken(item?.token_id)}>
                                      Revoke
                                    </Button>
                                  ) : null}
                                </StudioTd>
                              </tr>
                            ))}
                            {!tokens.length ? (
                              <tr><StudioTd colSpan={6}>No marketplace tokens created yet.</StudioTd></tr>
                            ) : null}
                          </tbody>
                        </StudioTable>
                      </div>
                    </div>
                  ) : null}

                  {tabFlags?.settings ? (
                    <div className="grid gap-4 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
                      <form className="rounded-xl border border-ui-border bg-ui-bg p-4 space-y-4" onSubmit={saveRepository}>
                        <div>
                          <p className="project-content-subtitle">Pack Repositories</p>
                          <p className="mt-2 text-sm text-ui-text-soft">
                            Add another Zebflow marketplace as a repository source. Local packs remain available by default.
                          </p>
                        </div>
                        <div className="rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3">
                          <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                            <div>
                              <p className="text-sm font-medium text-ui-text">Default external marketplace</p>
                              <p className="mt-1 text-xs text-ui-text-soft">Preload the proxied `marketplace.zebflow.com/api` base, then adjust token or remote owner/project if needed. The direct Zebflow form is the full marketplace API base: `http://127.0.0.1:10610/api/projects/superadmin/default/marketplace`.</p>
                            </div>
                            <Button type="button" variant="outline" size="sm" onClick={applyExternalRepoPreset}>
                              Use Zebflow Marketplace
                            </Button>
                          </div>
                        </div>
                        <Field label="Repository ID">
                          <Input value={repoForm.repository_id} onInput={(e) => setRepoForm((prev) => ({ ...prev, repository_id: e.target.value }))} placeholder="lab-b" />
                        </Field>
                        <Field label="Title">
                          <Input value={repoForm.title} onInput={(e) => setRepoForm((prev) => ({ ...prev, title: e.target.value }))} placeholder="Lab B Marketplace" />
                        </Field>
                        <Field label="Base URL">
                          <Input value={repoForm.base_url} onInput={(e) => setRepoForm((prev) => ({ ...prev, base_url: e.target.value }))} placeholder="http://127.0.0.1:10612/api/projects/superadmin/default/marketplace" />
                        </Field>
                        <p className="-mt-2 text-xs text-ui-text-soft">Use the exact marketplace base URL prefix here. Direct form: `http://127.0.0.1:10610/api/projects/superadmin/default/marketplace`. Proxied forms: `https://marketplace.zebflow.com/api`, `https://a.com/market`. Zebflow appends the marketplace endpoints under that prefix.</p>
                        <div className="grid gap-3 md:grid-cols-2">
                          <Field label="Remote Owner">
                            <Input value={repoForm.remote_owner} onInput={(e) => setRepoForm((prev) => ({ ...prev, remote_owner: e.target.value }))} placeholder="superadmin" />
                          </Field>
                          <Field label="Remote Project">
                            <Input value={repoForm.remote_project} onInput={(e) => setRepoForm((prev) => ({ ...prev, remote_project: e.target.value }))} placeholder="default" />
                          </Field>
                        </div>
                        <p className="-mt-2 text-xs text-ui-text-soft">Remote owner and project are only needed when the base URL is a host or proxy prefix. Leave them empty when the base URL already ends at <code>/api/projects/{"{owner}"}/{"{project}"}/marketplace</code>.</p>
                        <Field label="Read Token">
                          <Input value={repoForm.read_token} onInput={(e) => setRepoForm((prev) => ({ ...prev, read_token: e.target.value }))} placeholder="zfmt_..." />
                        </Field>
                        <label className="flex items-center gap-2 text-sm text-ui-text-soft">
                          <input type="checkbox" checked={repoForm.enabled} onChange={(e) => setRepoForm((prev) => ({ ...prev, enabled: e.target.checked }))} />
                          <span>Enabled</span>
                        </label>
                        <div>
                          <Button type="submit">Save Repository</Button>
                        </div>
                      </form>

                      <div className="space-y-4">
                        <div className="rounded-xl border border-ui-border bg-ui-bg p-4">
                          <p className="project-content-subtitle mb-4">Marketplace Producer</p>
                          <p className="text-sm text-ui-text-soft">Producer mode is disabled by default. Only curated superadmin-owned projects may host a marketplace authority.</p>
                          <div className="mt-4 rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3 text-sm">
                            <div className="text-ui-text-soft">Current status</div>
                            <div className="mt-1 font-medium text-ui-text">{producer?.enabled ? "enabled" : "disabled"}</div>
                          </div>
                          {producer?.can_manage ? (
                            <form className="mt-4 space-y-4" onSubmit={saveProducerMode}>
                              <label className="flex items-center gap-2 text-sm text-ui-text-soft">
                                <input type="checkbox" checked={producerForm.enabled} onChange={(e) => setProducerForm((prev) => ({ ...prev, enabled: e.target.checked }))} />
                                <span>Enable producer mode for this project</span>
                              </label>
                              <Field label="Project Slug Confirmation">
                                <Input value={producerForm.project_name} onInput={(e) => setProducerForm((prev) => ({ ...prev, project_name: e.target.value }))} placeholder={input?.project || ""} />
                              </Field>
                              <Field label="Password">
                                <Input type="password" value={producerForm.password} onInput={(e) => setProducerForm((prev) => ({ ...prev, password: e.target.value }))} placeholder="Confirm your password" />
                              </Field>
                              <div>
                                <Button type="submit">{producerForm.enabled ? "Save Producer Mode" : "Keep Producer Disabled"}</Button>
                              </div>
                            </form>
                          ) : (
                            <div className="mt-4 rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3 text-sm text-ui-text-soft">
                              Only curated superadmin-owned projects can enable producer mode.
                            </div>
                          )}
                        </div>

                        <div className="rounded-xl border border-ui-border bg-ui-bg p-4">
                          <p className="project-content-subtitle mb-4">Publishers</p>
                          {producer?.enabled ? (
                            <div className="space-y-4">
                              <form className="space-y-4" onSubmit={savePublisher}>
                                <div className="grid gap-3 md:grid-cols-2">
                                  <Field label="Publisher ID">
                                    <Input value={publisherForm.publisher_id} onInput={(e) => setPublisherForm((prev) => ({ ...prev, publisher_id: e.target.value }))} placeholder="zebflow-official" />
                                  </Field>
                                  <Field label="Display Name">
                                    <Input value={publisherForm.display_name} onInput={(e) => setPublisherForm((prev) => ({ ...prev, display_name: e.target.value }))} placeholder="Zebflow Official" />
                                  </Field>
                                </div>
                                <div className="grid gap-3 md:grid-cols-2">
                                  <Field label="Publisher URL">
                                    <Input value={publisherForm.publisher_url} onInput={(e) => setPublisherForm((prev) => ({ ...prev, publisher_url: e.target.value }))} placeholder="/publishers/zebflow-official" />
                                  </Field>
                                  <Field label="Email">
                                    <Input value={publisherForm.email} onInput={(e) => setPublisherForm((prev) => ({ ...prev, email: e.target.value }))} placeholder="publishers@zebflow.com" />
                                  </Field>
                                </div>
                                <Field label="Description">
                                  <Input value={publisherForm.description} onInput={(e) => setPublisherForm((prev) => ({ ...prev, description: e.target.value }))} placeholder="Official curated publisher" />
                                </Field>
                                <div className="grid gap-3 md:grid-cols-2">
                                  <Field label="Icon URL">
                                    <Input value={publisherForm.icon_url} onInput={(e) => setPublisherForm((prev) => ({ ...prev, icon_url: e.target.value }))} placeholder="https://..." />
                                  </Field>
                                  <Field label="Website URL">
                                    <Input value={publisherForm.website_url} onInput={(e) => setPublisherForm((prev) => ({ ...prev, website_url: e.target.value }))} placeholder="https://..." />
                                  </Field>
                                </div>
                                <label className="flex items-center gap-2 text-sm text-ui-text-soft">
                                  <input type="checkbox" checked={publisherForm.enabled} onChange={(e) => setPublisherForm((prev) => ({ ...prev, enabled: e.target.checked }))} />
                                  <span>Enabled</span>
                                </label>
                                <div>
                                  <Button type="submit" disabled={!publisherForm.publisher_id}>Save Publisher</Button>
                                </div>
                              </form>
                              <StudioTable>
                                <StudioThead>
                                  <tr>
                                    <StudioTh>Publisher</StudioTh>
                                    <StudioTh>URL</StudioTh>
                                    <StudioTh>Email</StudioTh>
                                    <StudioTh>Status</StudioTh>
                                    <StudioTh>Action</StudioTh>
                                  </tr>
                                </StudioThead>
                                <tbody>
                                  {publishers.map((item, index) => (
                                    <tr key={`${item?.publisher_id ?? "publisher"}-${index}`}>
                                      <StudioTd>{item?.display_name || item?.publisher_id}</StudioTd>
                                      <StudioTd>{item?.publisher_url || "-"}</StudioTd>
                                      <StudioTd>{item?.email || "-"}</StudioTd>
                                      <StudioTd>{item?.enabled ? "enabled" : "disabled"}</StudioTd>
                                      <StudioTd>
                                        <Button type="button" variant="ghost" size="sm" onClick={() => deletePublisher(item?.publisher_id)}>
                                          Remove
                                        </Button>
                                      </StudioTd>
                                    </tr>
                                  ))}
                                  {!publishers.length ? (
                                    <tr><StudioTd colSpan={5}>No publishers created yet.</StudioTd></tr>
                                  ) : null}
                                </tbody>
                              </StudioTable>
                            </div>
                          ) : (
                            <div className="rounded-lg border border-ui-border bg-ui-bg-muted/20 px-3 py-3 text-sm text-ui-text-soft">
                              Enable producer mode first to register publishers.
                            </div>
                          )}
                        </div>

                        <div className="rounded-xl border border-ui-border bg-ui-bg p-4">
                        <p className="project-content-subtitle mb-4">Configured Repositories</p>
                        <StudioTable>
                          <StudioThead>
                            <tr>
                              <StudioTh>Repository</StudioTh>
                              <StudioTh>Remote</StudioTh>
                              <StudioTh>Status</StudioTh>
                              <StudioTh>Action</StudioTh>
                            </tr>
                          </StudioThead>
                          <tbody>
                            {repositories.map((item, index) => (
                              <tr key={`${item?.repository_id ?? "repo"}-${index}`}>
                                <StudioTd>{item?.title || item?.repository_id}</StudioTd>
                                <StudioTd>{item?.remote_owner && item?.remote_project ? `${item?.base_url}/${item?.remote_owner}/${item?.remote_project}` : item?.base_url}</StudioTd>
                                <StudioTd>{item?.enabled ? "enabled" : "disabled"}</StudioTd>
                                <StudioTd>
                                  <Button type="button" variant="ghost" size="sm" onClick={() => deleteRepository(item?.repository_id)}>
                                    Remove
                                  </Button>
                                </StudioTd>
                              </tr>
                            ))}
                            {!repositories.length ? (
                              <tr><StudioTd colSpan={4}>No remote repositories configured yet.</StudioTd></tr>
                            ) : null}
                          </tbody>
                        </StudioTable>
                        </div>
                      </div>
                    </div>
                  ) : null}
                </div>
              </section>
            </div>
          </section>
        </div>
      </ProjectStudioShell>
  );
}
