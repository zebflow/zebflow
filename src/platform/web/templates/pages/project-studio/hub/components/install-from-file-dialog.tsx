import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";

/**
 * Installing a node bundle you already have as a file.
 *
 * Behind a button rather than open on the page: it is the rare path — most
 * people came to the Hub to browse — and as a permanent block it pushed the
 * catalogue below the fold.
 *
 * Composed as header / body / footer rather than dropping content straight into
 * `DialogContent`: the padding lives in those three, so content placed directly
 * inside sits flush against the edge.
 */
export default function InstallFromFileDialog({ open, onClose, children }) {
  return (
    <Dialog open={open} onOpenChange={(next) => (next ? null : onClose())}>
      <DialogContent className="max-w-[42rem]">
        <DialogHeader>
          <DialogTitle>Install a node bundle from a file</DialogTitle>
          <p className="text-sm text-muted-foreground">
            It runs the same review a published package runs. A Hub package is not safer,
            only published.
          </p>
        </DialogHeader>
        <div className="px-6 py-4">{children}</div>
        <DialogFooter>
          <Button type="button" variant="outline" size="sm" onClick={onClose}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
