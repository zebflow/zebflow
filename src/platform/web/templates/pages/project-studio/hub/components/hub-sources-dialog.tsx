import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";

/**
 * Which hubs this project reads from.
 *
 * Configuration, not browsing — you set it once and then forget it, so it does
 * not earn permanent space above the catalogue.
 *
 * Composed as header / body / footer rather than dropping content straight into
 * `DialogContent`: the padding lives in those three, so content placed directly
 * inside sits flush against the edge.
 */
export default function HubSourcesDialog({ open, onClose, children }) {
  return (
    <Dialog open={open} onOpenChange={(next) => (next ? null : onClose())}>
      <DialogContent className="max-w-[52rem]">
        <DialogHeader>
          <DialogTitle>Hub sources</DialogTitle>
          <p className="text-sm text-ui-text-soft">
            Project sources are private to this project. Shared sources are granted from
            Home &gt; Hub and are read-only here.
          </p>
        </DialogHeader>
        <div className="max-h-[60vh] overflow-y-auto px-6 py-4">{children}</div>
        <DialogFooter>
          <Button type="button" variant="outline" size="sm" onClick={onClose}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
