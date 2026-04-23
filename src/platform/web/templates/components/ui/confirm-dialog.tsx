import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";

/**
 * Reusable confirmation dialog.
 *
 * - Yes/No mode  → provide both `confirmLabel` and `cancelLabel` (defaults)
 * - OK-only mode → set `cancelLabel={null}`
 * - Destructive  → set `variant="destructive"` to style the confirm button red
 */
export default function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  variant = "default",
}: any) {
  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div className="px-6 py-4">
          {message ? <p className="text-[0.82rem] text-body-soft">{message}</p> : null}
        </div>
        <DialogFooter>
          {cancelLabel ? (
            <Button variant="outline" size="sm" onClick={onClose}>{cancelLabel}</Button>
          ) : null}
          <Button
            variant={variant === "destructive" ? "destructive" : "primary"}
            size="sm"
            onClick={() => { onConfirm(); onClose(); }}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
