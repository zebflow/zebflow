import { Link, useEffect, useState } from "zeb";
import ChromeHeader from "@/pages/home/components/chrome-header";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import Field from "@/components/ui/field";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";

const GITLAB_TOKEN_HELP =
  "In GitLab: User Settings → Access Tokens. Create a token with read_repository + write_repository scopes.";
const GITHUB_TOKEN_HELP =
  "In GitHub: Settings → Developer settings → Personal access tokens → Tokens (classic). Select the repo scope.";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export default function Page(input) {
  const initialProjects = Array.isArray(input?.projects) ? input.projects : [];
  const offices = Array.isArray(input?.offices) ? input.offices : [];
  const marketplaceApi = input?.marketplace_api ?? {};
  const runtimeTargets = Array.isArray(input?.runtime_targets)
    ? input.runtime_targets
    : [{ value: "local", label: "Local office", description: "" }];

  const [projects, setProjects] = useState(initialProjects);
  const [createOpen, setCreateOpen] = useState(false);
  const [cloneOpen, setCloneOpen] = useState(false);
  const [marketplaceOpen, setMarketplaceOpen] = useState(false);
  const [provider, setProvider] = useState("gitlab");
  const [projectSlug, setProjectSlug] = useState("");
  const [createBranch, setCreateBranch] = useState("main");
  const [createRuntimeMode, setCreateRuntimeMode] = useState("shared");
  const [createPlacementWorker, setCreatePlacementWorker] = useState("local");
  const [remoteBranch, setRemoteBranch] = useState("main");
  const [localBranch, setLocalBranch] = useState("main");
  const [cloneRuntimeMode, setCloneRuntimeMode] = useState("shared");
  const [clonePlacementWorker, setClonePlacementWorker] = useState("local");
  const [marketplaceSources, setMarketplaceSources] = useState([] as any[]);
  const [marketplaceApps, setMarketplaceApps] = useState([] as any[]);
  const [marketplaceBusy, setMarketplaceBusy] = useState(false);
  const [marketplaceStatus, setMarketplaceStatus] = useState("");
  const [installingId, setInstallingId] = useState("");
  const [marketplaceView, setMarketplaceView] = useState("settings");
  const [sourceForm, setSourceForm] = useState({
    repository_id: "",
    title: "",
    base_url: "",
    remote_owner: "",
    remote_project: "",
    read_token: "",
    enabled: true,
  });

  const openCloneDialog = () => {
    setProvider("gitlab");
    setProjectSlug("");
    setRemoteBranch("main");
    setLocalBranch("main");
    setCloneRuntimeMode("shared");
    setClonePlacementWorker("local");
    setCloneOpen(true);
  };

  const handleRemoteBranchInput = (e) => {
    const val = e.target.value;
    if (localBranch === remoteBranch) setLocalBranch(val);
    setRemoteBranch(val);
  };

  // Auto-derive project slug from the last path segment of the repo URL
  const handleRepoUrlInput = (e) => {
    const url = e.target.value.trim();
    if (!url) { setProjectSlug(""); return; }
    try {
      const clean = url.replace(/\.git$/, "").replace(/\/$/, "");
      const parts = clean.split("/");
      const last = parts[parts.length - 1] || "";
      setProjectSlug(last.toLowerCase().replace(/[^a-z0-9-_]/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, ""));
    } catch (_) {}
  };

  const tokenHelp = provider === "github" ? GITHUB_TOKEN_HELP : GITLAB_TOKEN_HELP;

  function describeStatus(value) {
    if (value == null) return "Unknown status";
    if (typeof value === "string") return value;
    if (typeof value === "number" || typeof value === "boolean") return String(value);
    if (Array.isArray(value)) return value.map((item) => describeStatus(item)).filter(Boolean).join(", ");
    if (typeof value === "object") {
      const direct = value.message || value.error || value.detail || value.reason;
      if (direct && direct !== value) return describeStatus(direct);
      try {
        return JSON.stringify(value);
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
        throw new Error(describeStatus(payload?.error || payload?.message || `${response.status} ${response.statusText}`));
      }
      return payload;
    });
  }

  async function loadMarketplace() {
    if (!marketplaceApi?.repositories || !marketplaceApi?.assets) return;
    setMarketplaceBusy(true);
    try {
      const [reposPayload, assetsPayload] = await Promise.all([
        requestJson(marketplaceApi.repositories),
        requestJson(marketplaceApi.assets),
      ]);
      setMarketplaceSources(Array.isArray(reposPayload?.items) ? reposPayload.items : []);
      setMarketplaceApps(Array.isArray(assetsPayload?.items) ? assetsPayload.items : []);
      setMarketplaceStatus("");
    } catch (err) {
      setMarketplaceStatus(err?.message || "Failed loading marketplace");
    } finally {
      setMarketplaceBusy(false);
    }
  }

  useEffect(() => {
    if (marketplaceOpen) loadMarketplace();
  }, [marketplaceOpen]);

  useEffect(() => {
    if (!marketplaceOpen) return;
    if (marketplaceView !== "settings" && !marketplaceSources.some((item) => item?.repository_id === marketplaceView)) {
      setMarketplaceView("settings");
    }
  }, [marketplaceOpen, marketplaceSources, marketplaceView]);

  async function handleSaveMarketplaceSource(e) {
    e.preventDefault();
    setMarketplaceBusy(true);
    try {
      await requestJson(marketplaceApi.repositories, {
        method: "POST",
        body: JSON.stringify(sourceForm),
      });
      setSourceForm({
        repository_id: "",
        title: "",
        base_url: "",
        remote_owner: "",
        remote_project: "",
        read_token: "",
        enabled: true,
      });
      await loadMarketplace();
    } catch (err) {
      setMarketplaceStatus(err?.message || "Failed saving marketplace source");
      setMarketplaceBusy(false);
    }
  }

  async function handleDeleteMarketplaceSource(repositoryId) {
    setMarketplaceBusy(true);
    try {
      await requestJson(`${marketplaceApi.repositories}/${encodeURIComponent(repositoryId)}`, {
        method: "DELETE",
      });
      await loadMarketplace();
    } catch (err) {
      setMarketplaceStatus(err?.message || "Failed deleting marketplace source");
      setMarketplaceBusy(false);
    }
  }

  async function handleInstallMarketplaceApp(item) {
    const installKey = `${item?.repository_id || ""}:${item?.package_id || ""}:${item?.latest_version || ""}`;
    setInstallingId(installKey);
    try {
      const payload = await requestJson(marketplaceApi.install, {
        method: "POST",
        body: JSON.stringify({
          repository_id: item?.repository_id,
          package_id: item?.package_id,
          version: item?.latest_version,
        }),
      });
      if (payload?.project) {
        setProjects((prev) => {
          const next = [payload.project, ...prev.filter((entry) => entry?.project !== payload.project?.project)];
          return next;
        });
      }
      setMarketplaceOpen(false);
      setMarketplaceStatus("");
    } catch (err) {
      setMarketplaceStatus(err?.message || "Failed installing app");
    } finally {
      setInstallingId("");
    }
  }

  return (
    <>
      <ChromeHeader />

      <main className="pb-16 pt-24">
        <section className="mx-auto max-w-6xl px-6">
          <header className="mb-10 flex flex-col gap-4 border-b border-gray-200 pb-4 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h1 className="text-3xl font-black tracking-tighter text-gray-900">
                Projects for {input.owner}
              </h1>
              <p className="mt-2 text-sm text-gray-500">
                Create and manage automation projects inside this office.
              </p>
              {input?.app_version ? (
                <p className="mt-1 text-[0.7rem] text-gray-400 tracking-wide">v{input.app_version}</p>
              ) : null}
            </div>
            <div className="flex shrink-0 flex-wrap gap-2">
              <Button type="button" variant="primary" onClick={() => setCreateOpen(true)}>
                Create project
              </Button>
              <Button type="button" variant="outline" onClick={openCloneDialog}>
                Clone project
              </Button>
              <Button type="button" variant="outline" onClick={() => setMarketplaceOpen(true)}>
                Marketplace
              </Button>
            </div>
          </header>

          <section className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
            {projects.map((item, index) => (
              <Card key={`${item?.project ?? "project"}-${index}`} className="transition-all hover:border-gray-300 hover:shadow-md">
                <CardContent className="py-5">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <CardTitle className="text-lg">{item?.title}</CardTitle>
                      <CardDescription className="mt-1">{item?.project}</CardDescription>
                    </div>
                    {item?.is_app ? (
                      <span className="inline-flex rounded-full bg-emerald-50 px-2 py-1 text-[0.7rem] font-semibold uppercase tracking-wide text-emerald-700">
                        App
                      </span>
                    ) : null}
                  </div>
                  <div className="mt-4 space-y-1 text-xs text-gray-500">
                    <p>
                      <span className="font-medium text-gray-700">Runtime:</span>{" "}
                      {item?.runtime_mode || "shared"} · {item?.runtime_summary || "Local office"}
                    </p>
                    <p>
                      <span className="font-medium text-gray-700">Office:</span>{" "}
                      {item?.office_label || "Local office"}
                    </p>
                    <p className="truncate">
                      <span className="font-medium text-gray-700">Address:</span>{" "}
                      {item?.office_url || "Uses the current office address"}
                    </p>
                  </div>
                  <div className="mt-5 flex flex-wrap gap-2">
                    {item?.open_app_path ? (
                      <Link href={item.open_app_path} className="inline-flex hover:no-underline">
                        <Button as="span" variant="primary" size="sm">
                          Play
                        </Button>
                      </Link>
                    ) : null}
                    <Link href={item?.edit_path ?? item?.path ?? "#"} className="inline-flex hover:no-underline">
                      <Button as="span" variant={item?.open_app_path ? "outline" : "primary"} size="sm">
                        Edit
                      </Button>
                    </Link>
                  </div>
                </CardContent>
              </Card>
            ))}
          </section>

          <section className="mt-12">
            <header className="mb-4">
              <h2 className="text-xl font-black tracking-tight text-gray-900">Office status</h2>
              <p className="mt-1 text-sm text-gray-500">
                Current office inventory, runtime availability, and placement health.
              </p>
            </header>
            <div className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
              {offices.map((office, index) => {
                const availability = String(office?.availability || "unknown");
                const availabilityTone =
                  availability === "online"
                    ? "bg-emerald-50 text-emerald-700 border-emerald-200"
                    : availability === "dangling"
                      ? "bg-amber-50 text-amber-700 border-amber-200"
                      : "bg-gray-100 text-gray-700 border-gray-200";
                const projects = Array.isArray(office?.hosted_projects) ? office.hosted_projects : [];
                const capabilities = Array.isArray(office?.capabilities) ? office.capabilities : [];
                return (
                  <Card key={`${office?.id ?? "office"}-${index}`}>
                    <CardContent className="py-5">
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <CardTitle className="text-lg">{office?.label || office?.id}</CardTitle>
                          <CardDescription className="mt-1">{office?.role || "Office"}</CardDescription>
                        </div>
                        <span
                          className={`inline-flex rounded-full border px-2 py-1 text-[0.7rem] font-semibold uppercase tracking-wide ${availabilityTone}`}
                        >
                          {availability}
                        </span>
                      </div>
                      <div className="mt-4 space-y-1 text-xs text-gray-500">
                        <p>
                          <span className="font-medium text-gray-700">State:</span>{" "}
                          {office?.resource_state || "unknown"}
                        </p>
                        <p className="truncate">
                          <span className="font-medium text-gray-700">Address:</span>{" "}
                          {office?.address || "No advertised address"}
                        </p>
                        <p>
                          <span className="font-medium text-gray-700">Version:</span>{" "}
                          {office?.version || "unknown"}
                        </p>
                        <p>
                          <span className="font-medium text-gray-700">Last seen:</span>{" "}
                          {office?.last_seen || "unknown"}
                        </p>
                        <p>
                          <span className="font-medium text-gray-700">Hosted projects:</span>{" "}
                          {office?.hosted_project_count ?? 0}
                        </p>
                        <p className="truncate">
                          <span className="font-medium text-gray-700">Capabilities:</span>{" "}
                          {capabilities.length > 0 ? capabilities.join(", ") : "none declared"}
                        </p>
                        {projects.length > 0 ? (
                          <p className="truncate">
                            <span className="font-medium text-gray-700">Examples:</span>{" "}
                            {projects.slice(0, 3).join(", ")}
                            {projects.length > 3 ? ` +${projects.length - 3} more` : ""}
                          </p>
                        ) : null}
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          </section>
        </section>
      </main>

      <Dialog open={marketplaceOpen} onOpenChange={setMarketplaceOpen}>
        <DialogContent
          className="p-0 overflow-hidden border border-gray-200 rounded-2xl shadow-[0_28px_90px_rgba(15,23,42,0.18)]"
          style={{
            width: "min(96vw, 1280px)",
            maxWidth: "min(96vw, 1280px)",
            height: "90vh",
            maxHeight: "90vh",
            backgroundColor: "#ffffff",
            color: "#111827",
          }}
        >
          <div className="flex h-full flex-col bg-white">
            <div className="border-b border-gray-200 px-6 py-5 pr-14">
              <DialogHeader>
                <DialogTitle>Marketplace</DialogTitle>
                <DialogDescription>
                  Register marketplace API sources here, then install published apps directly into Home.
                </DialogDescription>
              </DialogHeader>
            </div>

            <div className="flex min-h-0 flex-1">
              <aside className="min-h-0 w-60 shrink-0 overflow-y-auto border-r border-gray-200 bg-gray-50">
                <div className="py-3">
                  <button
                    type="button"
                    onClick={() => setMarketplaceView("settings")}
                    className={`flex w-full items-center border-l-2 px-4 py-3 text-left text-sm font-medium transition ${
                      marketplaceView === "settings"
                        ? "border-gray-900 bg-white text-gray-900"
                        : "border-transparent text-gray-600 hover:bg-white hover:text-gray-900"
                    }`}
                  >
                    Settings
                  </button>
                  {marketplaceSources.map((source) => (
                    <button
                      key={source?.repository_id}
                      type="button"
                      onClick={() => setMarketplaceView(source?.repository_id || "")}
                      className={`flex w-full items-center border-l-2 px-4 py-3 text-left text-sm transition ${
                        marketplaceView === source?.repository_id
                          ? "border-gray-900 bg-white text-gray-900"
                          : "border-transparent text-gray-600 hover:bg-white hover:text-gray-900"
                      }`}
                    >
                      <span className="truncate">{source?.title || source?.repository_id}</span>
                    </button>
                  ))}
                </div>
              </aside>

              <section className="min-h-0 min-w-0 flex-1 overflow-y-auto bg-white px-6 py-5">
                {marketplaceStatus ? (
                  <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-700">
                    {marketplaceStatus}
                  </div>
                ) : null}

                {marketplaceView === "settings" ? (
                  <div className="space-y-4">
                    <div>
                      <h3 className="text-base font-semibold text-gray-900">Settings</h3>
                      <p className="mt-1 text-sm text-gray-500">
                        Add marketplace API sources here. Example direct base:
                        {" "}
                        <code>http://127.0.0.1:10612/api/projects/superadmin/default/marketplace</code>
                      </p>
                    </div>
                    <form className="grid gap-4 md:grid-cols-2" onSubmit={handleSaveMarketplaceSource}>
                      <Field label="Repository ID" id="home-marketplace-repository-id">
                        <Input id="home-marketplace-repository-id" value={sourceForm.repository_id} onInput={(e) => setSourceForm((prev) => ({ ...prev, repository_id: e.target.value }))} placeholder="demo-source" required />
                      </Field>
                      <Field label="Title" id="home-marketplace-title">
                        <Input id="home-marketplace-title" value={sourceForm.title} onInput={(e) => setSourceForm((prev) => ({ ...prev, title: e.target.value }))} placeholder="Demo Source Zebflow" />
                      </Field>
                      <Field label="Base URL" id="home-marketplace-base-url">
                        <Input id="home-marketplace-base-url" value={sourceForm.base_url} onInput={(e) => setSourceForm((prev) => ({ ...prev, base_url: e.target.value }))} placeholder="https://marketplace.zebflow.com/api" required />
                      </Field>
                      <Field label="Read token" id="home-marketplace-read-token">
                        <Input id="home-marketplace-read-token" value={sourceForm.read_token} onInput={(e) => setSourceForm((prev) => ({ ...prev, read_token: e.target.value }))} placeholder="Optional" />
                      </Field>
                      <Field label="Remote owner" id="home-marketplace-remote-owner">
                        <Input id="home-marketplace-remote-owner" value={sourceForm.remote_owner} onInput={(e) => setSourceForm((prev) => ({ ...prev, remote_owner: e.target.value }))} placeholder="marketplace" />
                      </Field>
                      <Field label="Remote project" id="home-marketplace-remote-project">
                        <Input id="home-marketplace-remote-project" value={sourceForm.remote_project} onInput={(e) => setSourceForm((prev) => ({ ...prev, remote_project: e.target.value }))} placeholder="default" />
                      </Field>
                      <div className="md:col-span-2 flex justify-end">
                        <Button type="submit" variant="outline" disabled={marketplaceBusy}>
                          Add Marketplace
                        </Button>
                      </div>
                    </form>
                  </div>
                ) : (() => {
                  const source = marketplaceSources.find((item) => item?.repository_id === marketplaceView);
                  const items = marketplaceApps.filter((item) => item?.repository_id === marketplaceView);
                  return (
                    <div className="space-y-5">
                      <div className="flex items-start justify-between gap-3 border-b border-gray-200 pb-4">
                        <div>
                          <h3 className="text-base font-semibold text-gray-900">{source?.title || source?.repository_id}</h3>
                          <p className="mt-1 break-all text-sm text-gray-500">{source?.base_url}</p>
                          <p className="mt-2 text-xs text-gray-500">Repository ID: {source?.repository_id}</p>
                        </div>
                        <Button type="button" variant="ghost" size="sm" onClick={() => handleDeleteMarketplaceSource(source?.repository_id)} disabled={marketplaceBusy}>
                          Remove
                        </Button>
                      </div>

                      <div>
                        <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Apps</p>
                        {items.length === 0 ? (
                          <pre className="mt-3 text-xs text-gray-500">[]</pre>
                        ) : (
                          <ul className="mt-3 divide-y divide-gray-200 border-t border-b border-gray-200">
                            {items.map((item, itemIndex) => {
                              const installKey = `${item?.repository_id || ""}:${item?.package_id || ""}:${item?.latest_version || ""}`;
                              return (
                                <li key={`${item?.package_id ?? "app"}-${itemIndex}`} className="py-4">
                                  <div className="flex items-start justify-between gap-4">
                                    <div className="min-w-0">
                                      <p className="text-sm font-semibold text-gray-900">{item?.title || item?.package_id}</p>
                                      <p className="mt-1 text-sm text-gray-500">{item?.description || item?.package_id}</p>
                                      <p className="mt-2 text-[0.7rem] uppercase tracking-wide text-gray-400">
                                        {item?.package_id} · {item?.latest_version || "-"}
                                      </p>
                                    </div>
                                    <Button type="button" variant="primary" size="sm" disabled={installingId === installKey} onClick={() => handleInstallMarketplaceApp(item)}>
                                      Install
                                    </Button>
                                  </div>
                                </li>
                              );
                            })}
                          </ul>
                        )}
                      </div>
                    </div>
                  );
                })()}
              </section>
            </div>

            <div className="border-t border-gray-200 px-6 py-4">
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setMarketplaceOpen(false)}>
                  Close
                </Button>
              </DialogFooter>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Create project dialog */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <form method="post" action="/home/projects/create" className="flex flex-col">
            <div className="space-y-4 px-6 pt-6 pb-2">
              <DialogHeader>
                <DialogTitle>Create project</DialogTitle>
                <DialogDescription>Choose a URL slug and an optional display title.</DialogDescription>
              </DialogHeader>
              <div className="space-y-4">
                <Field label="Project slug" id="home-create-slug">
                  <Input
                    type="text"
                    name="project"
                    id="home-create-slug"
                    placeholder="e.g. my-app"
                    required
                  />
                </Field>
                <Field label="Title" id="home-create-title">
                  <Input type="text" name="title" id="home-create-title" placeholder="Display name" />
                </Field>
                <Field label="Default local branch" id="home-create-branch">
                  <Input
                    type="text"
                    name="local_branch"
                    id="home-create-branch"
                    placeholder="main"
                    value={createBranch}
                    onInput={(e) => setCreateBranch(e.target.value)}
                  />
                </Field>
                <Field label="Runtime mode" id="home-create-runtime-mode">
                  <select
                    id="home-create-runtime-mode"
                    name="runtime_mode"
                    value={createRuntimeMode}
                    onChange={(e) => setCreateRuntimeMode(e.target.value)}
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
                  >
                    <option value="shared">Shared</option>
                    <option value="pinned">Pinned</option>
                    <option value="dedicated">Dedicated</option>
                  </select>
                </Field>
                <Field
                  label="Office target"
                  id="home-create-placement-worker"
                  description="Local keeps the project inside this office. Pick another office to place the runtime remotely."
                >
                  <select
                    id="home-create-placement-worker"
                    name="placement_worker_id"
                    value={createPlacementWorker}
                    onChange={(e) => setCreatePlacementWorker(e.target.value)}
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
                  >
                    {runtimeTargets.map((item) => (
                      <option key={item.value} value={item.value}>
                        {item.label}
                      </option>
                    ))}
                  </select>
                </Field>
              </div>
            </div>
            <DialogFooter className="px-6 pb-6">
              <Button type="button" variant="outline" onClick={() => setCreateOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary">
                Create
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Clone project dialog */}
      <Dialog open={cloneOpen} onOpenChange={setCloneOpen}>
        <DialogContent>
          <form method="post" action="/home/projects/clone" className="flex flex-col">
            <input type="hidden" name="provider" value={provider} />

            <div className="space-y-4 px-6 pt-6 pb-2">
              <DialogHeader>
                <DialogTitle>Clone project</DialogTitle>
                <DialogDescription>Clone a remote Git repository into a new project.</DialogDescription>
              </DialogHeader>

              {/* Provider tabs */}
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant={provider === "gitlab" ? "primary" : "outline"}
                  size="sm"
                  onClick={() => setProvider("gitlab")}
                >
                  GitLab
                </Button>
                <Button
                  type="button"
                  variant={provider === "github" ? "primary" : "outline"}
                  size="sm"
                  onClick={() => setProvider("github")}
                >
                  GitHub
                </Button>
              </div>

              <div className="space-y-4">
                {provider === "gitlab" && (
                  <Field label="GitLab instance URL" id="home-clone-instance-url">
                    <Input
                      type="url"
                      name="instance_url"
                      id="home-clone-instance-url"
                      placeholder="https://gitlab.com"
                      defaultValue="https://gitlab.com"
                    />
                  </Field>
                )}

                <Field label="Repository URL" id="home-clone-repo-url">
                  <Input
                    type="url"
                    name="repo_url"
                    id="home-clone-repo-url"
                    placeholder={provider === "github" ? "https://github.com/user/repo.git" : "https://gitlab.com/user/repo.git"}
                    required
                    onInput={handleRepoUrlInput}
                  />
                </Field>

                <Field label="Project slug" id="home-clone-slug">
                  <Input
                    type="text"
                    name="project"
                    id="home-clone-slug"
                    placeholder="auto-derived from URL"
                    value={projectSlug}
                    onInput={(e) => setProjectSlug(e.target.value)}
                    required
                  />
                </Field>

                <Field
                  label="Remote branch"
                  id="home-clone-remote-branch"
                  description="Branch on the remote repository to clone from."
                >
                  <Input
                    type="text"
                    name="remote_branch"
                    id="home-clone-remote-branch"
                    placeholder="main"
                    value={remoteBranch}
                    onInput={handleRemoteBranchInput}
                  />
                </Field>

                <Field
                  label="Local branch name"
                  id="home-clone-local-branch"
                  description="Name for the local branch (leave same as remote, or rename e.g. dev)."
                >
                  <Input
                    type="text"
                    name="local_branch"
                    id="home-clone-local-branch"
                    placeholder="main"
                    value={localBranch}
                    onInput={(e) => setLocalBranch(e.target.value)}
                  />
                </Field>

                <Field label="Runtime mode" id="home-clone-runtime-mode">
                  <select
                    id="home-clone-runtime-mode"
                    name="runtime_mode"
                    value={cloneRuntimeMode}
                    onChange={(e) => setCloneRuntimeMode(e.target.value)}
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
                  >
                    <option value="shared">Shared</option>
                    <option value="pinned">Pinned</option>
                    <option value="dedicated">Dedicated</option>
                  </select>
                </Field>

                <Field
                  label="Office target"
                  id="home-clone-placement-worker"
                  description="Choose which office should host the cloned project's resident runtime."
                >
                  <select
                    id="home-clone-placement-worker"
                    name="placement_worker_id"
                    value={clonePlacementWorker}
                    onChange={(e) => setClonePlacementWorker(e.target.value)}
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
                  >
                    {runtimeTargets.map((item) => (
                      <option key={item.value} value={item.value}>
                        {item.label}
                      </option>
                    ))}
                  </select>
                </Field>

                <Field label="Username" id="home-clone-username">
                  <Input
                    type="text"
                    name="username"
                    id="home-clone-username"
                    placeholder={provider === "github" ? "GitHub username" : "GitLab username"}
                    required
                  />
                </Field>

                <Field
                  label="Access token"
                  id="home-clone-token"
                  description={tokenHelp}
                >
                  <Input
                    type="password"
                    name="token"
                    id="home-clone-token"
                    placeholder="Paste your access token"
                    required
                  />
                </Field>

                <Field label="Committer name" id="home-clone-git-name">
                  <Input
                    type="text"
                    name="git_name"
                    id="home-clone-git-name"
                    placeholder="Your Name"
                    required
                  />
                </Field>

                <Field label="Committer email" id="home-clone-git-email">
                  <Input
                    type="email"
                    name="git_email"
                    id="home-clone-git-email"
                    placeholder="you@example.com"
                    required
                  />
                </Field>
              </div>
            </div>

            <DialogFooter className="px-6 pb-6">
              <Button type="button" variant="outline" onClick={() => setCloneOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary">
                Clone
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
