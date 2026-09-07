import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** Deleting the project. Asks for the name back before it will act. */
export default function DangerZone({ owner, project }) {
  const [confirmName, setConfirmName] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const canDelete = confirmName.trim() === project && password.length > 0;

  async function handleDelete(e) {
    e.preventDefault();
    if (!canDelete || busy) return;
    setBusy(true);
    setError("");
    try {
      const res = await requestJson(`/api/users/${owner}/projects/${project}`, {
        method: "DELETE",
        body: JSON.stringify({ project_name: confirmName.trim(), password }),
      });
      if (res?.ok) {
        window.location.href = "/home";
      }
    } catch (e) {
      setError((e as any)?.message || "Deletion failed");
      setBusy(false);
    }
  }

  return (
    <SettingsSection
      title="Danger Zone"
      description="Permanently delete this project and all its data. This cannot be undone."
      tone="danger"
    >
      <form onSubmit={handleDelete} className="flex flex-col gap-3 max-w-sm">
        <Field label={`Type "${project}" to confirm`} id="danger-confirm-name">
          <Input
            id="danger-confirm-name"
            type="text"
            value={confirmName}
            onInput={(e) => setConfirmName((e.target as HTMLInputElement).value)}
            placeholder={project}
            autoComplete="off"
          />
        </Field>
        <Field label="Your password" id="danger-password">
          <Input
            id="danger-password"
            type="password"
            value={password}
            onInput={(e) => setPassword((e.target as HTMLInputElement).value)}
            placeholder="Enter your password"
            autoComplete="current-password"
          />
        </Field>
        {error && <p className="text-[0.72rem] text-red-400">{error}</p>}
        <Button
          type="submit"
          variant="destructive"
          disabled={!canDelete || busy}
          className="self-start"
        >
          {busy ? "Deleting…" : "Delete project"}
        </Button>
      </form>
    </SettingsSection>
  );
}
