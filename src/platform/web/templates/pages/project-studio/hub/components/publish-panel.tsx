import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";
import { SOURCE_TYPES } from "@/pages/project-studio/hub/components/publish-source-types";
import PublishPreview from "@/pages/project-studio/hub/components/publish-preview";
import PublishDetails from "@/pages/project-studio/hub/components/publish-details";

/**
 * The Publish tab: choose what to pack, see the exact file set, then review
 * and publish. `publish` is `usePublishForm()`.
 */
export default function PublishPanel({ publish }) {
  const publishForm = publish.form;
  const patchPublishForm = publish.patch;
  const selectedSource = publish.sources.find((item) => item.source_ref === publishForm.source_ref) || null;
  const selectedType = SOURCE_TYPES.find((item) => item.value === publishForm.source_type) || SOURCE_TYPES[0];

  return (
    <form className="space-y-4" onSubmit={publish.publishAsset}>
      <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
        <div>
          <p className="project-content-subtitle">1. Choose Source Type</p>
          <p className="text-sm text-muted-foreground">Pick the export scope first, then choose the specific item and review the final file tree.</p>
        </div>
        <Field label="Source type">
          <Select
            value={publishForm.source_type}
            onChange={(e) => patchPublishForm({ source_type: e.target.value, source_ref: "" })}
          >
            {SOURCE_TYPES.map((item) => (
              <SelectOption key={item.value} value={item.value} label={item.label} />
            ))}
          </Select>
        </Field>
        <div className="rounded-lg border border-border bg-accent/30 px-3 py-2 text-sm text-muted-foreground">
          <span className="font-medium text-foreground">{selectedType.label}:</span> {selectedType.note}
        </div>
      </div>

      <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
        <div>
          <p className="project-content-subtitle">2. Select Item</p>
          <p className="text-sm text-muted-foreground">Only name, description, and path are shown here. The actual export set is resolved in the preview step below.</p>
        </div>
        <Field label="Selected item">
          <Select
            value={publishForm.source_ref}
            onChange={(e) => patchPublishForm({ source_ref: e.target.value })}
          >
            <SelectOption value="" label="Select item" />
            {publish.sources.map((item, index) => (
              <SelectOption
                key={`${item?.source_ref ?? "src"}-${index}`}
                value={item?.source_ref}
                label={`${item?.name} · ${item?.path}`}
              />
            ))}
          </Select>
        </Field>
        {selectedSource ? (
          <div className="grid gap-3 md:grid-cols-3 rounded-lg border border-border bg-accent/20 px-3 py-3 text-sm">
            <div>
              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Name</div>
              <div className="mt-1 text-foreground">{selectedSource.name}</div>
            </div>
            <div>
              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Description</div>
              <div className="mt-1 text-foreground">{selectedSource.description}</div>
            </div>
            <div>
              <div className="text-muted-foreground uppercase tracking-wider text-[11px]">Path</div>
              <code className="mt-1 block text-xs text-foreground">{selectedSource.path}</code>
            </div>
          </div>
        ) : null}
      </div>

      <PublishPreview publishPreview={publish.preview} />

      <PublishDetails publish={publish} />
    </form>
  );
}
