import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import CommitDialog from "@/components/ui/commit-dialog";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** Defaults every pipeline run starts from. */
export default function RuntimeDefaultsPanel({ api, initialConfig }) {
  const [maxMb, setMaxMb] = useState(Number(initialConfig?.max_asset_size_mb ?? 10));
  const [fileMaxMb, setFileMaxMb] = useState(Number(initialConfig?.max_file_size_mb ?? 1024));
  const [webhookMaxMb, setWebhookMaxMb] = useState(Number(initialConfig?.webhook_body_max_mb ?? 100));
  const [nodeTimeoutSecs, setNodeTimeoutSecs] = useState(Number(initialConfig?.pipeline_node_timeout_secs ?? 30));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);

  function handleSubmit(e) {
    e.preventDefault();
    setPendingData({
      max_asset_size_mb: maxMb,
      max_file_size_mb: fileMaxMb,
      webhook_body_max_mb: webhookMaxMb,
      pipeline_node_timeout_secs: nodeTimeoutSecs,
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

  return (
    <SettingsSection
      title="Runtime Defaults"
      description="Project-level upload limits and node execution timeout defaults."
      tag="Assets"
    >
      <CommitDialog
        open={commitOpen}
        section="assets"
        defaultMessage="settings(assets): update runtime defaults"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <form className="flex flex-col gap-[0.65rem]" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>Max asset upload size: <strong>{maxMb} MB</strong></span>
          <input
            type="range"
            name="max_asset_size_mb"
            min={5}
            max={1024}
            step={1}
            value={maxMb}
            onInput={(e) => setMaxMb(Number((e.target as HTMLInputElement).value))}
            className="w-full cursor-pointer accent-dark-accent1"
          />
          <small className="pipeline-editor-field-help">
            Maximum file size per uploaded project asset (5–1024 MB).
          </small>
        </label>
        <label className="pipeline-editor-field">
          <span>Max FS upload size: <strong>{fileMaxMb} MB</strong></span>
          <input
            type="range"
            name="max_file_size_mb"
            min={5}
            max={1024}
            step={1}
            value={fileMaxMb}
            onInput={(e) => setFileMaxMb(Number((e.target as HTMLInputElement).value))}
            className="w-full cursor-pointer accent-dark-accent1"
          />
          <small className="pipeline-editor-field-help">
            Per-file limit for Project Files uploads at <code>/api/projects/.../files/upload</code> (5–1024 MB).
          </small>
        </label>
        <label className="pipeline-editor-field">
          <span>Webhook body max: <strong>{webhookMaxMb} MB</strong></span>
          <input
            type="range"
            name="webhook_body_max_mb"
            min={100}
            max={1024}
            step={1}
            value={webhookMaxMb}
            onInput={(e) => setWebhookMaxMb(Number((e.target as HTMLInputElement).value))}
            className="w-full cursor-pointer accent-dark-accent1"
          />
          <small className="pipeline-editor-field-help">
            Per-project logical request limit for <code>/wh/...</code> uploads (100–1024 MB). This should be at least as large as the documents your webhook pipelines need to accept.
          </small>
        </label>
        <label className="pipeline-editor-field">
          <span>Pipeline node timeout: <strong>{nodeTimeoutSecs} sec</strong></span>
          <input
            type="range"
            name="pipeline_node_timeout_secs"
            min={5}
            max={3600}
            step={5}
            value={nodeTimeoutSecs}
            onInput={(e) => setNodeTimeoutSecs(Number((e.target as HTMLInputElement).value))}
            className="w-full cursor-pointer accent-dark-accent1"
          />
          <small className="pipeline-editor-field-help">
            Maximum runtime per node before timeout (5–3600 sec effective). Raise this for heavy transforms like PDF conversion.
          </small>
        </label>
        <div className="flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Defaults"}
          />
          <span className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</span>
        </div>
      </form>
    </SettingsSection>
  );
}
