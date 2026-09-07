import { cx, useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import GitHealthPanel from "@/pages/project-studio/settings/components/git-health-panel";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";
import GitHealthPanel from "@/pages/project-studio/settings/components/git-health-panel";

/** The git remote, and the health report nested under it. */
export default function GitPanel({ remoteApi, initialConfig, healthApi, repairApi }) {
  const [credentialId, setCredentialId] = useState(String(initialConfig?.credential_id ?? ""));
  const [repoUrl, setRepoUrl] = useState(String(initialConfig?.repo_url ?? ""));
  const [branch, setBranch] = useState(String(initialConfig?.branch ?? "main"));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);

  function handleSubmit(e) {
    e.preventDefault();
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    requestJson(remoteApi, {
        method: "PUT",
        body: JSON.stringify({
          credential_id: credentialId.trim(),
          repo_url: repoUrl.trim(),
          branch: branch.trim() || "main",
        }),
      }).then((resp) => {
      if (resp?.ok) {
        setStatusMsg("Remote saved.");
        setStatusTone("ok");
      } else {
        setStatusMsg("Saved.");
        setStatusTone("ok");
      }
    }).catch((err) => {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    }).finally(() => {
      setSaving(false);
    });
  }

  useEffect(() => {
    setCredentialId(String(initialConfig?.credential_id ?? ""));
    setRepoUrl(String(initialConfig?.repo_url ?? ""));
    setBranch(String(initialConfig?.branch ?? "main"));
  }, [initialConfig?.credential_id, initialConfig?.repo_url, initialConfig?.branch]);

  return (
    <>
      <GitHealthPanel healthApi={healthApi} repairApi={repairApi} />
      <SettingsSection
        title="Git Remote"
        description="Remote repository settings for project sync and push operations. Git author identity comes from the acting platform user."
        tag="Git"
      >
        <form className="grid grid-cols-1 gap-[0.65rem]" onSubmit={handleSubmit}>
          <Field label="Credential ID">
            <Input
              name="credential_id"
              placeholder="git-origin"
              value={credentialId}
              onInput={(e) => setCredentialId(e.currentTarget.value)}
            />
          </Field>
          <Field label="Repository URL">
            <Input
              name="repo_url"
              placeholder="https://gitlab.com/org/repo.git"
              value={repoUrl}
              onInput={(e) => setRepoUrl(e.currentTarget.value)}
            />
          </Field>
          <Field label="Branch">
            <Input
              name="branch"
              placeholder="main"
              value={branch}
              onInput={(e) => setBranch(e.currentTarget.value)}
            />
          </Field>
          <div className="flex items-center gap-[0.7rem]">
            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={saving}
              label={saving ? "Saving..." : "Save Git Remote"}
            />
            <span className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</span>
          </div>
        </form>
      </SettingsSection>
    </>
  );
}
