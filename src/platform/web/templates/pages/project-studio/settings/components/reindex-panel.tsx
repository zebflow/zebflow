import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** Rebuilding the search index by hand. */
export default function ReIndexPanel({ api }) {
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);

  async function handleReindex() {
    setBusy(true);
    setResult(null);
    setError(null);
    try {
      const data = await requestJson(api, { method: "POST" });
      setResult(data);
    } catch (err) {
      setError((err as any)?.message || String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <SettingsSection
      title="Re-index from Files"
      description={
        <>
            Scan <code>repo/pipelines/</code> on disk and register all found files into the catalog.
            Use this after a DB wipe, crash recovery, or restoring from a git backup.
        </>
      }
      tag="Recovery"
    >
      <div className="flex flex-col gap-[0.55rem]">
        <div className="flex items-center gap-[0.7rem]">
          <Button
            variant="outline"
            size="sm"
            disabled={busy}
            label={busy ? "Indexing..." : "Run Re-index"}
            onClick={handleReindex}
          />
          {error ? (
            <span className="text-[0.72rem] text-red-300">{error}</span>
          ) : null}
        </div>
        {result ? (
          <div className="flex items-center gap-[0.55rem] flex-wrap">
            <span className="project-inline-chip">{result.pipelines} pipeline{result.pipelines !== 1 ? "s" : ""}</span>
            <span className="project-inline-chip">{result.templates} template{result.templates !== 1 ? "s" : ""}</span>
            <span className="project-inline-chip">{result.assets} asset{result.assets !== 1 ? "s" : ""}</span>
            {Array.isArray(result.errors) && result.errors.length > 0 ? (
              <span className="text-[0.72rem] text-red-300">{result.errors.length} error{result.errors.length !== 1 ? "s" : ""}</span>
            ) : (
              <span className="text-[0.72rem] text-dark-accent2">Done.</span>
            )}
          </div>
        ) : null}
      </div>
    </SettingsSection>
  );
}
