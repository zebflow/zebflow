import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
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
        <div className="flex flex-col gap-1.5">
          <p className="font-semibold text-body text-[0.95rem]">{title}</p>
          {message ? <p className="text-[0.82rem] text-body-soft">{message}</p> : null}
        </div>
        <div className="flex justify-end gap-2 pt-2">
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
        </div>
      </DialogContent>
    </Dialog>
  );
}
