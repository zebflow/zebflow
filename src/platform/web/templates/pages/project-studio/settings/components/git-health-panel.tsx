import { cx, useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** What is wrong with the repository, and the repair for it. */
export default function GitHealthPanel({ healthApi, repairApi }) {
  const [health, setHealth] = useState(null);
  const [statusMsg, setStatusMsg] = useState("");
  const [statusTone, setStatusTone] = useState("info");
  const [busyAction, setBusyAction] = useState("");
  const [confirmMode, setConfirmMode] = useState("");

  async function loadHealth(showStatus = false) {
    if (!healthApi) return;
    if (showStatus) {
      setStatusMsg("Checking repository health...");
      setStatusTone("info");
    }
    try {
      const payload = await requestJson(healthApi);
      setHealth(payload?.health ?? null);
      if (showStatus) {
        setStatusMsg("Repository health updated.");
        setStatusTone("ok");
      }
    } catch (err) {
      setStatusMsg(`Check failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    }
  }

  async function handleRepair(mode) {
    if (!repairApi) return;
    const label = mode === "repair" ? "Repairing" : "Reinitializing";
    setBusyAction(mode);
    setStatusMsg(`${label} repository...`);
    setStatusTone("info");
    try {
      const payload = await requestJson(repairApi, {
        method: "POST",
        body: JSON.stringify({ mode }),
      });
      setHealth(payload?.health ?? null);
      setStatusMsg(mode === "repair" ? "Repository repair completed." : "Repository reinitialized.");
      setStatusTone("ok");
    } catch (err) {
      setStatusMsg(`Action failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setBusyAction("");
    }
  }

  useEffect(() => {
    loadHealth(false);
  }, [healthApi]);

  const healthState = String(health?.state ?? "unknown");
  const recommended = String(health?.recommended_action ?? "none");
  const branch = String(health?.branch ?? "");
  const confirmTitle = confirmMode === "repair" ? "Repair Git metadata?" : "Reinitialize Git repository?";
  const confirmMessage = confirmMode === "repair"
    ? "This will try to repair the current Git metadata while keeping the existing project files."
    : "This will rebuild Git metadata for the current project worktree. Existing broken Git metadata will be replaced.";
  const detailItems = [
    { label: ".git", value: health?.git_dir_exists ? "present" : "missing" },
    { label: "work tree", value: health?.is_work_tree ? "valid" : "invalid" },
    { label: "HEAD", value: health?.head_exists ? "present" : "missing" },
    { label: "config", value: health?.config_exists ? "present" : "missing" },
    { label: "objects", value: health?.objects_exists ? "present" : "missing" },
    { label: "refs", value: health?.refs_exists ? "present" : "missing" },
  ];

  return (
    <SettingsSection
      title="Repository Health"
      description="Inspect and repair the local Git metadata for this project when status is broken or missing."
      tag="Git"
    >
      <ConfirmDialog
        open={confirmMode.length > 0}
        title={confirmTitle}
        message={confirmMessage}
        confirmLabel="Yes"
        cancelLabel="No"
        variant={confirmMode === "reinitialize" ? "destructive" : "default"}
        onClose={() => setConfirmMode("")}
        onConfirm={() => {
          const mode = confirmMode;
          setConfirmMode("");
          handleRepair(mode);
        }}
      />
      <div className="flex flex-col gap-[0.8rem]">
        <div className="flex items-start justify-between gap-3 flex-wrap">
          <div className="flex flex-col gap-[0.45rem] min-w-0">
            <div className="flex items-center gap-[0.5rem] flex-wrap">
              <span className={cx(
                "inline-flex items-center rounded-full border px-2 py-[0.22rem] text-[0.65rem] font-semibold uppercase tracking-[0.08em]",
                healthState === "healthy"
                  ? "border-emerald-400/40 bg-emerald-400/10 text-emerald-300"
                  : healthState === "missing"
                    ? "border-amber-400/40 bg-amber-400/10 text-amber-200"
                    : healthState === "broken"
                      ? "border-red-400/40 bg-red-400/10 text-red-300"
                      : "border-border bg-muted text-muted-foreground"
              )}>
                {healthState}
              </span>
              {branch ? <span className="project-inline-chip">branch: {branch}</span> : null}
              {recommended !== "none" ? (
                <span className="project-inline-chip">recommended: {recommended}</span>
              ) : null}
            </div>
            <div className="text-[0.72rem] text-muted-foreground break-all">
              {String(health?.repo_path ?? "Loading repository path...")}
            </div>
            {health?.last_error ? (
              <div className="text-[0.72rem] text-red-300 break-words">{health.last_error}</div>
            ) : null}
          </div>
          <div className="flex items-center gap-[0.45rem] flex-wrap">
            <Button
              variant="outline"
              size="sm"
              disabled={busyAction.length > 0}
              label="Check"
              onClick={() => loadHealth(true)}
            />
            <Button
              variant="outline"
              size="sm"
              disabled={busyAction.length > 0}
              label={busyAction === "repair" ? "Repairing..." : "Repair"}
              onClick={() => setConfirmMode("repair")}
            />
            <Button
              variant="ghost"
              size="sm"
              disabled={busyAction.length > 0}
              className="border border-red-400/25 text-red-200 hover:bg-red-500/10"
              label={busyAction === "reinitialize" ? "Reinitializing..." : "Reinitialize"}
              onClick={() => setConfirmMode("reinitialize")}
            />
          </div>
        </div>

        <div className="flex flex-wrap gap-[0.45rem]">
          {detailItems.map((item) => (
            <span
              key={item.label}
              className="inline-flex items-center gap-[0.35rem] rounded-full border border-border bg-muted px-[0.72rem] py-[0.32rem] text-[0.72rem] text-foreground"
            >
              <span className="uppercase tracking-[0.08em] text-muted-foreground">{item.label}</span>
              <span>{item.value}</span>
            </span>
          ))}
        </div>

        {statusMsg ? (
          <div className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</div>
        ) : null}
      </div>
    </SettingsSection>
  );
}
