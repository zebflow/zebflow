import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import CommitDialog from "@/components/ui/commit-dialog";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** Render engine behaviour: caching, hydration, and the cache escape hatch. */
export default function RwePanel({ api, initialConfig, owner, project }) {
  const [allowList, setAllowList] = useState(
    (initialConfig?.allow_list ?? []).join("\n")
  );
  const [minify, setMinify] = useState(Boolean(initialConfig?.minify_html));
  const [strict, setStrict] = useState(initialConfig?.strict_mode !== false);
  const [deploymentAssetBase, setDeploymentAssetBase] = useState(
    String(initialConfig?.deployment_asset_base ?? "")
  );
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);

  function handleSubmit(e) {
    e.preventDefault();
    setPendingData({
      allow_list: allowList.split(/[\n,]/).map((s) => s.trim()).filter(Boolean),
      minify_html: minify,
      strict_mode: strict,
      deployment_asset_base: deploymentAssetBase.trim() || null,
    });
    setCommitOpen(true);
  }

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const resp = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({ commit_message: commitMessage, data: pendingData }),
      });
      if (resp?.committed) {
        setStatusMsg("Saved & committed.");
        setStatusTone("ok");
      } else if (resp?.git_error) {
        setStatusMsg(`Saved (git: ${resp.git_error})`);
        setStatusTone("info");
      } else {
        setStatusMsg("Saved.");
        setStatusTone("ok");
      }
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
      setPendingData(null);
    }
  }

  async function handleClearCache() {
    setClearing(true);
    try {
      const res = await fetch(`/api/projects/${owner}/${project}/rwe/cache/clear`, { method: "POST" });
      if (res.ok) {
        setStatusMsg("Template cache cleared.");
        setStatusTone("ok");
      } else {
        setStatusMsg("Failed to clear cache.");
        setStatusTone("error");
      }
    } finally {
      setClearing(false);
    }
  }

  return (
    <SettingsSection
      title="Reactive Web Engine"
      description={
        <>
          Project-level compile and render controls for all <code>n.web.render</code> nodes.
        </>
      }
      tag="RWE"
    >
      <CommitDialog
        open={commitOpen}
        section="rwe"
        defaultMessage="settings(rwe): update RWE config"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>Script Allow List</span>
          <Textarea
            name="allow_list"
            rows={4}
            placeholder={"https://cdnjs.cloudflare.com/*\nhttps://cdn.jsdelivr.net/*"}
            value={allowList}
            onInput={(e) => setAllowList(e.currentTarget.value)}
          />
          <small className="pipeline-editor-field-help">
            One URL pattern per line (or comma-separated). Controls which external scripts and
            stylesheets <code>--load-scripts</code> may reference. Blessed <code>zeb/*</code>{" "}
            libraries are always allowed and do not appear here.
          </small>
        </label>

        <div className="flex flex-col gap-2 pt-1">
          <Checkbox
            name="minify_html"
            label="Minify HTML output"
            checked={minify}
            onChange={(e) => setMinify(e.target.checked)}
          />
          <Checkbox
            name="strict_mode"
            label="Strict compile-time checks"
            checked={strict}
            onChange={(e) => setStrict(e.target.checked)}
          />
        </div>

        <label className="pipeline-editor-field col-span-full">
          <span>Asset Base Path</span>
          <Input
            name="deployment_asset_base"
            placeholder={`/static/${owner ?? "owner"}/${project ?? "project"}`}
            value={deploymentAssetBase}
            onInput={(e) => setDeploymentAssetBase(e.currentTarget.value)}
          />
          <small className="pipeline-editor-field-help">
            Replaces the default <code>/static/{"{owner}/{project}"}</code> prefix in all rendered HTML — scripts, images, uploads, library chunks. E.g. set to <code>/my/custom/path</code> and <code>/static/{"{owner}/{project}"}/_rwe/…</code> becomes <code>/my/custom/path/_rwe/…</code>. Leave empty to keep the default.
          </small>
        </label>

        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save RWE Config"}
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={clearing}
            onClick={handleClearCache}
            label={clearing ? "Clearing…" : "Clear Template Cache"}
          />
          <span className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</span>
        </div>
      </form>
    </SettingsSection>
  );
}
