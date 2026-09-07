import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import CommitDialog from "@/components/ui/commit-dialog";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** The project's name and description. */
export default function ProfilePanel({ api, initialConfig }) {
  const [title, setTitle] = useState(String(initialConfig?.title ?? ""));
  const [description, setDescription] = useState(String(initialConfig?.description ?? ""));
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
        body: JSON.stringify({ commit_message: commitMessage, data: { title, description } }),
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
    <SettingsSection title="Project Profile" description="Portable display identity for this project." tag="Profile">
      <CommitDialog
        open={commitOpen}
        section="profile"
        defaultMessage="settings(profile): update project profile"
        onConfirm={handleCommit}
        onCancel={() => setCommitOpen(false)}
      />
      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={(event) => { event.preventDefault(); setCommitOpen(true); }}>
        <Field label="Title">
          <Input value={title} maxLength={256} onInput={(event) => setTitle(event.currentTarget.value)} />
        </Field>
        <label className="pipeline-editor-field col-span-full">
          <span>Description</span>
          <Textarea rows={4} maxLength={16384} value={description} onInput={(event) => setDescription(event.currentTarget.value)} />
        </label>
        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button type="submit" variant="primary" size="sm" disabled={saving} label={saving ? "Saving..." : "Save Profile"} />
          <span className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</span>
        </div>
      </form>
    </SettingsSection>
  );
}
