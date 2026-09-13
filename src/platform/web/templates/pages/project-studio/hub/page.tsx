import { requestJson } from "@/components/lib/http";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";
import CheckboxField from "@/components/ui/checkbox-field";
import Input from "@/components/ui/input";
import SqlReport from "@/components/package-review/sql-report";
import { ViolationNotice, WarningNotice } from "@/components/package-review/findings";
import { useEffect, useState } from "zeb/react";
import { formatBytes } from "@/components/lib/format";
import HubBrowser from "@/components/hub/hub-browser";
import InstallFromFileDialog from "@/pages/project-studio/hub/components/install-from-file-dialog";
import HubSourcesDialog from "@/pages/project-studio/hub/components/hub-sources-dialog";
import { useHubInventory } from "@/components/hub/use-hub-inventory";
import { needsDestination, verbOf } from "@/components/hub/hub-kinds";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import NodeRegistryPanel from "@/pages/project-studio/hub/components/node-registry-panel";
import DependenciesPanel from "@/pages/project-studio/hub/components/dependencies-panel";

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

function blankProjectSource() {
  return {
    repository_id: "",
    title: "",
    base_url: "https://hub.zebflow.com/api",
    remote_owner: "",
    remote_project: "",
    read_token: "",
    enabled: true,
  };
}

function ReviewList({ title, items, danger = false }) {
  const values = Array.isArray(items) ? items.filter(Boolean) : [];
  return (
    <div className={danger ? "rounded-lg border border-red-400/30 bg-red-500/10 px-3 py-2" : "rounded-lg border border-border bg-accent/20 px-3 py-2"}>
      <p className={danger ? "m-0 text-xs font-medium text-red-100" : "m-0 text-xs font-medium text-foreground"}>{title}</p>
      {values.length ? (
        <ul className={danger ? "mt-1 space-y-1 text-[11px] text-red-50" : "mt-1 space-y-1 text-[11px] text-muted-foreground"}>
          {values.slice(0, 8).map((item, index) => <li key={`${title}-${index}`}>{String(item)}</li>)}
          {values.length > 8 ? <li>+{values.length - 8} more</li> : null}
        </ul>
      ) : (
        <p className="m-0 mt-1 text-[11px] text-muted-foreground">None</p>
      )}
    </div>
  );
}

function BundleReviewList({ title, items }) {
  const values = Array.isArray(items) ? items : [];
  return (
    <div className="rounded-lg border border-border bg-accent/20 px-3 py-2">
      <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{title}</p>
      {values.length ? (
        <ul className="mt-1 list-disc pl-5 text-sm text-foreground">
          {values.map((item, index) => (
            <li key={`${title}-${index}`}>{item}</li>
          ))}
        </ul>
      ) : (
        <p className="mt-1 text-sm text-muted-foreground">None</p>
      )}
    </div>
  );
}

export default function Page(input) {
  const tabs = Array.isArray(input?.hub_tabs) ? input.hub_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const api = input?.hub_api ?? {};
  const [packs, setPacks] = useState(Array.isArray(input?.assets) ? input.assets : []);
  // Where an added package lands. Only add-kinds ask for it; install-kinds go to
  // `data/` and have nowhere to choose.
  const [hubReviewTargetFolder, setHubReviewTargetFolder] = useState("");
  const [fromFileOpen, setFromFileOpen] = useState(false);
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const { browseItems, refreshInventory } = useHubInventory(packs, input?.installed, api);

  // What an add would overwrite, held while the reader decides. Null means
  // nothing is being asked.
  const [overwritePrompt, setOverwritePrompt] = useState(null);
  const [browseBusy, setBrowseBusy] = useState(false);

  /**
   * Acting on the selected package.
   *
   * Reviews before it writes. The review already knows which files an add would
   * replace, and replacing a file someone wrote is the one part of this that
   * cannot be undone — so it is the one part worth stopping for. Everything
   * else proceeds without a question.
   */
  async function onBrowseAct(item) {
    if (item?.repair_all) {
      await repairDependencies();
      return;
    }
    if (item?.intent === "remove") {
      await removeLibrary(item);
      return;
    }
    const packageId = item?.package_id;
    const version = item?.latest_version;
    if (!packageId || !version) {
      showStatus("That package has no installable version.");
      return;
    }

    // Only add-kinds land in the repository, so only they have somewhere to
    // choose. Install-kinds go to `data/` and the folder is meaningless.
    const folder = needsDestination(item?.asset_kind) ? hubReviewTargetFolder : "";

    setBrowseBusy(true);
    try {
      const review = await requestJson(reviewUrl(item), {
        method: "POST",
        body: JSON.stringify({ target_folder: folder }),
      });
      const overwritten = review?.review?.files_overwritten ?? [];
      if (overwritten.length) {
        setOverwritePrompt({ item, folder, files: overwritten });
        return;
      }
      await commitAdd(item, folder);
    } catch (err) {
      showStatus(err?.message || err);
    } finally {
      setBrowseBusy(false);
    }
  }

  function reviewUrl(item) {
    const packageId = encodeURIComponent(item?.package_id);
    const version = encodeURIComponent(item?.latest_version);
    return item?.source === "remote"
      ? `${api.repositories}/${encodeURIComponent(item.repository_id)}/packs/${packageId}/${version}/review`
      : `${api.assets}/${packageId}/${version}/review`;
  }

  /** The write itself, after the reader has seen what it costs. */
  async function commitAdd(item, folder) {
    const packageId = item?.package_id;
    const version = item?.latest_version;
    const verb = verbOf(item?.asset_kind) === "add" ? "Adding" : "Installing";
    showStatus(`${verb} ${packageId}@${version}...`);
    const url = item?.source === "remote"
      ? `${api.repositories}/${encodeURIComponent(item.repository_id)}/packs/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`
      : `${api.assets}/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`;
    const payload = await requestJson(url, {
      method: "POST",
      body: JSON.stringify({ target_folder: folder }),
    });
    const result = payload?.result || {};
    // The row must now say "in project", which it only learns by re-reading.
    await refresh();
    showStatus(
      `${packageId}@${version} — ${result.files_written || 0} file(s) into ${result.install_root || "the project"}`,
    );
  }

  // What a removal would delete, held while the reader decides.
  const [removePrompt, setRemovePrompt] = useState(null);

  async function removeLibrary(item) {
    setRemovePrompt(item);
  }

  async function commitRemove(item) {
    const name = libraryNameOf(item);
    showStatus(`Removing ${name}...`);
    try {
      await requestJson(`${api.libraries_remove}?name=${encodeURIComponent(name)}`, {
        method: "DELETE",
      });
      await refresh();
      showStatus(`Removed ${name} — its lock entry and its files are gone.`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  /**
   * The name the lock knows a library by.
   *
   * The hub calls it `zebflow.use`; the lock and every template import call it
   * `zeb/use`. Removal is addressed by the second, so a package id has to be
   * translated back before it is sent.
   */
  function libraryNameOf(item) {
    if (item?.name?.startsWith("zeb/")) return item.name;
    const id = String(item?.package_id ?? "");
    return id.startsWith("zebflow.") ? `zeb/${id.slice("zebflow.".length)}` : id;
  }

  async function repairDependencies() {
    setBrowseBusy(true);
    showStatus("Repairing packages that no longer match zeb.lock...");
    try {
      await requestJson(api.dependencies, { method: "POST", body: "{}" });
      await refresh();
      showStatus("Repair finished.");
    } catch (err) {
      showStatus(err?.message || err);
    } finally {
      setBrowseBusy(false);
    }
  }
  const [myPacks, setMyPacks] = useState(Array.isArray(input?.my_assets) ? input.my_assets : []);
  const [hubSources, setHubSources] = useState([]);
  const [sourceForm, setSourceForm] = useState(blankProjectSource());
  const [editingSourceId, setEditingSourceId] = useState("");
  const [publishSources, setPublishSources] = useState(Array.isArray(input?.publish_sources) ? input.publish_sources : []);
  const [publishPreview, setPublishPreview] = useState(null);
  const [publishReview, setPublishReview] = useState(null);
  const [publishReviewDirty, setPublishReviewDirty] = useState(true);
  const [imageUploadBusy, setImageUploadBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [bundleFileName, setBundleFileName] = useState("");
  const [bundleArtifact, setBundleArtifact] = useState(null);
  const [bundleReview, setBundleReview] = useState(null);
  const [bundleBusy, setBundleBusy] = useState(false);
  const publishOptions = input?.publish_options ?? {};
  const availableLibraries = Array.isArray(publishOptions?.libraries) ? publishOptions.libraries : [];
  const sekejapSchemaAvailable = !!publishOptions?.sekejap_schema?.available;
  const sqliteSchemaAvailable = !!publishOptions?.sqlite_schema?.available;
  const initialDataItems = Array.isArray(publishOptions?.initial_data?.items) ? publishOptions.initial_data.items : [];
  const initialDataAvailable = !!publishOptions?.initial_data?.available && initialDataItems.length > 0;
  const [publishForm, setPublishForm] = useState({
    source_type: "pipeline_with_dependencies",
    source_ref: (input?.publish_sources?.[0]?.source_ref) || "",
    package_id: "",
    version: "0.1.0",
    publisher_token: "",
    title: "",
    description: "",
    image_file_path: "",
    visibility: "private",
    tags_csv: "",
    include_sekejap_schema: sekejapSchemaAvailable,
    include_sqlite_schema: sqliteSchemaAvailable,
    include_libraries: availableLibraries.map((item) => item?.name).filter(Boolean),
    include_initial_data: false,
    initial_data_paths: [],
  });

  function showStatus(value) {
    setStatus(describeStatus(value));
  }

  function patchPublishForm(patch) {
    setPublishForm((prev) => ({ ...prev, ...patch }));
    setPublishReviewDirty(true);
  }

  function publishPayload() {
    return {
      source_type: publishForm.source_type,
      source_ref: publishForm.source_ref,
      package_id: publishForm.package_id,
      version: publishForm.version,
      title: publishForm.title,
      description: publishForm.description,
      image_file_path: publishForm.image_file_path,
      publisher_token: publishForm.publisher_token,
      visibility: publishForm.visibility,
      tags: String(publishForm.tags_csv || "").split(",").map((s) => s.trim()).filter(Boolean),
      include_sekejap_schema: publishForm.source_type === "project_files" && sekejapSchemaAvailable ? !!publishForm.include_sekejap_schema : false,
      include_sqlite_schema: publishForm.source_type === "project_files" && sqliteSchemaAvailable ? !!publishForm.include_sqlite_schema : false,
      include_libraries: publishForm.source_type === "project_files" ? (Array.isArray(publishForm.include_libraries) ? publishForm.include_libraries : []) : [],
      include_initial_data: publishForm.source_type === "project_files" && initialDataAvailable ? !!publishForm.include_initial_data : false,
      initial_data_paths: publishForm.source_type === "project_files" && publishForm.include_initial_data ? (Array.isArray(publishForm.initial_data_paths) ? publishForm.initial_data_paths : []) : [],
    };
  }

  useEffect(() => {
    if (!status) return;
    const timer = setTimeout(() => setStatus(""), 5000);
    return () => clearTimeout(timer);
  }, [status]);

  async function refresh() {
    const tasks = [
      requestJson(api.assets),
      requestJson(api.my_assets),
      api.access ? requestJson(api.access) : Promise.resolve(null),
      refreshInventory(),
    ];
    const [assetsRes, myRes, sourceRes] = await Promise.all(tasks);
    setPacks(Array.isArray(assetsRes?.items) ? assetsRes.items : []);
    setMyPacks(Array.isArray(myRes?.items) ? myRes.items : []);
    setHubSources(Array.isArray(sourceRes?.repositories) ? sourceRes.repositories : []);
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
    setPublishReview(null);
    setPublishReviewDirty(true);
  }, [publishForm.source_type, publishForm.source_ref]);

  async function uploadCoverImage(event) {
    const file = event?.target?.files?.[0];
    if (!file || !api.upload) return;
    setImageUploadBusy(true);
    showStatus("Uploading cover image...");
    try {
      const form = new FormData();
      form.append("file", file, file.name || "cover");
      const payload = await requestJson(`${api.upload}?path=${encodeURIComponent("hub-media")}`, {
        method: "POST",
        body: form,
      });
      patchPublishForm({ image_file_path: payload?.path || "" });
      showStatus(`Uploaded cover image to ${payload?.path || "hub-media"}. It will be normalized to WebP during review/publish.`);
    } catch (err) {
      showStatus(err?.message || err);
    } finally {
      setImageUploadBusy(false);
      if (event?.target) event.target.value = "";
    }
  }

  async function reviewPublish(event) {
    event?.preventDefault?.();
    showStatus("Reviewing package...");
    try {
      const payload = await requestJson(api.publish_review, {
        method: "POST",
        body: JSON.stringify(publishPayload()),
      });
      setPublishReview(payload?.review || null);
      setPublishReviewDirty(false);
      showStatus(`Review ready: risk ${payload?.review?.risk_level || "unknown"}`);
    } catch (err) {
      setPublishReview(null);
      setPublishReviewDirty(true);
      showStatus(err?.message || err);
    }
  }

  async function publishAsset(event) {
    event?.preventDefault?.();
    if (!publishReview || publishReviewDirty) {
      showStatus("Run package review before publishing.");
      return;
    }
    showStatus("Publishing package...");
    try {
      const payload = await requestJson(api.publish_asset, {
        method: "POST",
        body: JSON.stringify(publishPayload()),
      });
      await refresh();
      setPublishReview(null);
      setPublishReviewDirty(true);
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
        body: JSON.stringify({ target_folder: "" }),
      });
      const result = payload?.result || {};
      showStatus(`Added ${result.files_written || 0} file(s) into ${result.install_root || "project"} workspace`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function retractAsset(item) {
    const packageId = item?.package_id;
    if (!packageId) return;
    if (!publishForm.publisher_token) {
      showStatus("Publisher token is required to retract a package.");
      return;
    }
    if (!window.confirm(`Retract ${packageId}? Its files are destroyed, its name and versions stay listed, and they can never be published again.`)) {
      return;
    }
    showStatus(`Retracting ${packageId}...`);
    try {
      const payload = await requestJson(`${api.assets}/${encodeURIComponent(packageId)}`, {
        method: "DELETE",
        headers: { Authorization: `Bearer ${publishForm.publisher_token}` },
      });
      await refresh();
      showStatus(`Retracted ${packageId} (${payload?.retracted_versions || 0} version(s))`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  function editSource(item) {
    setEditingSourceId(item?.repository_id || "");
    setSourceForm({
      repository_id: item?.repository_id || "",
      title: item?.title || "",
      base_url: item?.base_url || "https://hub.zebflow.com/api",
      remote_owner: item?.remote_owner || "",
      remote_project: item?.remote_project || "",
      read_token: "",
      enabled: item?.enabled !== false,
    });
  }

  function resetSourceForm() {
    setEditingSourceId("");
    setSourceForm(blankProjectSource());
  }

  async function saveSource(event) {
    event?.preventDefault?.();
    if (!api.repositories) return;
    const repositoryId = sourceForm.repository_id || slugify(sourceForm.title || "hub-source");
    showStatus("Saving project Hub source...");
    try {
      await requestJson(api.repositories, {
        method: "POST",
        body: JSON.stringify({
          repository_id: repositoryId,
          title: sourceForm.title || repositoryId,
          base_url: sourceForm.base_url,
          remote_owner: sourceForm.remote_owner,
          remote_project: sourceForm.remote_project,
          read_token: sourceForm.read_token,
          enabled: !!sourceForm.enabled,
        }),
      });
      resetSourceForm();
      await refresh();
      showStatus("Project Hub source saved.");
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function deleteSource(item) {
    const repositoryId = item?.repository_id;
    if (!repositoryId || !api.repositories || item?.editable === false) return;
    if (!window.confirm(`Delete project Hub source ${repositoryId}?`)) return;
    showStatus(`Deleting ${repositoryId}...`);
    try {
      await requestJson(`${api.repositories}/${encodeURIComponent(repositoryId)}`, { method: "DELETE" });
      if (editingSourceId === repositoryId) resetSourceForm();
      await refresh();
      showStatus("Project Hub source deleted.");
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  function togglePublishLibrary(name) {
    const value = String(name || "");
    if (!value) return;
    const current = Array.isArray(publishForm.include_libraries) ? publishForm.include_libraries : [];
    patchPublishForm({
      include_libraries: current.includes(value)
        ? current.filter((item) => item !== value)
        : [...current, value],
    });
  }

  function toggleInitialDataPath(path) {
    const value = String(path || "");
    if (!value) return;
    const current = Array.isArray(publishForm.initial_data_paths) ? publishForm.initial_data_paths : [];
    const next = current.includes(value)
      ? current.filter((item) => item !== value)
      : [...current, value];
    patchPublishForm({
      initial_data_paths: next,
      include_initial_data: next.length > 0,
    });
  }

  const selectedSource = publishSources.find((item) => item.source_ref === publishForm.source_ref) || null;
  const selectedType = SOURCE_TYPES.find((item) => item.value === publishForm.source_type) || SOURCE_TYPES[0];
  const projectSources = hubSources.filter((item) => item?.source_scope === "project_local" || item?.editable);
  const sharedSources = hubSources.filter((item) => item?.source_scope !== "project_local" && !item?.editable);

  const bundleIdentity = () => {
    const metadata = bundleArtifact?.metadata ?? {};
    return {
      package_id: String(metadata.name ?? ""),
      version: String(metadata.version ?? ""),
    };
  };

  const onPickBundleFile = (event) => {
    const file = event?.target?.files?.[0];
    if (!file) return;
    setBundleReview(null);
    setBundleArtifact(null);
    setBundleFileName(file.name);
    file
      .text()
      .then((text) => {
        const parsed = JSON.parse(text);
        setBundleArtifact(parsed);
        showStatus(`Loaded ${file.name}. Review it before installing.`);
      })
      .catch(() => {
        setBundleFileName("");
        showStatus("That file is not a readable Zebflow package document.");
      });
  };

  const reviewBundle = () => {
    const identity = bundleIdentity();
    if (!bundleArtifact || !identity.package_id || !identity.version) {
      showStatus("Choose a package file that declares metadata.name and metadata.version.");
      return;
    }
    setBundleBusy(true);
    requestJson(api.node_bundle_review, {
      method: "POST",
      body: JSON.stringify({ ...identity, target_folder: "", artifact: bundleArtifact }),
    })
      .then((payload) => {
        setBundleReview(payload?.review ?? null);
        showStatus("Reviewed. Check what this package does before installing.");
      })
      .catch((err) => showStatus(err?.message || err))
      .then(() => setBundleBusy(false));
  };

  const installBundle = () => {
    const identity = bundleIdentity();
    setBundleBusy(true);
    requestJson(api.node_bundle_install, {
      method: "POST",
      body: JSON.stringify({ ...identity, target_folder: "", artifact: bundleArtifact }),
    })
      .then(() => {
        setBundleReview(null);
        setBundleArtifact(null);
        setBundleFileName("");
        showStatus(`Installed ${identity.package_id} ${identity.version}.`);
      })
      .catch((err) => showStatus(err?.message || err))
      .then(() => setBundleBusy(false));
  };

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

          <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-background">
            <div className="project-content-wrap">
              <section className="project-content-section">
                <div className="project-content-head">
                  <div>
                    <p className="project-content-title">Project Hub</p>
                    <p className="project-content-copy">Browse packages and add them to this project.</p>
                  </div>
                  <div className="flex items-center gap-2">
                    {/* The rare paths, behind buttons: as permanent blocks they
                        pushed the catalogue below the fold. */}
                    <Button type="button" variant="outline" size="sm" onClick={() => setSourcesOpen(true)}>
                      Sources
                    </Button>
                    <Button type="button" variant="outline" size="sm" onClick={() => setFromFileOpen(true)}>
                      Install from file
                    </Button>
                    <Button type="button" variant="outline" size="sm" onClick={() => refresh().then(() => showStatus("Refreshed")).catch((err) => showStatus(err?.message || err))}>
                      Refresh
                    </Button>
                  </div>
                </div>
              </section>

              <section className="project-content-section">
                <div className="project-content-body space-y-4">
                  {status ? (
                    <div className="rounded-lg border border-border bg-accent/30 px-4 py-3 text-sm text-muted-foreground">
                      <span className="font-medium text-foreground">Status:</span> {status}
                    </div>
                  ) : null}

                  <ConfirmDialog
        open={!!removePrompt}
        onClose={() => setRemovePrompt(null)}
        onConfirm={async () => {
          const pending = removePrompt;
          setRemovePrompt(null);
          if (!pending) return;
          setBrowseBusy(true);
          try {
            await commitRemove(pending);
          } finally {
            setBrowseBusy(false);
          }
        }}
        title="Remove this package from the project?"
        confirmLabel="Remove it"
        cancelLabel="Cancel"
        variant="destructive"
        busy={browseBusy}
      >
        <p className="text-[0.8rem] text-muted-foreground">
          <span className="font-mono">{libraryNameOf(removePrompt)}</span> is deleted from{" "}
          <code className="font-mono">zeb.lock</code> and its installed files are removed. Any
          template importing it stops working until it is installed again.
        </p>
      </ConfirmDialog>

      <ConfirmDialog
        open={!!overwritePrompt}
        onClose={() => setOverwritePrompt(null)}
        onConfirm={async () => {
          const pending = overwritePrompt;
          setOverwritePrompt(null);
          if (!pending) return;
          setBrowseBusy(true);
          try {
            await commitAdd(pending.item, pending.folder);
          } catch (err) {
            showStatus(err?.message || err);
          } finally {
            setBrowseBusy(false);
          }
        }}
        title="Replace files already in this project?"
        confirmLabel="Replace them"
        cancelLabel="Cancel"
        variant="destructive"
        busy={browseBusy}
      >
        <p className="text-[0.8rem] text-muted-foreground">
          Adding <span className="font-mono">{overwritePrompt?.item?.package_id}</span> writes over
          {" "}{overwritePrompt?.files?.length} file(s) that already exist here. What is in them now
          is lost.
        </p>
        <ul className="mt-3 flex flex-col gap-1">
          {(overwritePrompt?.files ?? []).map((file) => (
            <li key={file} className="font-mono text-[0.72rem] text-muted-foreground">
              {file}
            </li>
          ))}
        </ul>
      </ConfirmDialog>

      <InstallFromFileDialog open={fromFileOpen} onClose={() => setFromFileOpen(false)}>
                    <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                      <div>
                        <p className="project-content-subtitle">Node Bundle From File</p>
                        <p className="text-sm text-muted-foreground">
                          Install a node bundle you authored locally or received as a file. It runs the same
                          review a published package runs: a Hub package is not safer, only published.
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        <Button as="label" type="button" variant="outline" size="sm">
                          Choose file
                          <input
                            type="file"
                            accept="application/json,.json"
                            className="sr-only"
                            onChange={onPickBundleFile}
                          />
                        </Button>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          disabled={!bundleArtifact || bundleBusy}
                          onClick={reviewBundle}
                        >
                          Review
                        </Button>
                        <Button
                          type="button"
                          size="sm"
                          disabled={!bundleReview?.installable || bundleBusy}
                          onClick={installBundle}
                        >
                          Install
                        </Button>
                      </div>
                    </div>

                    {bundleFileName ? (
                      <p className="mt-3 text-sm text-muted-foreground">
                        Selected: <span className="font-medium text-foreground">{bundleFileName}</span>
                      </p>
                    ) : null}

                    {bundleReview ? (
                      <div className="mt-4 space-y-3">
                        <ViolationNotice items={bundleReview.violations} subject="This bundle" />
                        <WarningNotice items={bundleReview.warnings} />

                        <SqlReport
                          reports={bundleReview.database_initialization}
                          emptyNote="This bundle carries no install-time SQL. Nothing is replayed into any store."
                        />

                        <div className="grid gap-3 md:grid-cols-2">
                          <BundleReviewList title="Nodes provided" items={bundleReview.nodes_used} />
                          <BundleReviewList title="Credentials requested" items={bundleReview.credentials_required} />
                          <BundleReviewList title="External URLs contacted" items={bundleReview.external_urls} />
                          <BundleReviewList title="Public endpoints created" items={bundleReview.public_endpoints} />
                          <BundleReviewList title="Database effects" items={bundleReview.database_effects} />
                          <BundleReviewList title="File effects" items={bundleReview.filesystem_effects} />
                          <BundleReviewList title="Outbound connections" items={bundleReview.network_effects} />
                          <BundleReviewList title="Runs supplied code" items={bundleReview.code_execution} />
                          <BundleReviewList title="Schedules" items={bundleReview.schedules} />
                          <BundleReviewList title="Files written" items={bundleReview.files_added} />
                          <BundleReviewList title="Files overwritten" items={bundleReview.files_overwritten} />
                        </div>

                        <p className="text-sm text-muted-foreground">
                          Risk level: <span className="font-medium text-foreground">{bundleReview.risk_level}</span>
                        </p>
                      </div>
                    ) : null}
                  </InstallFromFileDialog>

                  {tabFlags?.packs ? (
                    <>
                      <HubSourcesDialog open={sourcesOpen} onClose={() => setSourcesOpen(false)}>
                        <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                          <div>
                            <p className="project-content-subtitle">Hub Sources</p>
                            <p className="text-sm text-muted-foreground">Project sources are private to this project. Shared sources are granted from Home &gt; Hub and are read-only here.</p>
                          </div>
                          <Button type="button" variant="outline" size="sm" onClick={resetSourceForm}>
                            New Project Source
                          </Button>
                        </div>

                        <form className="mt-4 grid gap-3 md:grid-cols-2" onSubmit={saveSource}>
                          <Field label="Source ID">
                            <Input
                              value={sourceForm.repository_id}
                              onInput={(e) => setSourceForm((prev) => ({ ...prev, repository_id: slugify(e.currentTarget.value) }))}
                              placeholder="my-private-hub"
                            />
                          </Field>
                          <Field label="Title">
                            <Input
                              value={sourceForm.title}
                              onInput={(e) => setSourceForm((prev) => ({ ...prev, title: e.currentTarget.value }))}
                              placeholder="My Private Hub"
                            />
                          </Field>
                          <Field label="Base URL">
                            <Input
                              value={sourceForm.base_url}
                              onInput={(e) => setSourceForm((prev) => ({ ...prev, base_url: e.currentTarget.value }))}
                              placeholder="https://hub.zebflow.com/api"
                            />
                          </Field>
                          <Field label="Read Token">
                            <Input
                              type="password"
                              value={sourceForm.read_token}
                              onInput={(e) => setSourceForm((prev) => ({ ...prev, read_token: e.currentTarget.value }))}
                              placeholder={editingSourceId ? "Leave blank to keep existing token" : "Optional"}
                            />
                          </Field>
                          <Field label="Remote owner">
                            <Input
                              value={sourceForm.remote_owner}
                              onInput={(e) => setSourceForm((prev) => ({ ...prev, remote_owner: e.currentTarget.value }))}
                              placeholder="Only for legacy project-coordinate Hub"
                            />
                          </Field>
                          <Field label="Remote project">
                            <Input
                              value={sourceForm.remote_project}
                              onInput={(e) => setSourceForm((prev) => ({ ...prev, remote_project: e.currentTarget.value }))}
                              placeholder="Only for legacy project-coordinate Hub"
                            />
                          </Field>
                          <CheckboxField
                            label="Enabled"
                            checked={sourceForm.enabled}
                            onChange={(e) => setSourceForm((prev) => ({ ...prev, enabled: e.currentTarget.checked }))}
                          />
                          <div className="flex items-center justify-end gap-2">
                            {editingSourceId ? <Button type="button" variant="outline" size="sm" onClick={resetSourceForm}>Cancel Edit</Button> : null}
                            <Button type="submit" size="sm" variant="primary">{editingSourceId ? "Save Source" : "Add Project Source"}</Button>
                          </div>
                        </form>

                        <div className="mt-5 grid gap-4 lg:grid-cols-2">
                          <div className="rounded-lg border border-border bg-accent/20">
                            <div className="border-b border-border px-3 py-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                              Project sources
                            </div>
                            <div className="divide-y divide-border/60">
                              {projectSources.map((item) => (
                                <div key={`project-${item.repository_id}`} className="flex items-start justify-between gap-3 px-3 py-3">
                                  <div className="min-w-0">
                                    <div className="flex items-center gap-2">
                                      <span className="truncate text-sm font-medium text-foreground">{item.title || item.repository_id}</span>
                                      <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase text-muted-foreground">{item.enabled ? "enabled" : "off"}</span>
                                    </div>
                                    <p className="m-0 mt-1 truncate text-xs text-muted-foreground">{item.base_url}</p>
                                    <p className="m-0 mt-1 text-xs text-muted-foreground">
                                      {item.repository_id}{item.has_read_token ? " · token set" : ""}
                                    </p>
                                  </div>
                                  <div className="flex shrink-0 gap-1">
                                    <Button type="button" variant="ghost" size="sm" onClick={() => editSource(item)}>Edit</Button>
                                    <Button type="button" variant="ghost" size="sm" onClick={() => deleteSource(item)}>Delete</Button>
                                  </div>
                                </div>
                              ))}
                              {!projectSources.length ? (
                                <div className="px-3 py-4 text-sm text-muted-foreground">No project-local Hub sources.</div>
                              ) : null}
                            </div>
                          </div>

                          <div className="rounded-lg border border-border bg-accent/20">
                            <div className="border-b border-border px-3 py-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                              Shared from platform
                            </div>
                            <div className="divide-y divide-border/60">
                              {sharedSources.map((item) => (
                                <div key={`shared-${item.repository_id}`} className="px-3 py-3">
                                  <div className="flex items-center gap-2">
                                    <span className="truncate text-sm font-medium text-foreground">{item.title || item.repository_id}</span>
                                    <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase text-muted-foreground">read-only</span>
                                  </div>
                                  <p className="m-0 mt-1 truncate text-xs text-muted-foreground">{item.base_url}</p>
                                  <p className="m-0 mt-1 text-xs text-muted-foreground">
                                    {item.repository_id}{item.has_read_token ? " · token set" : ""}
                                  </p>
                                </div>
                              ))}
                              {!sharedSources.length ? (
                                <div className="px-3 py-4 text-sm text-muted-foreground">No platform-granted Hub sources.</div>
                              ) : null}
                            </div>
                          </div>
                        </div>
                      </HubSourcesDialog>

                      <div className="flex flex-col rounded-lg border border-border bg-popover">
                        <HubBrowser
                          items={browseItems}
                          owner={input?.owner}
                          project={input?.project}
                          initialState="available"
                          destination={hubReviewTargetFolder}
                          onDestinationChange={setHubReviewTargetFolder}
                          busy={browseBusy}
                          onAct={onBrowseAct}
                        />
                      </div>
                    </>
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
                          <StudioTh>Action</StudioTh>
                        </tr>
                      </StudioThead>
                      <tbody>
                        {myPacks.map((item, index) => (
                          <tr key={`${item?.package_id ?? "mine"}-${index}`}>
                            <StudioTd>
                              <div className="flex items-center gap-3">
                                {item?.image_url ? <img src={item.image_url} alt="" className="h-10 w-14 rounded-md object-cover border border-border" /> : null}
                                <span>{item?.package_id}</span>
                              </div>
                            </StudioTd>
                            <StudioTd>{item?.title}</StudioTd>
                            <StudioTd>{item?.asset_kind}</StudioTd>
                            <StudioTd>{item?.latest_version || "-"}</StudioTd>
                            <StudioTd>{item?.retracted ? "retracted" : item?.visibility}</StudioTd>
                            <StudioTd>{fmtTs(item?.updated_at)}</StudioTd>
                            <StudioTd>
                              {item?.retracted ? null : (
                                <Button type="button" variant="ghost" size="sm" onClick={() => retractAsset(item)}>
                                  Retract
                                </Button>
                              )}
                            </StudioTd>
                          </tr>
                        ))}
                        {!myPacks.length ? (
                          <tr><StudioTd colSpan={7}>You have not published any packages yet.</StudioTd></tr>
                        ) : null}
                      </tbody>
                    </StudioTable>
                  ) : null}

                  {tabFlags?.publish ? (
                    <form className="space-y-4" onSubmit={publishAsset}>
                      <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">1. Choose Source Type</p>
                          <p className="text-sm text-muted-foreground">Pick the export scope first, then choose the specific item and review the final file tree.</p>
                        </div>
                        <Field label="Source type">
                          <Select
                            value={publishForm.source_type}
                            onChange={(e) => patchPublishForm({ source_type: e.target.value, source_ref: "" })}
                          >
                            {SOURCE_TYPES.map((item) => (
                              <SelectOption key={item.value} value={item.value} label={item.label} />
                            ))}
                          </Select>
                        </Field>
                        <div className="rounded-lg border border-border bg-accent/30 px-3 py-2 text-sm text-muted-foreground">
                          <span className="font-medium text-foreground">{selectedType.label}:</span> {selectedType.note}
                        </div>
                      </div>

                      <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">2. Select Item</p>
                          <p className="text-sm text-muted-foreground">Only name, description, and path are shown here. The actual export set is resolved in the preview step below.</p>
                        </div>
                        <Field label="Selected item">
                          <Select
                            value={publishForm.source_ref}
                            onChange={(e) => patchPublishForm({ source_ref: e.target.value })}
                          >
                            <SelectOption value="" label="Select item" />
                            {publishSources.map((item, index) => (
                              <SelectOption
                                key={`${item?.source_ref ?? "src"}-${index}`}
                                value={item?.source_ref}
                                label={`${item?.name} · ${item?.path}`}
                              />
                            ))}
                          </Select>
                        </Field>
                        {selectedSource ? (
                          <div className="grid gap-3 md:grid-cols-3 rounded-lg border border-border bg-accent/20 px-3 py-3 text-sm">
                            <div>
                              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Name</div>
                              <div className="mt-1 text-foreground">{selectedSource.name}</div>
                            </div>
                            <div>
                              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Description</div>
                              <div className="mt-1 text-foreground">{selectedSource.description}</div>
                            </div>
                            <div>
                              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Path</div>
                              <code className="mt-1 block text-xs text-foreground">{selectedSource.path}</code>
                            </div>
                          </div>
                        ) : null}
                      </div>

                      <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">3. Export Tree Preview</p>
                          <p className="text-sm text-muted-foreground">This is the exact file set that will be packed and published.</p>
                        </div>
                        {publishPreview ? (
                          <>
                            <div className="grid gap-3 md:grid-cols-4 rounded-lg border border-border bg-accent/20 px-3 py-3 text-sm">
                              <div>
                                <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Package kind</div>
                                <div className="mt-1 text-foreground">{publishPreview.asset_kind}</div>
                              </div>
                              <div>
                                <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Source</div>
                                <div className="mt-1 text-foreground">{sourceLabel(publishPreview.source_type)}</div>
                              </div>
                              <div>
                                <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Files</div>
                                <div className="mt-1 text-foreground">{publishPreview.total_files}</div>
                              </div>
                              <div>
                                <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Bytes</div>
                                <div className="mt-1 text-foreground">{formatBytes(publishPreview.total_bytes)}</div>
                              </div>
                            </div>

                            <div className="rounded-lg border border-border bg-accent/20">
                              <div className="border-b border-border px-3 py-2 text-xs font-mono uppercase tracking-widest text-muted-foreground">Resolved Files</div>
                              <div className="max-h-80 overflow-auto divide-y divide-border/60">
                                {publishPreview.entries.map((entry, index) => (
                                  <div key={`${entry.rel_path}-${index}`} className="grid gap-2 px-3 py-2 md:grid-cols-[minmax(0,1fr)_140px_90px]">
                                    <div className="min-w-0">
                                      <code className="block truncate text-xs text-foreground">{entry.rel_path}</code>
                                      <div className="mt-1 text-[11px] text-muted-foreground">{entry.reason}</div>
                                    </div>
                                    <div className="text-xs text-muted-foreground">{entry.kind}</div>
                                    <div className="text-right text-xs text-muted-foreground">{entry.size_bytes}</div>
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
                          <div className="rounded-lg border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
                            Select a source item to generate the export tree.
                          </div>
                        )}
                      </div>

                      <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
                        <div>
                          <p className="project-content-subtitle">4. Publish Package</p>
                          <p className="text-sm text-muted-foreground">Use a publisher token issued from Home &gt; Hub. Project sources can be added on the Browse tab; publishers and tokens stay in Home &gt; Hub.</p>
                        </div>
                        {publishForm.source_type === "project_files" ? (
                          <div className="rounded-lg border border-border bg-accent/20 p-3 space-y-3">
                            <div>
                              <p className="m-0 text-sm font-medium text-foreground">Project bundle initialization</p>
                              <p className="m-0 mt-1 text-xs text-muted-foreground">Choose the schema and runtime settings that the installed project should initialize from.</p>
                            </div>
                            <CheckboxField
                              label="Include Sekejap schema"
                              description={
                                sekejapSchemaAvailable
                                  ? `${publishOptions?.sekejap_schema?.table_count || 0} managed table schema(s)`
                                  : "No Sekejap schema found in this project"
                              }
                              checked={publishForm.include_sekejap_schema}
                              disabled={!sekejapSchemaAvailable}
                              onChange={(e) => patchPublishForm({ include_sekejap_schema: e.target.checked })}
                            />
                            <CheckboxField
                              label="Include SQLite schema"
                              description={
                                sqliteSchemaAvailable
                                  ? "Local SQLite DDL will be exported as schemas/sqlite/schema.sql"
                                  : "No SQLite schema found in local.db"
                              }
                              checked={publishForm.include_sqlite_schema}
                              disabled={!sqliteSchemaAvailable}
                              onChange={(e) => patchPublishForm({ include_sqlite_schema: e.target.checked })}
                            />
                            <div className="space-y-2">
                              <div className="flex items-center justify-between gap-3">
                                <div>
                                  <p className="m-0 text-xs font-medium uppercase tracking-wider text-muted-foreground">Initial data</p>
                                  <p className="m-0 text-xs text-muted-foreground">Selected SQL files execute during install after schemas are applied.</p>
                                </div>
                                <CheckboxField
                                  label=""
                                  checked={publishForm.include_initial_data}
                                  disabled={!initialDataAvailable}
                                  onChange={(e) => patchPublishForm({
                                    include_initial_data: e.target.checked,
                                    initial_data_paths: e.target.checked ? initialDataItems.map((item) => item.path).filter(Boolean) : [],
                                  })}
                                />
                              </div>
                              {initialDataAvailable ? (
                                <div className="grid gap-2">
                                  {initialDataItems.map((item) => {
                                    const path = item?.path || "";
                                    const checked = (Array.isArray(publishForm.initial_data_paths) ? publishForm.initial_data_paths : []).includes(path);
                                    return (
                                      <label key={path} className="flex items-center justify-between gap-3 rounded-md border border-border bg-popover px-3 py-2 text-sm text-foreground">
                                        <span className="min-w-0">
                                          <code className="block truncate text-xs">{path}</code>
                                          <span className="block text-xs text-muted-foreground">{item?.engine || "data"} · {item?.statement_count || 0} statement(s) · {formatBytes(item?.size_bytes || 0)}</span>
                                        </span>
                                        <CheckboxField label="" checked={checked} onChange={() => toggleInitialDataPath(path)} />
                                      </label>
                                    );
                                  })}
                                </div>
                              ) : (
                                <p className="m-0 text-xs text-muted-foreground">No initial data SQL files found under initial-data, init, or seeds.</p>
                              )}
                            </div>
                            <div className="space-y-2">
                              <p className="m-0 text-xs font-medium uppercase tracking-wider text-muted-foreground">Installed libraries</p>
                              {availableLibraries.length ? (
                                <div className="grid gap-2">
                                  {availableLibraries.map((item) => {
                                    const name = item?.name || "";
                                    const checked = (Array.isArray(publishForm.include_libraries) ? publishForm.include_libraries : []).includes(name);
                                    return (
                                      <label key={name} className="flex items-center justify-between gap-3 rounded-md border border-border bg-popover px-3 py-2 text-sm text-foreground">
                                        <span className="min-w-0">
                                          <span className="block truncate">{name}</span>
                                          <span className="block text-xs text-muted-foreground">{item?.version || "default"} · {item?.source || "offline"}</span>
                                        </span>
                                        <CheckboxField label="" checked={checked} onChange={() => togglePublishLibrary(name)} />
                                      </label>
                                    );
                                  })}
                                </div>
                              ) : (
                                <p className="m-0 text-xs text-muted-foreground">No project-enabled RWE libraries.</p>
                              )}
                            </div>
                          </div>
                        ) : null}
                        <Field label="Publisher Token">
                          <Input type="password" value={publishForm.publisher_token} onInput={(e) => patchPublishForm({ publisher_token: e.target.value })} placeholder="zfmt_..." />
                        </Field>
                        <div className="grid gap-3 md:grid-cols-2">
                          <Field label="Package ID">
                            <Input value={publishForm.package_id} onInput={(e) => patchPublishForm({ package_id: e.target.value })} placeholder="ev-charging-demo" />
                          </Field>
                          <Field label="Version">
                            <Input value={publishForm.version} onInput={(e) => patchPublishForm({ version: e.target.value })} placeholder="0.1.0" />
                          </Field>
                        </div>
                        <Field label="Title">
                          <Input value={publishForm.title} onInput={(e) => patchPublishForm({ title: e.target.value })} placeholder="EV Charging Demo Pipeline" />
                        </Field>
                        <Field label="Description">
                          <Input value={publishForm.description} onInput={(e) => patchPublishForm({ description: e.target.value })} placeholder="Short summary" />
                        </Field>
                        <Field label="Package image from Files">
                          <div className="space-y-2">
                            <Input value={publishForm.image_file_path} onInput={(e) => patchPublishForm({ image_file_path: e.target.value })} placeholder="hub-media/cover.png" />
                            <div className="flex items-center gap-2">
                              <label className="inline-flex items-center">
                                <input className="sr-only" type="file" accept="image/png,image/jpeg,image/webp,image/gif" onChange={uploadCoverImage} disabled={imageUploadBusy} />
                                <span className="zf-btn zf-btn-outline zf-btn-sm cursor-pointer">{imageUploadBusy ? "Uploading..." : "Upload image"}</span>
                              </label>
                              <span className="text-xs text-muted-foreground">Uploaded images are stored in Files and normalized to WebP in the Hub package.</span>
                            </div>
                          </div>
                        </Field>
                        <div className="grid gap-3 md:grid-cols-2">
                          <Field label="Visibility">
                            <Select value={publishForm.visibility} onChange={(e) => patchPublishForm({ visibility: e.target.value })}>
                              <SelectOption value="private" label="private" />
                              <SelectOption value="public" label="public" />
                              <SelectOption value="unlisted" label="unlisted" />
                            </Select>
                          </Field>
                          <Field label="Tags">
                            <Input value={publishForm.tags_csv} onInput={(e) => patchPublishForm({ tags_csv: e.target.value })} placeholder="ev, mobility, demo" />
                          </Field>
                        </div>
                        <div className="flex flex-wrap gap-2">
                          <Button type="button" variant="outline" onClick={reviewPublish} disabled={!publishForm.source_ref || !publishPreview?.entries?.length || !publishForm.publisher_token}>
                            {publishReviewDirty ? "Review Package" : "Review Again"}
                          </Button>
                          <Button type="submit" disabled={!publishForm.source_ref || !publishPreview?.entries?.length || !publishForm.publisher_token || !publishReview || publishReviewDirty || !!publishReview?.violations?.length}>{publishReview?.violations?.length ? "Blocked" : "Publish Package"}</Button>
                        </div>
                        {publishReview ? (
                          <div className="rounded-lg border border-border bg-accent/20 p-3 space-y-3">
                            <div className="flex items-start justify-between gap-3">
                              <div>
                                <p className="m-0 text-sm font-semibold text-foreground">Publish Review</p>
                                <p className="m-0 mt-1 text-xs text-muted-foreground">
                                  {publishReview.package_id}@{publishReview.version} · {publishReview.asset_kind} · risk {publishReview.risk_level}
                                </p>
                              </div>
                              {publishReviewDirty ? <span className="rounded-full border border-amber-400/40 px-2 py-0.5 text-[10px] uppercase text-amber-100">stale</span> : <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase text-muted-foreground">ready</span>}
                            </div>
                            <div className="grid gap-3 md:grid-cols-4 text-sm">
                              <div><div className="text-[11px] uppercase text-muted-foreground">Files</div><div>{publishReview.total_files}</div></div>
                              <div><div className="text-[11px] uppercase text-muted-foreground">Bytes</div><div>{formatBytes(publishReview.total_bytes)}</div></div>
                              <div><div className="text-[11px] uppercase text-muted-foreground">Visibility</div><div>{publishReview.visibility}</div></div>
                              <div><div className="text-[11px] uppercase text-muted-foreground">Cover</div><div>{publishReview.media?.[0]?.name || "None"}</div></div>
                            </div>
                            {publishReview.media?.length ? (
                              <div className="rounded-lg border border-border bg-popover px-3 py-2 text-xs text-muted-foreground">
                                Cover will be published as <code>{publishReview.media[0].name}</code> ({publishReview.media[0].content_type}, {formatBytes(publishReview.media[0].size_bytes)}).
                              </div>
                            ) : null}
                            {publishReview.asset_kind === "project_bundle" ? (
                              <div className="rounded-lg border border-border bg-popover px-3 py-2 text-xs text-muted-foreground">
                                <div className="font-medium text-foreground">Initialization</div>
                                <div className="mt-1">Sekejap schema: {publishReview.project_initialization?.include_sekejap_schema ? "included" : "not included"}</div>
                                <div>SQLite schema: {publishReview.project_initialization?.include_sqlite_schema ? "included" : "not included"}</div>
                                <div>Libraries: {(publishReview.project_initialization?.libraries || []).length ? publishReview.project_initialization.libraries.join(", ") : "none"}</div>
                                <div className="mt-2 font-medium text-foreground">Initial data to execute</div>
                                {(publishReview.project_initialization?.initial_data || []).length ? (
                                  <ul className="m-0 mt-1 space-y-1 pl-4">
                                    {publishReview.project_initialization.initial_data.map((item, index) => (
                                      <li key={`${item.path}-${index}`}>
                                        <code>{item.path}</code> · {item.engine} · {item.statement_count || 0} statement(s)
                                      </li>
                                    ))}
                                  </ul>
                                ) : (
                                  <div>none</div>
                                )}
                              </div>
                            ) : null}
                            <ViolationNotice items={publishReview.violations} subject="This package" outcome="published" refuser="publish" />
                            <div className="grid gap-2 text-xs md:grid-cols-2">
                              <ReviewList title="Nodes" items={publishReview.nodes_used} />
                              <ReviewList title="Credentials" items={publishReview.credentials_required} danger />
                              <ReviewList title="External URLs" items={publishReview.external_urls} danger />
                              <ReviewList title="Database effects" items={publishReview.database_effects} danger />
                              <ReviewList title="Filesystem effects" items={publishReview.filesystem_effects} />
                              <ReviewList title="Outbound connections" items={publishReview.network_effects} danger />
                              <ReviewList title="Runs supplied code" items={publishReview.code_execution} danger />
                              <ReviewList title="Public endpoints" items={publishReview.public_endpoints} danger />
                              <ReviewList title="Schedules" items={publishReview.schedules} danger />
                              <ReviewList title="Large files" items={publishReview.large_files} />
                              <ReviewList title="Seed/demo data" items={publishReview.seed_data} />
                              <ReviewList title="Warnings" items={publishReview.warnings} danger />
                            </div>
                          </div>
                        ) : null}
                      </div>
                    </form>
                  ) : null}

                  {/* What this project has installed. These sit with the Hub
                      because the Hub is where installing happens — the
                      inventory used to live in Settings while the install
                      button lived here. */}
                  {tabFlags?.nodes ? (
                    <NodeRegistryPanel
                      groups={input?.installed?.node_groups ?? []}
                      count={input?.installed?.node_count ?? 0}
                    />
                  ) : null}

                  {tabFlags?.dependencies ? (
                    <DependenciesPanel
                      api={input?.installed?.dependencies?.api ?? ""}
                      initialStatus={input?.installed?.dependencies?.status ?? {}}
                    />
                  ) : null}

                </div>
              </section>
            </div>
          </section>
        </div>
      </ProjectStudioShell>
  );
}
