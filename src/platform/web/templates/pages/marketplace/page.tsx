import { useEffect, useMemo, useState } from "zeb";
import ChromeHeader from "@/pages/home/components/chrome-header";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";

export const page = {
  html: { lang: "en" },
  body: { className: "min-h-screen bg-zinc-50 text-gray-900 font-sans" },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "Zebflow Marketplace",
      description: input?.seo?.description ?? "",
    },
  };
}

const DEFAULT_SOURCE = {
  repository_id: "zebflow-com",
  title: "Zebflow Marketplace",
  base_url: "https://market.zebflow.com/api",
  remote_owner: "",
  remote_project: "",
  read_token: "",
  visibility: "public",
  enabled: true,
};

function describeStatus(value) {
  if (value == null) return "Unknown status";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (typeof value === "object") {
    const direct = value.message || value.error || value.detail || value.reason;
    if (direct && direct !== value) return describeStatus(direct);
    try { return JSON.stringify(value); } catch (_) { return String(value); }
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
      throw new Error(describeStatus(payload?.error || payload?.message || `${response.status} ${response.statusText}`));
    }
    return payload;
  });
}

function blankSource() {
  return { ...DEFAULT_SOURCE, repository_id: "", title: "", base_url: "", read_token: "" };
}

function blankPublisher() {
  return {
    publisher_id: "",
    display_name: "",
    description: "",
    publisher_url: "",
    email: "",
    website_url: "",
    icon_url: "",
    enabled: true,
    can_read: true,
    can_publish: true,
    can_manage: false,
    max_packages: 20,
    max_package_bytes: 10485760,
    max_media_files: 8,
    max_image_bytes: 2097152,
  };
}

function fmtBytes(value) {
  const n = Number(value || 0);
  if (!n) return "-";
  if (n >= 1024 * 1024) return `${Math.round(n / 1024 / 1024)} MB`;
  if (n >= 1024) return `${Math.round(n / 1024)} KB`;
  return `${n} B`;
}

function publisherScopes(item) {
  return [
    item?.can_read ? "read" : "",
    item?.can_publish ? "publish" : "",
    item?.can_manage ? "manage" : "",
  ].filter(Boolean);
}

export default function Page(input) {
  const api = input?.marketplace_api ?? {};
  const isSuperadmin = !!input?.is_superadmin;
  const offices = Array.isArray(input?.offices) ? input.offices : [];
  const projects = Array.isArray(input?.projects) ? input.projects : [];
  const [activeTab, setActiveTab] = useState("explore");
  const [sources, setSources] = useState(Array.isArray(input?.repositories) ? input.repositories : []);
  const [apps, setApps] = useState(Array.isArray(input?.remote_apps) ? input.remote_apps : []);
  const [publishers, setPublishers] = useState(Array.isArray(input?.publishers) ? input.publishers : []);
  const [tokens, setTokens] = useState(Array.isArray(input?.tokens) ? input.tokens : []);
  const [service, setService] = useState(input?.service || null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [sourceSettingsOpen, setSourceSettingsOpen] = useState(false);
  const [sourceForm, setSourceForm] = useState(blankSource());
  const [serviceForm, setServiceForm] = useState({
    host_office_id: input?.service?.host_office_id || offices?.[0]?.id || "standalone",
    public_base_url: input?.service?.public_base_url || "",
    password: "",
    enabled: !!input?.service?.enabled,
  });
  const [publisherForm, setPublisherForm] = useState(blankPublisher());
  const [selectedPublisherId, setSelectedPublisherId] = useState((Array.isArray(input?.publishers) ? input.publishers : [])?.[0]?.publisher_id || "");
  const [tokenForm, setTokenForm] = useState({
    publisher_id: "",
    title: "",
    owner: input?.source_owner || input?.owner || "superadmin",
    project: "platform",
    read: true,
    publish: false,
    manage: false,
  });
  const [tokenValue, setTokenValue] = useState("");
  const sourceById = useMemo(() => {
    const out = {};
    sources.forEach((item) => { out[item?.repository_id] = item; });
    return out;
  }, [sources]);
  const tokenTargets = useMemo(() => {
    const base = [{ value: `${input?.source_owner || input?.owner || "superadmin"}/platform`, label: "Platform authority", owner: input?.source_owner || input?.owner || "superadmin", project: "platform" }];
    projects.forEach((item) => {
      base.push({
        value: `${item?.owner || input?.owner || "superadmin"}/${item?.project}`,
        label: item?.title || item?.project,
        owner: item?.owner || input?.owner || "superadmin",
        project: item?.project,
      });
    });
    return base;
  }, [projects]);
  const tokenTargetValue = `${tokenForm.owner}/${tokenForm.project}`;
  const selectedPublisher = useMemo(() => publishers.find((item) => item?.publisher_id === selectedPublisherId) || null, [publishers, selectedPublisherId]);
  const selectedPublisherTokens = useMemo(() => {
    if (!selectedPublisherId) return tokens;
    return tokens.filter((item) => item?.publisher_id === selectedPublisherId);
  }, [tokens, selectedPublisherId]);

  function selectPublisher(item) {
    if (!item) {
      setSelectedPublisherId("");
      setPublisherForm(blankPublisher());
      setTokenForm((prev) => ({ ...prev, publisher_id: "" }));
      return;
    }
    setSelectedPublisherId(item.publisher_id);
    setPublisherForm({ ...blankPublisher(), ...item });
    setTokenForm((prev) => ({ ...prev, publisher_id: item.publisher_id }));
  }

  function startNewPublisher() {
    setSelectedPublisherId("");
    setPublisherForm(blankPublisher());
    setTokenForm((prev) => ({ ...prev, publisher_id: "" }));
  }

  async function reloadExplore() {
    const [sourcePayload, appPayload] = await Promise.all([
      requestJson(api.repositories),
      requestJson(api.assets),
    ]);
    setSources(Array.isArray(sourcePayload?.items) ? sourcePayload.items : []);
    setApps(Array.isArray(appPayload?.items) ? appPayload.items : []);
  }

  async function reloadManage() {
    if (!isSuperadmin) return;
    const [servicePayload, publisherPayload, tokenPayload] = await Promise.all([
      requestJson(api.service),
      requestJson(api.publishers),
      requestJson(api.tokens),
    ]);
    setService(servicePayload?.service || null);
    setPublishers(Array.isArray(publisherPayload?.items) ? publisherPayload.items : []);
    setTokens(Array.isArray(tokenPayload?.items) ? tokenPayload.items : []);
  }

  async function saveSource(e) {
    e.preventDefault();
    setBusy(true);
    setStatus("");
    try {
      await requestJson(api.repositories, { method: "POST", body: JSON.stringify(sourceForm) });
      setSourceForm(blankSource());
      await reloadExplore();
      setStatus("Marketplace source saved");
    } catch (err) {
      setStatus(err?.message || "Failed saving marketplace source");
    } finally {
      setBusy(false);
    }
  }

  async function deleteSource(repositoryId) {
    setBusy(true);
    setStatus("");
    try {
      await requestJson(`${api.repositories}/${encodeURIComponent(repositoryId)}`, { method: "DELETE" });
      await reloadExplore();
      setStatus("Marketplace source deleted");
    } catch (err) {
      setStatus(err?.message || "Failed deleting marketplace source");
    } finally {
      setBusy(false);
    }
  }

  async function installApp(item) {
    setBusy(true);
    setStatus("");
    try {
      await requestJson(api.install, {
        method: "POST",
        body: JSON.stringify({
          repository_id: item?.repository_id,
          package_id: item?.package_id,
          version: item?.latest_version,
        }),
      });
      setStatus("App installed");
    } catch (err) {
      setStatus(err?.message || "Failed installing app");
    } finally {
      setBusy(false);
    }
  }

  async function saveService(e) {
    e.preventDefault();
    setBusy(true);
    setStatus("");
    try {
      const payload = await requestJson(api.service, { method: "POST", body: JSON.stringify(serviceForm) });
      setService(payload?.service || null);
      setServiceForm((prev) => ({ ...prev, password: "" }));
      setStatus(payload?.service?.enabled ? "Marketplace service enabled" : "Marketplace service disabled");
    } catch (err) {
      setStatus(err?.message || "Failed saving marketplace service");
    } finally {
      setBusy(false);
    }
  }

  async function savePublisher(e) {
    e.preventDefault();
    setBusy(true);
    setStatus("");
    try {
      const payload = await requestJson(api.publishers, { method: "POST", body: JSON.stringify(publisherForm) });
      const saved = payload?.publisher || publisherForm;
      setSelectedPublisherId(saved?.publisher_id || publisherForm.publisher_id);
      setPublisherForm({ ...blankPublisher(), ...saved });
      setTokenForm((prev) => ({ ...prev, publisher_id: saved?.publisher_id || publisherForm.publisher_id }));
      await reloadManage();
      setStatus("Publisher saved");
    } catch (err) {
      setStatus(err?.message || "Failed saving publisher");
    } finally {
      setBusy(false);
    }
  }

  async function deletePublisher(publisherId) {
    setBusy(true);
    setStatus("");
    try {
      await requestJson(`${api.publishers}/${encodeURIComponent(publisherId)}`, { method: "DELETE" });
      if (publisherForm.publisher_id === publisherId) startNewPublisher();
      await reloadManage();
      setStatus("Publisher deleted");
    } catch (err) {
      setStatus(err?.message || "Failed deleting publisher");
    } finally {
      setBusy(false);
    }
  }

  async function createToken(e) {
    e.preventDefault();
    setBusy(true);
    setStatus("");
    setTokenValue("");
    try {
      const scopes = [];
      if (tokenForm.read) scopes.push("marketplace:read");
      if (tokenForm.publish) scopes.push("marketplace:publish");
      if (tokenForm.manage) scopes.push("marketplace:manage");
      const payload = await requestJson(api.tokens, {
        method: "POST",
        body: JSON.stringify({
          owner: tokenForm.owner,
          project: tokenForm.project,
          publisher_id: tokenForm.publisher_id,
          title: tokenForm.title,
          scopes,
        }),
      });
      setTokenValue(payload?.token_value || "");
      await reloadManage();
      setStatus("Token created");
    } catch (err) {
      setStatus(err?.message || "Failed creating token");
    } finally {
      setBusy(false);
    }
  }

  async function revokeToken(tokenId) {
    setBusy(true);
    setStatus("");
    try {
      await requestJson(`${api.tokens}/${encodeURIComponent(tokenId)}`, { method: "DELETE" });
      await reloadManage();
      setStatus("Token revoked");
    } catch (err) {
      setStatus(err?.message || "Failed revoking token");
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (activeTab === "explore") reloadExplore().catch(() => {});
    if (activeTab === "manage") reloadManage().catch(() => {});
  }, [activeTab]);

  useEffect(() => {
    if (activeTab !== "manage" || !publishers.length) return;
    if (!selectedPublisherId || !publishers.some((item) => item?.publisher_id === selectedPublisherId)) {
      selectPublisher(publishers[0]);
    }
  }, [activeTab, publishers, selectedPublisherId]);

  return (
    <>
      <ChromeHeader />
      <main className="pb-16 pt-24">
        <section className="mx-auto max-w-6xl px-6">
          <div className="mb-6 flex flex-col gap-4 border-b border-gray-200 pb-5 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Platform Home</p>
              <h1 className="mt-1 text-3xl font-black text-gray-900">Marketplace</h1>
            </div>
            <Button as="a" href="/home" variant="outline">Home</Button>
          </div>

          <div className="mb-5 flex flex-wrap gap-2">
            <Button type="button" variant={activeTab === "explore" ? "primary" : "outline"} onClick={() => setActiveTab("explore")}>
              Explore Apps From Other Marketplace
            </Button>
            {isSuperadmin ? (
              <Button type="button" variant={activeTab === "manage" ? "primary" : "outline"} onClick={() => setActiveTab("manage")}>
                Manage This Platform-Owned Marketplace
              </Button>
            ) : null}
          </div>

          {status ? (
            <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800">{status}</div>
          ) : null}

          {activeTab === "explore" ? (
            <section className="space-y-5">
              <div className="rounded-lg border border-gray-200 bg-white p-5">
                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                  <div>
                    <h2 className="text-lg font-semibold text-gray-900">Explore Apps From Other Marketplace</h2>
                    <p className="mt-1 text-sm text-gray-500">Default source: https://market.zebflow.com/api. Only full app/project packages are shown here.</p>
                  </div>
                  {isSuperadmin ? (
                    <Button type="button" variant="outline" onClick={() => setSourceSettingsOpen(true)}>Settings</Button>
                  ) : null}
                </div>
                <div className="mt-4 flex flex-wrap gap-2 text-xs">
                  {sources.map((item) => (
                    <span key={item?.repository_id} className="rounded-full border border-gray-200 bg-gray-50 px-3 py-1 text-gray-600">
                      {item?.title || item?.repository_id} · {item?.visibility || "public"}
                    </span>
                  ))}
                </div>
              </div>

              {apps.length ? (
                <div className="grid gap-4 md:grid-cols-2">
                  {apps.map((item, index) => {
                    const source = sourceById[item?.repository_id] || {};
                    return (
                      <article key={`${item?.repository_id}-${item?.package_id}-${index}`} className="rounded-lg border border-gray-200 bg-white p-5">
                        <div className="flex items-start justify-between gap-3">
                          <div className="min-w-0">
                            <p className="text-lg font-semibold text-gray-900">{item?.title || item?.package_id}</p>
                            <p className="mt-1 text-sm text-gray-500">{item?.description || "Marketplace app"}</p>
                          </div>
                          <span className="rounded-full border border-gray-200 px-2 py-1 text-[0.7rem] uppercase text-gray-500">App</span>
                        </div>
                        <div className="mt-4 space-y-1 text-xs text-gray-500">
                          <p><span className="font-medium text-gray-700">Source:</span> {source?.title || item?.repository_id}</p>
                          <p><span className="font-medium text-gray-700">Package:</span> {item?.package_id}</p>
                          <p><span className="font-medium text-gray-700">Version:</span> {item?.latest_version || "-"}</p>
                          <p><span className="font-medium text-gray-700">Publisher:</span> {item?.publisher_display_name || item?.publisher_id || "-"}</p>
                        </div>
                        <div className="mt-5">
                          <Button type="button" variant="primary" disabled={busy} onClick={() => installApp(item)}>Install App</Button>
                        </div>
                      </article>
                    );
                  })}
                </div>
              ) : (
                <div className="rounded-lg border border-dashed border-gray-300 bg-white px-4 py-10 text-sm text-gray-500">
                  No app packages found from visible marketplace sources.
                </div>
              )}
            </section>
          ) : (
            <section className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_360px]">
              <div className="space-y-5">
                <section className="rounded-lg border border-gray-200 bg-white p-5">
                  <h2 className="text-lg font-semibold text-gray-900">Marketplace Service</h2>
                  <div className="mt-4 grid gap-3 text-sm md:grid-cols-3">
                    <div className="rounded-lg border border-gray-200 bg-gray-50 px-3 py-3">
                      <p className="text-gray-500">Status</p>
                      <p className="font-medium text-gray-900">{service?.enabled ? "enabled" : "disabled"}</p>
                    </div>
                    <div className="rounded-lg border border-gray-200 bg-gray-50 px-3 py-3">
                      <p className="text-gray-500">Host office</p>
                      <p className="font-medium text-gray-900">{service?.host_office_id || "-"}</p>
                    </div>
                    <div className="rounded-lg border border-gray-200 bg-gray-50 px-3 py-3">
                      <p className="text-gray-500">Public base URL</p>
                      <p className="break-all font-medium text-gray-900">{service?.public_base_url || "-"}</p>
                    </div>
                  </div>
                  <form className="mt-5 grid gap-4 md:grid-cols-2" onSubmit={saveService}>
                    <Field label="Host Office" id="marketplace-service-office">
                      <select id="marketplace-service-office" className="h-10 w-full rounded-lg border border-gray-300 bg-white px-3 text-sm" value={serviceForm.host_office_id} onChange={(e) => setServiceForm((prev) => ({ ...prev, host_office_id: e.target.value }))}>
                        {offices.map((office) => <option key={office?.id} value={office?.id}>{office?.label || office?.id}</option>)}
                      </select>
                    </Field>
                    <Field label="Public Base URL" id="marketplace-service-url">
                      <Input id="marketplace-service-url" value={serviceForm.public_base_url} onInput={(e) => setServiceForm((prev) => ({ ...prev, public_base_url: e.target.value }))} placeholder="https://market.zebflow.com/api" />
                    </Field>
                    <Field label="Password" id="marketplace-service-password">
                      <Input id="marketplace-service-password" type="password" value={serviceForm.password} onInput={(e) => setServiceForm((prev) => ({ ...prev, password: e.target.value }))} required />
                    </Field>
                    <label className="flex items-center gap-2 pt-6 text-sm text-gray-600">
                      <input type="checkbox" checked={serviceForm.enabled} onChange={(e) => setServiceForm((prev) => ({ ...prev, enabled: e.target.checked }))} />
                      <span>Service enabled</span>
                    </label>
                    <div className="md:col-span-2">
                      <Button type="submit" variant="primary" disabled={busy}>Save Service</Button>
                    </div>
                  </form>
                </section>

                <section className="rounded-lg border border-gray-200 bg-white p-5">
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                    <div>
                      <h2 className="text-lg font-semibold text-gray-900">Publisher Management</h2>
                      <p className="mt-1 text-sm text-gray-500">Select a publisher, edit its public profile, and issue scoped access tokens from the same workspace.</p>
                    </div>
                    <Button type="button" variant="outline" onClick={startNewPublisher}>New Publisher</Button>
                  </div>

                  <div className="mt-5 grid gap-5 lg:grid-cols-[300px_minmax(0,1fr)]">
                    <aside className="rounded-lg border border-gray-200 bg-gray-50 p-3">
                      <div className="mb-3 flex items-center justify-between gap-3">
                        <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Publishers</p>
                        <span className="rounded-full border border-gray-200 bg-white px-2 py-1 text-xs text-gray-500">{publishers.length}</span>
                      </div>
                      <div className="space-y-2">
                        {publishers.map((item) => {
                          const scopes = publisherScopes(item);
                          const isSelected = selectedPublisherId === item?.publisher_id;
                          const tokenCount = tokens.filter((token) => token?.publisher_id === item?.publisher_id && !token?.revoked_at).length;
                          return (
                            <button
                              key={item?.publisher_id}
                              type="button"
                              onClick={() => selectPublisher(item)}
                              className={`w-full rounded-lg border bg-white p-3 text-left transition ${isSelected ? "border-gray-900 shadow-sm" : "border-gray-200 hover:border-gray-300"}`}
                            >
                              <div className="flex items-start justify-between gap-3">
                                <div className="min-w-0">
                                  <p className="truncate text-sm font-semibold text-gray-900">{item?.display_name || item?.publisher_id}</p>
                                  <p className="mt-1 truncate text-xs text-gray-500">{item?.publisher_id}</p>
                                </div>
                                <span className={`shrink-0 rounded-full px-2 py-1 text-[0.7rem] ${item?.enabled ? "bg-emerald-50 text-emerald-800" : "bg-gray-100 text-gray-500"}`}>
                                  {item?.enabled ? "active" : "off"}
                                </span>
                              </div>
                              <div className="mt-3 flex flex-wrap gap-1.5">
                                {scopes.map((scope) => <span key={scope} className="rounded-full border border-gray-200 bg-gray-50 px-2 py-0.5 text-[0.7rem] text-gray-600">{scope}</span>)}
                                {!scopes.length ? <span className="text-xs text-gray-400">no scopes</span> : null}
                              </div>
                              <p className="mt-3 text-xs text-gray-500">{tokenCount} active token(s) · {item?.max_packages || "-"} packages</p>
                            </button>
                          );
                        })}
                        {!publishers.length ? (
                          <div className="rounded-lg border border-dashed border-gray-300 bg-white px-3 py-6 text-sm text-gray-500">Create a publisher to issue marketplace access.</div>
                        ) : null}
                      </div>
                    </aside>

                    <div className="space-y-5">
                      <section className="rounded-lg border border-gray-200 bg-gray-50 p-4">
                        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                          <div>
                            <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Publisher Profile</p>
                            <h3 className="mt-1 text-lg font-semibold text-gray-900">{publisherForm.publisher_id ? (publisherForm.display_name || publisherForm.publisher_id) : "New Publisher"}</h3>
                            {selectedPublisher ? (
                              <p className="mt-1 text-sm text-gray-500">{selectedPublisher.email || selectedPublisher.website_url || selectedPublisher.publisher_url || "No contact profile set"}</p>
                            ) : null}
                          </div>
                          {publisherForm.publisher_id ? (
                            <Button type="button" variant="outline" disabled={busy} onClick={() => deletePublisher(publisherForm.publisher_id)}>Delete</Button>
                          ) : null}
                        </div>

                        <form className="mt-4 grid gap-4 md:grid-cols-2" onSubmit={savePublisher}>
                          <Field label="Publisher ID" id="publisher-id"><Input id="publisher-id" value={publisherForm.publisher_id} onInput={(e) => setPublisherForm((prev) => ({ ...prev, publisher_id: e.target.value }))} required /></Field>
                          <Field label="Display Name" id="publisher-name"><Input id="publisher-name" value={publisherForm.display_name} onInput={(e) => setPublisherForm((prev) => ({ ...prev, display_name: e.target.value }))} /></Field>
                          <Field label="Email" id="publisher-email"><Input id="publisher-email" type="email" value={publisherForm.email} onInput={(e) => setPublisherForm((prev) => ({ ...prev, email: e.target.value }))} /></Field>
                          <Field label="Website URL" id="publisher-website"><Input id="publisher-website" value={publisherForm.website_url} onInput={(e) => setPublisherForm((prev) => ({ ...prev, website_url: e.target.value }))} placeholder="https://example.com" /></Field>
                          <Field label="Marketplace Profile URL" id="publisher-profile-url"><Input id="publisher-profile-url" value={publisherForm.publisher_url} onInput={(e) => setPublisherForm((prev) => ({ ...prev, publisher_url: e.target.value }))} placeholder="/publishers/example" /></Field>
                          <Field label="Icon URL" id="publisher-icon-url"><Input id="publisher-icon-url" value={publisherForm.icon_url} onInput={(e) => setPublisherForm((prev) => ({ ...prev, icon_url: e.target.value }))} placeholder="https://example.com/icon.png" /></Field>
                          <Field label="Description" id="publisher-description">
                            <textarea id="publisher-description" className="min-h-24 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm outline-none" value={publisherForm.description} onInput={(e) => setPublisherForm((prev) => ({ ...prev, description: e.target.value }))} />
                          </Field>
                          <div className="grid gap-4 md:grid-cols-2">
                            <Field label="Max Packages" id="publisher-max-packages"><Input id="publisher-max-packages" type="number" value={publisherForm.max_packages} onInput={(e) => setPublisherForm((prev) => ({ ...prev, max_packages: Number(e.target.value || 0) }))} /></Field>
                            <Field label="Max Package Bytes" id="publisher-max-bytes"><Input id="publisher-max-bytes" type="number" value={publisherForm.max_package_bytes} onInput={(e) => setPublisherForm((prev) => ({ ...prev, max_package_bytes: Number(e.target.value || 0) }))} /></Field>
                            <Field label="Max Media Files" id="publisher-max-media"><Input id="publisher-max-media" type="number" value={publisherForm.max_media_files} onInput={(e) => setPublisherForm((prev) => ({ ...prev, max_media_files: Number(e.target.value || 0) }))} /></Field>
                            <Field label="Max Image Bytes" id="publisher-max-image-bytes"><Input id="publisher-max-image-bytes" type="number" value={publisherForm.max_image_bytes} onInput={(e) => setPublisherForm((prev) => ({ ...prev, max_image_bytes: Number(e.target.value || 0) }))} /></Field>
                          </div>
                          <div className="md:col-span-2 flex flex-wrap gap-4 text-sm text-gray-600">
                            {[
                              ["enabled", "Enabled"],
                              ["can_read", "Read"],
                              ["can_publish", "Publish"],
                              ["can_manage", "Manage"],
                            ].map(([key, label]) => (
                              <label key={key} className="flex items-center gap-2">
                                <input type="checkbox" checked={!!publisherForm[key]} onChange={(e) => setPublisherForm((prev) => ({ ...prev, [key]: e.target.checked }))} />
                                <span>{label}</span>
                              </label>
                            ))}
                          </div>
                          <div className="md:col-span-2 flex flex-wrap gap-2">
                            <Button type="submit" variant="primary" disabled={busy}>Save Publisher</Button>
                            <Button type="button" variant="outline" disabled={busy} onClick={startNewPublisher}>Clear</Button>
                          </div>
                        </form>
                      </section>

                      <section className="rounded-lg border border-gray-200 bg-gray-50 p-4">
                        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                          <div>
                            <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Access Tokens</p>
                            <h3 className="mt-1 text-lg font-semibold text-gray-900">{selectedPublisher ? (selectedPublisher.display_name || selectedPublisher.publisher_id) : "All Publishers"}</h3>
                          </div>
                          <span className="rounded-full border border-gray-200 bg-white px-3 py-1 text-xs text-gray-500">{selectedPublisherTokens.length} token(s)</span>
                        </div>

                        <form className="mt-4 grid gap-4 md:grid-cols-2" onSubmit={createToken}>
                          <Field label="Publisher" id="token-publisher">
                            <select id="token-publisher" className="h-10 w-full rounded-lg border border-gray-300 bg-white px-3 text-sm" value={tokenForm.publisher_id} onChange={(e) => {
                              const publisher = publishers.find((item) => item?.publisher_id === e.target.value);
                              setTokenForm((prev) => ({ ...prev, publisher_id: e.target.value }));
                              if (publisher) setSelectedPublisherId(publisher.publisher_id);
                            }} required>
                              <option value="">Select publisher</option>
                              {publishers.map((item) => <option key={item?.publisher_id} value={item?.publisher_id}>{item?.display_name || item?.publisher_id}</option>)}
                            </select>
                          </Field>
                          <Field label="Title" id="token-title"><Input id="token-title" value={tokenForm.title} onInput={(e) => setTokenForm((prev) => ({ ...prev, title: e.target.value }))} required /></Field>
                          <Field label="Target" id="token-target">
                            <select
                              id="token-target"
                              className="h-10 w-full rounded-lg border border-gray-300 bg-white px-3 text-sm"
                              value={tokenTargetValue}
                              onChange={(e) => {
                                const target = tokenTargets.find((item) => item.value === e.target.value) || tokenTargets[0];
                                setTokenForm((prev) => ({ ...prev, owner: target.owner, project: target.project }));
                              }}
                            >
                              {tokenTargets.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
                            </select>
                          </Field>
                          <div className="flex flex-wrap items-center gap-3 pt-6 text-sm text-gray-600">
                            {[
                              ["read", "Read"],
                              ["publish", "Publish"],
                              ["manage", "Manage"],
                            ].map(([key, label]) => (
                              <label key={key} className="flex items-center gap-2">
                                <input type="checkbox" checked={!!tokenForm[key]} onChange={(e) => setTokenForm((prev) => ({ ...prev, [key]: e.target.checked }))} />
                                <span>{label}</span>
                              </label>
                            ))}
                          </div>
                          <div className="md:col-span-2">
                            <Button type="submit" variant="primary" disabled={busy || !publishers.length}>Create Token</Button>
                          </div>
                        </form>

                        {tokenValue ? (
                          <div className="mt-4 rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-xs text-emerald-800">
                            <p className="font-semibold">One-time token</p>
                            <p className="mt-1 break-all">{tokenValue}</p>
                          </div>
                        ) : null}

                        <div className="mt-5 grid gap-2 md:grid-cols-2">
                          {selectedPublisherTokens.map((item) => (
                            <div key={item?.token_id} className="rounded-lg border border-gray-200 bg-white px-3 py-3 text-xs text-gray-500">
                              <div className="flex items-start justify-between gap-3">
                                <div className="min-w-0">
                                  <p className="truncate font-medium text-gray-900">{item?.title}</p>
                                  <p className="mt-1">{item?.owner}/{item?.project}</p>
                                  <p>{(item?.scopes || []).join(", ") || "no scopes"} · {item?.revoked_at ? "revoked" : "active"}</p>
                                </div>
                                <Button type="button" variant="outline" size="sm" disabled={busy || !!item?.revoked_at} onClick={() => revokeToken(item?.token_id)}>Revoke</Button>
                              </div>
                            </div>
                          ))}
                          {!selectedPublisherTokens.length ? <p className="text-sm text-gray-500">No tokens for this selection.</p> : null}
                        </div>
                      </section>
                    </div>
                  </div>
                </section>
              </div>

              <aside className="space-y-5">
                <section className="rounded-lg border border-gray-200 bg-white p-5">
                  <h2 className="text-lg font-semibold text-gray-900">Project Access</h2>
                  <div className="mt-3 space-y-2 text-sm">
                    {projects.map((item) => (
                      <a key={item?.project} href={item?.marketplace_href} className="block rounded-lg border border-gray-200 px-3 py-2 text-gray-700 hover:bg-gray-50">
                        {item?.title || item?.project}
                      </a>
                    ))}
                  </div>
                </section>
              </aside>
            </section>
          )}
        </section>
      </main>

      <Dialog open={sourceSettingsOpen} onOpenChange={setSourceSettingsOpen}>
        <DialogContent>
          <form onSubmit={saveSource}>
            <div className="space-y-4 px-6 pt-6 pb-2">
              <DialogHeader>
                <DialogTitle>External Marketplace Sources</DialogTitle>
                <DialogDescription>Only superadmin can add sources. Public sources are visible to all users; private sources are superadmin-only.</DialogDescription>
              </DialogHeader>
              <div className="grid gap-4 md:grid-cols-2">
                <Field label="Repository ID" id="source-id"><Input id="source-id" value={sourceForm.repository_id} onInput={(e) => setSourceForm((prev) => ({ ...prev, repository_id: e.target.value }))} required /></Field>
                <Field label="Title" id="source-title"><Input id="source-title" value={sourceForm.title} onInput={(e) => setSourceForm((prev) => ({ ...prev, title: e.target.value }))} /></Field>
                <Field label="Base URL" id="source-url"><Input id="source-url" value={sourceForm.base_url} onInput={(e) => setSourceForm((prev) => ({ ...prev, base_url: e.target.value }))} placeholder="https://market.zebflow.com/api" required /></Field>
                <Field label="Read Token" id="source-token"><Input id="source-token" type="password" value={sourceForm.read_token} onInput={(e) => setSourceForm((prev) => ({ ...prev, read_token: e.target.value }))} /></Field>
                <Field label="Visibility" id="source-visibility">
                  <select id="source-visibility" className="h-10 w-full rounded-lg border border-gray-300 bg-white px-3 text-sm" value={sourceForm.visibility} onChange={(e) => setSourceForm((prev) => ({ ...prev, visibility: e.target.value }))}>
                    <option value="public">Public</option>
                    <option value="private">Private</option>
                  </select>
                </Field>
                <label className="flex items-center gap-2 pt-6 text-sm text-gray-600">
                  <input type="checkbox" checked={sourceForm.enabled} onChange={(e) => setSourceForm((prev) => ({ ...prev, enabled: e.target.checked }))} />
                  <span>Enabled</span>
                </label>
              </div>
              <div className="divide-y divide-gray-200 border-t border-b border-gray-200">
                {sources.map((item) => (
                  <div key={item?.repository_id} className="flex items-center justify-between gap-4 py-3 text-sm">
                    <div className="min-w-0">
                      <p className="font-medium text-gray-900">{item?.title || item?.repository_id}</p>
                      <p className="truncate text-xs text-gray-500">{item?.base_url} · {item?.visibility || "public"}</p>
                    </div>
                    <div className="flex gap-2">
                      <Button type="button" size="sm" variant="outline" onClick={() => setSourceForm({ ...DEFAULT_SOURCE, ...item, read_token: "" })}>Edit</Button>
                      <Button type="button" size="sm" variant="outline" onClick={() => deleteSource(item?.repository_id)} disabled={busy || item?.repository_id === "zebflow-com"}>Delete</Button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setSourceSettingsOpen(false)}>Close</Button>
              <Button type="submit" variant="primary" disabled={busy}>Save Source</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
