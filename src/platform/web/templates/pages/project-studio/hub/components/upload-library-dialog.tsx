import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";

/**
 * Uploading the project's own library: the .tar.gz zeb-bundler builds from a
 * library recipe. Zebflow scans and checks every file, then installs it for
 * this project only; the answer names what pages may import.
 */
export default function UploadLibraryDialog({ open, onClose, api, onUploaded }) {
  const [file, setFile] = useState<any>(null);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<any>(null);
  const [error, setError] = useState("");

  function close() {
    setFile(null);
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
      const payload = await requestJson(api.libraries_upload, {
        method: "POST",
        body: file,
        headers: { "Content-Type": "application/gzip" },
      });
      setResult(payload?.library || null);
      onUploaded();
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setBusy(false);
    }
  }

  const library = result?.name || "";
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
          {error ? <p className="m-0 text-sm text-danger">Refused: {error}</p> : null}
          {result ? (
            <div className="rounded-lg border border-border bg-accent/20 px-3 py-2 text-sm text-foreground">
              <p className="m-0">
                {result.changed ? "Installed" : "Already installed"}: <span className="font-mono">{library} {result.version}</span>, {result.files} files.
              </p>
              <p className="m-0 mt-1 text-muted-foreground">Pages import:</p>
              <ul className="m-0 list-disc pl-5 font-mono text-xs">
                {(result.entries || []).map((entry) => <li key={entry}>{`${library}/${entry}`}</li>)}
              </ul>
            </div>
          ) : null}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" size="sm" onClick={close} disabled={busy}>Close</Button>
          <Button type="button" size="sm" onClick={upload} disabled={!file || busy}>{busy ? "Checking…" : "Upload"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
