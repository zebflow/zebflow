import ConfirmDialog from "@/components/ui/confirm-dialog";

/**
 * What the reader typed that the engine will not accept, with an example of
 * what it wanted instead. Acknowledged, not answered — there is nothing to
 * confirm, so both buttons mean the same thing.
 */
export default function ValidationNoticeDialog({ notice, onClose }) {
  return (
    <ConfirmDialog
      open={!!notice}
      onClose={onClose}
      onConfirm={onClose}
      title={notice?.title || "Invalid Input"}
      message={notice?.message || ""}
      confirmLabel="OK"
      cancelLabel={null}
    >
      {notice?.example ? (
        <div className="mt-3">
          <p className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">Example</p>
          <pre className="overflow-auto rounded-md border border-border/70 bg-accent/30 p-3 text-xs text-foreground">{notice.example}</pre>
        </div>
      ) : null}
    </ConfirmDialog>
  );
}
