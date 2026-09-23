import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CommitDialog from "@/components/ui/commit-dialog";
import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";

/**
 * Where this project keeps its files, and the control that moves it.
 *
 * Lives with the Files page rather than in Settings: it describes the store you
 * are looking at. The two icons are used nowhere else, so they stay here.
 *
 * Two backends: the local `files/` directory, or an S3-compatible bucket
 * reached through a credential of kind `s3`. Saving writes the word into
 * `repo/zebflow.yaml` (committed) and the credential choice into this
 * instance's `data/store/files-backend.json` (not committed), after the
 * server has opened the bucket once to prove the credential reaches it.
 */
export function DiskIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <ellipse cx="12" cy="6" rx="8" ry="3" />
      <path d="M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6" />
      <path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" />
    </svg>
  );
}

export function BucketIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 8V16a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8"/>
      <path d="M3 8l9-5 9 5"/>
      <path d="M12 3v18"/>
    </svg>
  );
}

const BACKENDS = [
  { value: "zebfs", label: "Local disk", blurb: "Objects live under this project's files/ directory on this machine." },
  { value: "s3", label: "Object store (S3)", blurb: "Objects live in an S3-compatible bucket: AWS S3, Cloudflare R2, MinIO, SeaweedFS, Garage, B2, Tigris." },
];

export default function StorageBackendPanel({ storage }) {
  const active = String(storage?.backend ?? "zebfs");
  const activeLabel = String(storage?.backend_label ?? "Local disk");
  const credentials = Array.isArray(storage?.credentials) ? storage.credentials : [];
  const [backend, setBackend] = useState(active);
  const [credentialId, setCredentialId] = useState(String(storage?.credential_id ?? ""));
  const [statusMsg, setStatusMsg] = useState(storage?.problem ? String(storage.problem) : "");
  const [statusTone, setStatusTone] = useState(storage?.problem ? "error" : "info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [saved, setSaved] = useState(null);

  const shown = saved ?? storage;
  const shownBackend = String(shown?.backend ?? active);
  const dirty = backend !== shownBackend || (backend === "s3" && credentialId !== String(shown?.credential_id ?? ""));
  const needsCredential = backend === "s3" && !credentialId;

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Opening the store...");
    setStatusTone("info");
    try {
      const response = await requestJson(storage?.api, {
        method: "PUT",
        body: JSON.stringify({
          commit_message: commitMessage,
          data: { backend, credential_id: backend === "s3" ? credentialId : null },
        }),
      });
      setSaved(response?.data ?? null);
      const problem = response?.data?.problem;
      setStatusMsg(problem ? String(problem) : response?.committed ? "Saved & committed." : response?.git_error ? `Saved (git: ${response.git_error})` : "Saved.");
      setStatusTone(problem ? "error" : "ok");
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
    }
  }

  const toneClass = statusTone === "error" ? "text-destructive" : statusTone === "ok" ? "text-success" : "text-muted-foreground";

  return (
    <div className="project-settings-panel">
      <CommitDialog
        open={commitOpen}
        section="files"
        defaultMessage={`settings(files): keep files on ${backend === "s3" ? "an object store" : "local disk"}`}
        onConfirm={handleCommit}
        onCancel={() => setCommitOpen(false)}
      />
      <div className="project-settings-panel-head">
        <p className="project-card-label">File Storage Backend</p>
        <Badge variant="outline">{shownBackend}</Badge>
      </div>
      <div className="project-settings-panel-body flex flex-col gap-6 pt-2">
        <p className="text-[0.78rem] text-muted-foreground">
          Where this project keeps its own files. Every object under{" "}
          <code className="font-mono text-[0.75rem]">files/</code> lives in this store, and every FileRef a
          pipeline passes around carries its name in <code className="font-mono text-[0.75rem]">backend</code>.
          A bucket a pipeline reads from is a different thing — that is a connection with a credential,
          chosen per pipeline, and the bytes it fetches still land here.
        </p>

        <Card className="border-success/40 bg-success/5">
          <CardContent className="flex items-start gap-4 pt-5">
            <div className="mt-0.5 rounded bg-success/15 p-2 text-success">
              {shownBackend === "s3" ? <BucketIcon /> : <DiskIcon />}
            </div>
            <div className="flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <p className="text-[0.88rem] font-semibold text-foreground">{String(shown?.backend_label ?? activeLabel)}</p>
                <Badge variant="outline" className="text-[0.72rem] border-success/50 bg-success/10 text-success">Active</Badge>
              </div>
              {shown?.bucket ? (
                <p className="mt-0.5 text-[0.78rem] text-muted-foreground">
                  <code className="font-mono text-[0.75rem]">{String(shown.bucket.endpoint)}</code>{" "}
                  bucket <code className="font-mono text-[0.75rem]">{String(shown.bucket.bucket)}</code>
                  {shown.bucket.prefix ? <> under <code className="font-mono text-[0.75rem]">{String(shown.bucket.prefix)}/</code></> : null}
                  , served through <code className="font-mono text-[0.75rem]">/fs/...</code> under this project&apos;s access rules.
                </p>
              ) : (
                <p className="mt-0.5 text-[0.78rem] text-muted-foreground">
                  Objects are stored on this machine&apos;s disk and served through{" "}
                  <code className="font-mono text-[0.75rem]">/fs/...</code> under this project&apos;s access rules.
                </p>
              )}
              <p className="mt-2 text-[0.75rem] text-muted-foreground">
                <code className="font-mono text-[0.75rem]">{String(shown?.field ?? "spec.files.backend")}: {shownBackend}</code>{" "}
                {shown?.declared ? "declared in" : "— the default; not declared in"}{" "}
                <code className="font-mono text-[0.75rem]">repo/zebflow.yaml</code>
              </p>
            </div>
          </CardContent>
        </Card>

        <form
          className="grid grid-cols-2 gap-[0.65rem]"
          onSubmit={(event) => { event.preventDefault(); if (!needsCredential) setCommitOpen(true); }}
        >
          <Field label="Store" id="files-backend">
            <Select id="files-backend" value={backend} onChange={(event) => setBackend(event.currentTarget.value)}>
              {BACKENDS.map((option) => (
                <SelectOption key={option.value} value={option.value} label={option.label} />
              ))}
            </Select>
          </Field>
          {backend === "s3" ? (
            <Field label="Credential" id="files-credential" description="A credential of kind s3: endpoint, bucket, region, prefix and keys.">
              <Select id="files-credential" value={credentialId} onChange={(event) => setCredentialId(event.currentTarget.value)}>
                <SelectOption value="" label={credentials.length ? "Choose a credential…" : "No s3 credential in this project yet"} />
                {credentials.map((credential) => (
                  <SelectOption key={credential.id} value={credential.id} label={`${credential.title} (${credential.id})`} />
                ))}
              </Select>
            </Field>
          ) : (
            <div />
          )}
          <p className="col-span-full text-[0.75rem] text-muted-foreground">
            {BACKENDS.find((option) => option.value === backend)?.blurb}
            {backend === "s3" ? (
              <>
                {" "}Add one under{" "}
                <a href={String(storage?.credentials_href ?? "#")} className="text-info underline-offset-4 hover:underline">Credentials</a>.
                Saving opens the bucket once to prove the credential reaches it. Existing objects are not moved.
              </>
            ) : null}
          </p>
          <div className="col-span-full flex items-center gap-[0.7rem]">
            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={saving || !dirty || needsCredential}
              label={saving ? "Saving..." : "Save Store"}
            />
            <span className={cx("text-[0.72rem]", toneClass)}>{statusMsg}</span>
          </div>
        </form>
      </div>
    </div>
  );
}
