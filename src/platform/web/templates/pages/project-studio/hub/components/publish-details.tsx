import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";
import Input from "@/components/ui/input";
import PublishInitOptions from "@/pages/project-studio/hub/components/publish-init-options";
import PublishReview from "@/pages/project-studio/hub/components/publish-review";

/**
 * Step 4 of publishing: the package's name, token and metadata, the review
 * and publish buttons, and the review result. `publish` is `usePublishForm()`.
 */
export default function PublishDetails({ publish }) {
  const publishForm = publish.form;
  const patchPublishForm = publish.patch;
  const publishPreview = publish.preview;
  const publishReview = publish.review;
  const publishReviewDirty = publish.reviewDirty;
  const imageUploadBusy = publish.imageUploadBusy;

  return (
    <div className="rounded-lg border border-border bg-popover p-4 space-y-4">
      <div>
        <p className="project-content-subtitle">4. Publish Package</p>
        <p className="text-sm text-muted-foreground">Use a publisher token issued from Home &gt; Hub. Project sources can be added on the Browse tab; publishers and tokens stay in Home &gt; Hub.</p>
      </div>
      {publishForm.source_type === "project_files" ? <PublishInitOptions publish={publish} /> : null}
      <Field label="Publisher Token">
        <Input type="password" value={publishForm.publisher_token} onInput={(e) => patchPublishForm({ publisher_token: e.target.value })} placeholder="zfmt_..." />
      </Field>
      <div className="grid gap-3 md:grid-cols-2">
        <Field label="Package ID">
          <Input value={publishForm.package_id} onInput={(e) => patchPublishForm({ package_id: e.target.value })} placeholder="site-a-catalog" />
        </Field>
        <Field label="Version">
          <Input value={publishForm.version} onInput={(e) => patchPublishForm({ version: e.target.value })} placeholder="0.1.0" />
        </Field>
      </div>
      <Field label="Title">
        <Input value={publishForm.title} onInput={(e) => patchPublishForm({ title: e.target.value })} placeholder="Site A Catalog Pipeline" />
      </Field>
      <Field label="Description">
        <Input value={publishForm.description} onInput={(e) => patchPublishForm({ description: e.target.value })} placeholder="Short summary" />
      </Field>
      <Field label="Package image from Files">
        <div className="space-y-2">
          <Input value={publishForm.image_file_path} onInput={(e) => patchPublishForm({ image_file_path: e.target.value })} placeholder="hub-media/cover.png" />
          <div className="flex items-center gap-2">
            <label className="inline-flex items-center">
              <input className="sr-only" type="file" accept="image/png,image/jpeg,image/webp,image/gif" onChange={publish.uploadCoverImage} disabled={imageUploadBusy} />
              <span className="zf-btn zf-btn-outline zf-btn-sm cursor-pointer">{imageUploadBusy ? "Uploading..." : "Upload image"}</span>
            </label>
            <span className="text-xs text-muted-foreground">Uploaded images are stored in Files and normalized to WebP in the Hub package.</span>
          </div>
        </div>
      </Field>
      <div className="grid gap-3 md:grid-cols-2">
        <Field label="Visibility">
          <Select value={publishForm.visibility} onChange={(e) => patchPublishForm({ visibility: e.target.value })}>
            <SelectOption value="private" label="private" />
            <SelectOption value="public" label="public" />
            <SelectOption value="unlisted" label="unlisted" />
          </Select>
        </Field>
        <Field label="Tags">
          <Input value={publishForm.tags_csv} onInput={(e) => patchPublishForm({ tags_csv: e.target.value })} placeholder="catalog, demo" />
        </Field>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button type="button" variant="outline" onClick={publish.reviewPublish} disabled={!publishForm.source_ref || !publishPreview?.entries?.length || !publishForm.publisher_token}>
          {publishReviewDirty ? "Review Package" : "Review Again"}
        </Button>
        <Button type="submit" disabled={!publishForm.source_ref || !publishPreview?.entries?.length || !publishForm.publisher_token || !publishReview || publishReviewDirty || !!publishReview?.violations?.length}>{publishReview?.violations?.length ? "Blocked" : "Publish Package"}</Button>
      </div>
      {publishReview ? <PublishReview publishReview={publishReview} publishReviewDirty={publishReviewDirty} /> : null}
    </div>
  );
}
