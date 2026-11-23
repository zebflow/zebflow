import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import { slugify } from "@/pages/project-studio/hub/components/hub-slug";
import { publishOptionFacts } from "@/pages/project-studio/hub/components/publish-source-types";

/**
 * The publish form: what is being packed, its preview, its review, and the
 * calls that review and publish it.
 */
export function usePublishForm({ input, api, refresh, showStatus }) {
  const [publishSources, setPublishSources] = useState(Array.isArray(input?.publish_sources) ? input.publish_sources : []);
  const [publishPreview, setPublishPreview] = useState(null);
  const [publishReview, setPublishReview] = useState(null);
  const [publishReviewDirty, setPublishReviewDirty] = useState(true);
  const [imageUploadBusy, setImageUploadBusy] = useState(false);
  const publishOptions = input?.publish_options ?? {};
  const facts = publishOptionFacts(publishOptions);
  const { availableLibraries, sekejapSchemaAvailable, sqliteSchemaAvailable, initialDataAvailable } = facts;
  const [publishForm, setPublishForm] = useState({
    source_type: "pipeline_with_dependencies",
    source_ref: (input?.publish_sources?.[0]?.source_ref) || "",
    package_id: "",
    version: "0.1.0",
    publisher_token: "",
    destination: "hub_repository",
    repo_url: "",
    branch: "main",
    repo_token: "",
    readme_file_path: "",
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

  function patchPublishForm(patch) {
    setPublishForm((prev) => ({ ...prev, ...patch }));
    setPublishReviewDirty(true);
  }

  const toRepository = publishForm.destination === "hub_repository";
  const credentialReady = toRepository ? !!publishForm.repo_token : !!publishForm.publisher_token;

  function publishPayload() {
    const destination = toRepository
      ? { repo_url: publishForm.repo_url, branch: publishForm.branch, token: publishForm.repo_token, readme_file_path: publishForm.readme_file_path }
      : {};
    return {
      ...destination,
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
      const payload = await requestJson(toRepository ? api.review_repository : api.publish_review, {
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
      const payload = await requestJson(toRepository ? api.publish_repository : api.publish_asset, {
        method: "POST",
        body: JSON.stringify(publishPayload()),
      });
      if (!toRepository) await refresh();
      setPublishReview(null);
      setPublishReviewDirty(true);
      const report = payload?.report;
      showStatus(report
        ? (report.changed
          ? `Published ${report.package_id}@${report.version} to ${report.repository} (commit ${String(report.commit).slice(0, 7)})`
          : `${report.package_id}@${report.version} is already in ${report.repository}; nothing changed`)
        : `Published ${payload?.package?.package_id || publishForm.package_id}@${payload?.version?.version || publishForm.version}`);
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

  return {
    form: publishForm,
    patch: patchPublishForm,
    sources: publishSources,
    preview: publishPreview,
    review: publishReview,
    reviewDirty: publishReviewDirty,
    credentialReady,
    officialHubGitUrl: api.official_hub_git_url || "",
    imageUploadBusy,
    options: publishOptions,
    facts,
    uploadCoverImage,
    reviewPublish,
    publishAsset,
    togglePublishLibrary,
    toggleInitialDataPath,
  };
}
