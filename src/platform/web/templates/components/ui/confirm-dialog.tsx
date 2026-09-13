import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";

/**
 * Reusable confirmation dialog. Every yes/no question in the platform asks it
 * here, so one answer to "does this scroll" and "which button is destructive"
 * holds everywhere.
 *
 * - Yes/No mode  → provide both `confirmLabel` and `cancelLabel` (defaults)
 * - OK-only mode → set `cancelLabel={null}`
 * - Destructive  → set `variant="destructive"` to style the confirm button red
 * - Long content → pass `children`; the body scrolls while the title and the
 *   buttons stay put, so a long list can never push the buttons out of reach
 * - Busy         → set `busy` to disable both buttons while the work runs
 * - Gated        → set `confirmDisabled` when the reader must do something
 *   first, such as typing a name to confirm a destructive action
 */
export default function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  message,
  children,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  variant = "default",
  busy = false,
  confirmDisabled = false,
}: any) {
  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        {/* Only the body scrolls. The title above and the buttons below stay
            in place, so content long enough to overflow still leaves the
            reader somewhere to click. */}
        <div className="min-h-0 max-h-[60vh] overflow-y-auto overscroll-contain px-6 py-4">
          {message ? <p className="text-[0.82rem] text-muted-foreground">{message}</p> : null}
          {children}
        </div>
        <DialogFooter>
          {cancelLabel ? (
            <Button variant="outline" size="sm" disabled={busy} onClick={onClose}>{cancelLabel}</Button>
          ) : null}
          <Button
            variant={variant === "destructive" ? "destructive" : "primary"}
            size="sm"
            disabled={busy || confirmDisabled}
            onClick={() => { onConfirm(); onClose(); }}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
