import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import CheckboxField from "@/components/ui/checkbox-field";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import TraceCaptureFields from "@/components/ui/trace-capture-fields";
import { traceCaptureFormConfig, traceCaptureFormValues } from "@/components/lib/trace-capture";

/** Mounted on opening so Cancel discards local edits; Apply updates the pipeline draft. */
export default function PipelineSettingsDialog({ metadata, defaults, locked, onApply, onClose, onDelete }) {
  const retention = metadata?.settings?.invocation_retention;
  const [inherit, setInherit] = useState(!retention?.max_invocations && !retention?.max_age_secs);
  const [maxInv, setMaxInv] = useState(retention?.max_invocations ? String(retention.max_invocations) : "");
  const [maxAge, setMaxAge] = useState(retention?.max_age_secs ? String(Math.max(1, Math.round(Number(retention.max_age_secs) / 86400))) : "");
  const [captureValues, setCaptureValues] = useState(traceCaptureFormValues(metadata?.settings?.trace_capture));
  const [error, setError] = useState("");

  function applySettings(event) {
    event.preventDefault();
    try {
      const settings = { ...(metadata?.settings || {}) };
      const count = Number(maxInv || 0);
      const days = Number(maxAge || 0);
      if (!inherit && (!Number.isInteger(count) || count < 0 || count > 1000 || !Number.isInteger(days) || days < 0)) {
        throw new Error("Retention count must be at most 1000, and count and days must be positive whole numbers or blank.");
      }
      if (inherit || (!count && !days)) delete settings.invocation_retention;
      else settings.invocation_retention = {
        ...(count > 0 ? { max_invocations: count } : {}),
        ...(days > 0 ? { max_age_secs: days * 86400 } : {}),
      };
      const capture = traceCaptureFormConfig(captureValues);
      if (Object.keys(capture).length) settings.trace_capture = capture;
      else delete settings.trace_capture;
      const next = { ...(metadata || {}) };
      if (Object.keys(settings).length) next.settings = settings;
      else delete next.settings;
      onApply(next);
      onClose();
    } catch (err) {
      setError(err?.message || String(err));
    }
  }

  return (
    <Dialog open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto border-border bg-surface text-body">
        <form className="flex flex-col gap-4" onSubmit={applySettings}>
          <DialogHeader className="px-6 pt-6">
            <DialogTitle>Pipeline Settings</DialogTitle>
            <p className="text-xs text-body-muted">Configure log capture and retention for this pipeline. Apply, save, and activate the draft to use these settings.</p>
          </DialogHeader>
          <div className="grid gap-3 px-6">
            <CheckboxField
              label="Use project default retention"
              description={`Current inherited count limit: ${defaults?.max_invocations ?? 20} invocation(s) per pipeline.`}
              checked={inherit}
              onChange={(e) => setInherit(e.currentTarget.checked)}
            />
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <label className="pipeline-editor-field">
                <span>Max invocation count override</span>
                <input className="zf-input" type="number" min="1" max="1000" step="1" placeholder={String(defaults?.max_invocations ?? 20)} value={maxInv} disabled={inherit} onInput={(e) => setMaxInv(e.currentTarget.value)} />
                <small className="pipeline-editor-field-help">Optional cap on retained runs.</small>
              </label>
              <label className="pipeline-editor-field">
                <span>Max age override (days)</span>
                <input className="zf-input" type="number" min="1" step="1" placeholder="1" value={maxAge} disabled={inherit} onInput={(e) => setMaxAge(e.currentTarget.value)} />
                <small className="pipeline-editor-field-help">Optional time limit for retained runs.</small>
              </label>
            </div>
            <TraceCaptureFields values={captureValues} onChange={setCaptureValues} defaults={defaults?.trace_capture} scope="pipeline" />
            {error ? <p role="alert" className="text-xs text-red-400">{error}</p> : null}
          </div>
          <div className="mx-6 flex items-center justify-between gap-3 border-t border-border-soft pt-4">
            <div>
              <div className="text-xs font-semibold uppercase tracking-[0.12em] text-body-muted">Danger Zone</div>
              <div className="text-xs text-body-muted">Delete this pipeline from the project.</div>
            </div>
            {locked ? <span className="text-xs text-body-muted">Locked — cannot delete</span> : onDelete ? (
              <Button variant="destructive" size="xs" type="button" onClick={() => { onClose(); onDelete(); }}>Delete Pipeline</Button>
            ) : null}
          </div>
          <div className="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
            <Button variant="outline" size="xs" type="button" onClick={onClose}>Cancel</Button>
            <Button variant="primary" size="xs" type="submit">Apply to Draft</Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
