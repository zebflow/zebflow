import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import CheckboxField from "@/components/ui/checkbox-field";
import Input from "@/components/ui/input";
import HubSourcesDialog from "@/pages/project-studio/hub/components/hub-sources-dialog";
import HubSourceLists from "@/pages/project-studio/hub/components/hub-source-lists";
import { slugify } from "@/pages/project-studio/hub/components/hub-slug";

function blankProjectSource() {
  return {
    repository_id: "",
    title: "",
    base_url: "https://hub.zebflow.com/api",
    remote_owner: "",
    remote_project: "",
    read_token: "",
    enabled: true,
  };
}

/** Adding, editing and deleting the hub sources this project reads from. */
export default function HubSourcesPanel({ open, onClose, api, hubSources, refresh, showStatus }) {
  const [sourceForm, setSourceForm] = useState(blankProjectSource());
  const [editingSourceId, setEditingSourceId] = useState("");

  function editSource(item) {
    setEditingSourceId(item?.repository_id || "");
    setSourceForm({
      repository_id: item?.repository_id || "",
      title: item?.title || "",
      base_url: item?.base_url || "https://hub.zebflow.com/api",
      remote_owner: item?.remote_owner || "",
      remote_project: item?.remote_project || "",
      read_token: "",
      enabled: item?.enabled !== false,
    });
  }

  function resetSourceForm() {
    setEditingSourceId("");
    setSourceForm(blankProjectSource());
  }

  async function saveSource(event) {
    event?.preventDefault?.();
    if (!api.repositories) return;
    const repositoryId = sourceForm.repository_id || slugify(sourceForm.title || "hub-source");
    showStatus("Saving project Hub source...");
    try {
      await requestJson(api.repositories, {
        method: "POST",
        body: JSON.stringify({
          repository_id: repositoryId,
          title: sourceForm.title || repositoryId,
          base_url: sourceForm.base_url,
          remote_owner: sourceForm.remote_owner,
          remote_project: sourceForm.remote_project,
          read_token: sourceForm.read_token,
          enabled: !!sourceForm.enabled,
        }),
      });
      resetSourceForm();
      await refresh();
      showStatus("Project Hub source saved.");
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  async function deleteSource(item) {
    const repositoryId = item?.repository_id;
    if (!repositoryId || !api.repositories || item?.editable === false) return;
    if (!window.confirm(`Delete project Hub source ${repositoryId}?`)) return;
    showStatus(`Deleting ${repositoryId}...`);
    try {
      await requestJson(`${api.repositories}/${encodeURIComponent(repositoryId)}`, { method: "DELETE" });
      if (editingSourceId === repositoryId) resetSourceForm();
      await refresh();
      showStatus("Project Hub source deleted.");
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  return (
    <HubSourcesDialog open={open} onClose={onClose}>
      <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="project-content-subtitle">Hub Sources</p>
          <p className="text-sm text-muted-foreground">Project sources are private to this project. Shared sources are granted from Home &gt; Hub and are read-only here.</p>
        </div>
        <Button type="button" variant="outline" size="sm" onClick={resetSourceForm}>
          New Project Source
        </Button>
      </div>

      <form className="mt-4 grid gap-3 md:grid-cols-2" onSubmit={saveSource}>
        <Field label="Source ID">
          <Input
            value={sourceForm.repository_id}
            onInput={(e) => setSourceForm((prev) => ({ ...prev, repository_id: slugify(e.currentTarget.value) }))}
            placeholder="my-private-hub"
          />
        </Field>
        <Field label="Title">
          <Input
            value={sourceForm.title}
            onInput={(e) => setSourceForm((prev) => ({ ...prev, title: e.currentTarget.value }))}
            placeholder="My Private Hub"
          />
        </Field>
        <Field label="Base URL">
          <Input
            value={sourceForm.base_url}
            onInput={(e) => setSourceForm((prev) => ({ ...prev, base_url: e.currentTarget.value }))}
            placeholder="https://hub.zebflow.com/api"
          />
        </Field>
        <Field label="Read Token">
          <Input
            type="password"
            value={sourceForm.read_token}
            onInput={(e) => setSourceForm((prev) => ({ ...prev, read_token: e.currentTarget.value }))}
            placeholder={editingSourceId ? "Leave blank to keep existing token" : "Optional"}
          />
        </Field>
        <Field label="Remote owner">
          <Input
            value={sourceForm.remote_owner}
            onInput={(e) => setSourceForm((prev) => ({ ...prev, remote_owner: e.currentTarget.value }))}
            placeholder="Only for legacy project-coordinate Hub"
          />
        </Field>
        <Field label="Remote project">
          <Input
            value={sourceForm.remote_project}
            onInput={(e) => setSourceForm((prev) => ({ ...prev, remote_project: e.currentTarget.value }))}
            placeholder="Only for legacy project-coordinate Hub"
          />
        </Field>
        <CheckboxField
          label="Enabled"
          checked={sourceForm.enabled}
          onChange={(e) => setSourceForm((prev) => ({ ...prev, enabled: e.currentTarget.checked }))}
        />
        <div className="flex items-center justify-end gap-2">
          {editingSourceId ? <Button type="button" variant="outline" size="sm" onClick={resetSourceForm}>Cancel Edit</Button> : null}
          <Button type="submit" size="sm" variant="primary">{editingSourceId ? "Save Source" : "Add Project Source"}</Button>
        </div>
      </form>

      <HubSourceLists hubSources={hubSources} onEdit={editSource} onDelete={deleteSource} />
    </HubSourcesDialog>
  );
}
