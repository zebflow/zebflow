import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";

/** The upload address, asking it to replace a built-in library when one is chosen. */
function uploadUrl(api, replace: string, entry: string) {
  if (!replace) return api.libraries_upload;
  const query = new URLSearchParams({ override: replace });
  if (entry.trim()) query.set("entry", entry.trim());
  return `${api.libraries_upload}?${query.toString()}`;
}

/** What the check found, in one sentence a reader can act on. */
function overrideSummary(report) {
  const files = report.files_checked?.length || 0;
  const added = report.added_names ? `, plus ${report.added_names} new` : "";
  const checked = files ? ` ${files} ${files === 1 ? "file that imports it still checks" : "files that import it still check"}.` : "";
  return `Replaces ${report.library} in this project's pages with ${report.with}/${report.entry}: it has all ${report.built_in_names} names of the built-in library${added}.${checked}`;
}

/**
 * Uploading the project's own library: the .tar.gz zeb-bundler builds from a
 * library recipe. Zebflow scans and checks every file, then installs it for
 * this project only; the answer names what pages may import. It may also
 * replace a built-in library, once it has that library's whole interface.
 */
export default function UploadLibraryDialog({ open, onClose, api, replaceable, onUploaded }) {
  const [file, setFile] = useState<any>(null);
  const [replace, setReplace] = useState("");
  const [entry, setEntry] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<any>(null);
  const [error, setError] = useState("");

  function close() {
    setFile(null);
    setReplace("");
    setEntry("");
    setResult(null);
    setError("");
    onClose();
  }

  async function upload() {
    if (!file || busy) return;
    setBusy(true);
    setError("");
    setResult(null);
    try {
      const payload = await requestJson(uploadUrl(api, replace, entry), {
        method: "POST",
        body: file,
        headers: { "Content-Type": "application/gzip" },
      });
      setResult({ ...(payload?.library || {}), override: payload?.override || null });
      // Done: the same file is not sent twice by a second click.
      setFile(null);
      onUploaded();
    } catch (err: any) {
      setError(err?.message || String(err));
      // The library may be installed even though it could not replace one.
      onUploaded();
    } finally {
      setBusy(false);
    }
  }

  const library = result?.name || "";
  const installedOnly = error.includes("is installed, but it does not replace");
  const choices = Array.isArray(replaceable) ? replaceable : [];
  return (
    <Dialog open={open} onOpenChange={(next) => (next ? null : close())}>
      <DialogContent className="max-w-[42rem]">
        <DialogHeader>
          <DialogTitle>Upload a library</DialogTitle>
          <p className="text-sm text-muted-foreground">
            The .tar.gz zeb-bundler builds from a library recipe. Every file is scanned and checked
            against its fingerprint, then the library is installed for this project only.
          </p>
        </DialogHeader>
        <div className="grid gap-3 px-6 py-4">
          <input
            type="file"
            accept=".tar.gz,.tgz,application/gzip"
            disabled={busy}
            onChange={(e) => { setFile(e.target.files?.[0] || null); setResult(null); setError(""); }}
            className="text-sm text-foreground"
          />
          <Field
            id="upload-library-replace"
            label="Replace a built-in library"
            description="Pages that import it load this library instead, in this project only. It must export every name the built-in one does; removing it brings the built-in one back."
          >
            <Select id="upload-library-replace" value={replace} disabled={busy} onChange={(e) => { setReplace(e.currentTarget.value); setError(""); }}>
              <SelectOption value="">No, just install it</SelectOption>
              {choices.map((name) => <SelectOption key={name} value={name}>{name}</SelectOption>)}
            </Select>
          </Field>
          {replace ? (
            <Field
              id="upload-library-entry"
              label="Entry (optional)"
              description="Leave empty and the entry that exports every name of the built-in library is used. Name one only if several do."
            >
              <Input id="upload-library-entry" value={entry} disabled={busy} placeholder="found for you" onInput={(e) => setEntry(e.currentTarget.value)} />
            </Field>
          ) : null}
          {error ? (
            <div className={installedOnly
              ? "rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-sm text-warning"
              : "rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"}>
              {installedOnly ? "Installed, but not used as a replacement. " : "Not installed. "}{error}
            </div>
          ) : null}
          {result ? (
            <div className="rounded-lg border border-border bg-accent/20 px-3 py-2 text-sm text-foreground">
              <p className="m-0">
                {result.changed ? "Installed" : "Already installed"}: <span className="font-mono">{library} {result.version}</span>, {result.files} {result.files === 1 ? "file" : "files"}.
              </p>
              {result.override ? <p className="m-0 mt-1 text-success">{overrideSummary(result.override)}</p> : null}
              <p className="m-0 mt-1 text-muted-foreground">Pages import:</p>
              <ul className="m-0 list-disc pl-5 font-mono text-xs">
                {result.override ? (
                  <li key="replaced">
                    {result.override.library} <span className="font-sans text-muted-foreground">(now loads this library)</span>
                  </li>
                ) : null}
                {(result.entries || []).map((name) => <li key={name}>{`${library}/${name}`}</li>)}
              </ul>
            </div>
          ) : null}
        </div>
        <DialogFooter>
          {result ? (
            <Button type="button" size="sm" onClick={close}>Done</Button>
          ) : (
            <>
              <Button type="button" variant="outline" size="sm" onClick={close} disabled={busy}>Close</Button>
              <Button type="button" size="sm" onClick={upload} disabled={!file || busy}>
                {busy ? "Checking…" : replace ? "Upload and replace" : "Upload"}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
