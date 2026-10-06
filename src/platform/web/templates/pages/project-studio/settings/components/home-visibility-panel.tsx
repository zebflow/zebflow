import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import CheckboxField from "@/components/ui/checkbox-field";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/**
 * Hide from home. A setting of this office, not of the project's files: it is
 * not committed, so it saves the moment the box is ticked.
 */
export default function HomeVisibilityPanel({ api, initialHidden }) {
  const [hidden, setHidden] = useState(Boolean(initialHidden));
  const [statusMsg, setStatusMsg] = useState("");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);

  async function save(next) {
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const response = await requestJson(api, { method: "PUT", body: JSON.stringify({ data: { hidden: next } }) });
      setHidden(Boolean(response?.data?.hidden));
      setStatusMsg(response?.data?.hidden ? "Hidden from home." : "Shown on home.");
      setStatusTone("ok");
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
    }
  }

  return (
    <SettingsSection title="Home" description="Whether this office's home page lists the project." tag="Home">
      <div className="flex flex-col gap-2" data-home-visibility>
        <CheckboxField
          label="Hide from home"
          description="Left off the home project list and kept under Hidden projects there. The project still opens by its address and in search."
          checked={hidden}
          disabled={saving || !api}
          onChange={(event) => save(Boolean(event.currentTarget.checked))}
        />
        {statusMsg ? <span className={cx("text-[0.72rem]", settingsStatusToneClass(statusTone))}>{statusMsg}</span> : null}
      </div>
    </SettingsSection>
  );
}
