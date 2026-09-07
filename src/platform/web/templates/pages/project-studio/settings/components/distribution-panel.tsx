import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import CommitDialog from "@/components/ui/commit-dialog";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** Whether the project is published, and under what terms. */
export default function DistributionPanel({ api, initialConfig }) {
  const [entryUrl, setEntryUrl] = useState(String(initialConfig?.entry_url ?? ""));
  const [asApp, setAsApp] = useState(Boolean(initialConfig?.as_app));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const response = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({ commit_message: commitMessage, data: { entry_url: entryUrl, as_app: asApp } }),
      });
      setStatusMsg(response?.committed ? "Saved & committed." : response?.git_error ? `Saved (git: ${response.git_error})` : "Saved.");
      setStatusTone(response?.git_error ? "info" : "ok");
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
    }
  }

  return (
    <SettingsSection title="Project Presentation" description="How this project appears as an application from the project dashboard." tag="Hub">
      <CommitDialog
        open={commitOpen}
        section="distribution"
        defaultMessage="settings(distribution): update project presentation"
        onConfirm={handleCommit}
        onCancel={() => setCommitOpen(false)}
      />
      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={(event) => { event.preventDefault(); setCommitOpen(true); }}>
        <Field label="Application Entry Path">
          <Input placeholder="/app" value={entryUrl} onInput={(event) => setEntryUrl(event.currentTarget.value)} />
        </Field>
        <div className="flex items-center pt-5">
          <Checkbox label="Open this project as an application" checked={asApp} onChange={(event) => setAsApp(event.target.checked)} />
        </div>
        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button type="submit" variant="primary" size="sm" disabled={saving} label={saving ? "Saving..." : "Save Presentation"} />
          <span className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</span>
        </div>
      </form>
    </SettingsSection>
  );
}
