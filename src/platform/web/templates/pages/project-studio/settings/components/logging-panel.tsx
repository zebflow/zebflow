import { cx, useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import CommitDialog from "@/components/ui/commit-dialog";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import TraceCaptureFields from "@/components/ui/trace-capture-fields";
import { traceCaptureFormConfig, traceCaptureFormValues } from "@/components/lib/trace-capture";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import InvocationStatsGrid from "@/pages/project-studio/settings/components/invocation-stats-grid";
import InvocationLogTable from "@/pages/project-studio/settings/components/invocation-log-table";

/**
 * Project defaults for invocation log capture and retention.
 *
 * Owns the stats and every way they change — loading, saving retention,
 * clearing one pipeline or all of them. The grid and the table below it only
 * display what this panel holds.
 */
export default function LoggingPanel({ api, invocationsApi, initialConfig }) {
  const [maxInv, setMaxInv] = useState(String(initialConfig?.max_invocations ?? 20));
  const [captureValues, setCaptureValues] = useState(traceCaptureFormValues(initialConfig?.trace_capture));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);
  const [stats, setStats] = useState(null);
  const [loadingStats, setLoadingStats] = useState(false);
  const [clearTarget, setClearTarget] = useState(null);
  const [clearing, setClearing] = useState(false);

  async function loadStats() {
    if (!invocationsApi) return;
    setLoadingStats(true);
    try {
      const resp = await requestJson(invocationsApi);
      setStats(resp?.stats ?? null);
    } catch (err) {
      setStatusMsg(`Failed to load logs: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setLoadingStats(false);
    }
  }

  useEffect(() => {
    loadStats();
  }, [invocationsApi]);

  function handleSubmit(e) {
    e.preventDefault();
    try {
      setPendingData({ max_invocations: parseInt(maxInv, 10) || 20, trace_capture: traceCaptureFormConfig(captureValues) });
      setCommitOpen(true);
    } catch (err) {
      setStatusMsg(err?.message || String(err));
      setStatusTone("error");
    }
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

  async function handleClearLogs() {
    if (!invocationsApi || !clearTarget) return;
    setClearing(true);
    setStatusMsg("Clearing invocation logs...");
    setStatusTone("info");
    try {
      const url = clearTarget.file_rel_path
        ? `${invocationsApi}?pipeline=${encodeURIComponent(clearTarget.file_rel_path)}`
        : invocationsApi;
      const resp = await requestJson(url, { method: "DELETE" });
      setStatusMsg(`Cleared ${resp?.deleted ?? 0} invocation log row(s).`);
      setStatusTone("ok");
      await loadStats();
    } catch (err) {
      setStatusMsg(`Clear failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setClearing(false);
      setClearTarget(null);
    }
  }

  const pipelines = Array.isArray(stats?.pipelines) ? stats.pipelines : [];

  return (
    <article className="border border-border rounded-lg bg-card p-[0.85rem] mb-[0.9rem]">
      <CommitDialog
        open={commitOpen}
        section="logging"
        defaultMessage="settings(logging): update capture and retention config"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <ConfirmDialog
        open={!!clearTarget}
        title={clearTarget?.file_rel_path ? "Clear Pipeline Invocation Logs" : "Clear All Invocation Logs"}
        message={
          clearTarget?.file_rel_path
            ? `Delete invocation history for ${clearTarget.file_rel_path}? This cannot be undone.`
            : "Delete all invocation history for this project? This cannot be undone."
        }
        confirmLabel={clearing ? "Clearing..." : "Clear Logs"}
        cancelLabel="Cancel"
        variant="destructive"
        onConfirm={handleClearLogs}
        onClose={() => !clearing && setClearTarget(null)}
      />
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Pipeline Logging</h3>
          <p className="project-card-copy">Default data capture, retention, storage, and cleanup for project invocation logs.</p>
        </div>
        <span className="project-inline-chip">Logging</span>
      </header>
      <InvocationStatsGrid stats={stats} />

      <form className="grid grid-cols-2 gap-[0.65rem] mb-4" onSubmit={handleSubmit}>
        <Field label="Max Invocations Per Pipeline">
          <Input
            name="max_invocations"
            type="number"
            min="1"
            max="1000"
            value={maxInv}
            onInput={(e) => setMaxInv(e.currentTarget.value)}
          />
          <small className="pipeline-editor-field-help">
            How many invocation log entries to retain per pipeline. Oldest are dropped. Default: 20.
          </small>
        </Field>
        <TraceCaptureFields values={captureValues} onChange={setCaptureValues} scope="project" />
        <div className="col-span-full flex flex-wrap items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Logging Config"}
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={loadingStats}
            onClick={loadStats}
          >
            {loadingStats ? "Refreshing..." : "Refresh Stats"}
          </Button>
          <Button
            type="button"
            variant="destructive"
            size="sm"
            disabled={clearing || Number(stats?.count || 0) === 0}
            onClick={() => setClearTarget({ file_rel_path: null })}
          >
            Clear All Logs
          </Button>
          <span className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</span>
        </div>
      </form>

      <InvocationLogTable
        pipelines={pipelines}
        loadingStats={loadingStats}
        clearing={clearing}
        onClear={(filePath) => setClearTarget({ file_rel_path: filePath })}
      />
    </article>
  );
}
