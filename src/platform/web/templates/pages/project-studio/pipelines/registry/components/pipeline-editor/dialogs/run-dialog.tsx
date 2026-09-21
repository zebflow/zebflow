import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Textarea from "@/components/ui/textarea";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";

/**
 * The Run dialog for a pipeline that declares no inputs: one JSON box for
 * the payload the manual trigger receives. A pipeline with `input.*` nodes
 * never opens this — its form is drawn under the nodes on the canvas.
 */

interface RunDialogProps {
  open: boolean;
  busy: boolean;
  onRun: (input: unknown) => void;
  onClose: () => void;
}

export default function RunDialog({ open, busy, onRun, onClose }: RunDialogProps) {
  const [text, setText] = useState("{}");
  const [error, setError] = useState("");
  if (!open) return null;

  function handleRun() {
    let parsed: unknown = {};
    const raw = text.trim();
    if (raw) {
      try {
        parsed = JSON.parse(raw);
      } catch (err: any) {
        setError(`Not JSON: ${err?.message || String(err)}`);
        return;
      }
    }
    setError("");
    onRun(parsed);
  }

  return (
    <Dialog open={true} onOpenChange={(next: boolean) => !next && onClose()}>
      <DialogContent className="max-w-lg border-border bg-card text-foreground" onKeyDown={(e) => e.stopPropagation()}>
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Run pipeline</DialogTitle>
          <p className="text-xs text-muted-foreground">
            The payload the manual trigger receives. Add <code>input.*</code> nodes after the trigger to get a form here instead.
          </p>
        </DialogHeader>
        <div className="px-6 py-4">
          <Textarea
            rows={8}
            value={text}
            spellcheck={false}
            className="w-full font-mono text-xs"
            onInput={(e) => setText(e.currentTarget.value)}
          />
          {error ? (
            <p className="mt-2 text-xs text-red-400" role="alert">
              {error}
            </p>
          ) : null}
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
          <Button variant="outline" size="xs" type="button" onClick={onClose}>
            Cancel
          </Button>
          <Button size="xs" type="button" disabled={busy} onClick={handleRun}>
            {busy ? "Running…" : "Run"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
