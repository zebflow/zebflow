import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import CommitDialog from "@/components/ui/commit-dialog";
import HelpTooltip from "@/components/ui/help-tooltip";
import Sonner from "@/components/ui/sonner";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

const DIALOG_SIZES = ["sm", "md", "lg", "xl", "wide"];

export default function OverlaysSection() {
  const [dialogSize, setDialogSize] = useState(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [dangerOpen, setDangerOpen] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [last, setLast] = useState("nothing yet");
  const [toasts, setToasts] = useState([
    { id: 1, msg: "Pipeline registered.", variant: "success" },
    { id: 2, msg: "Rebuilding templates…", variant: "info" },
    { id: 3, msg: "Credential expires in 3 days.", variant: "warning" },
    { id: 4, msg: "Webhook returned 500.", variant: "error" },
  ]);

  return (
    <div>
      <SectionHeading title="Overlays" description="Things that appear above the page and go away." />

      <Entry
        name="Dialog"
        file="dialog.tsx"
        description="A native <dialog> behind a controlled `open`. Header, Title, Description, Footer are the sections; `size` on DialogContent: sm | md | lg | xl | wide."
        code={`<Dialog open={open} onOpenChange={setOpen}>
  <DialogContent size="md">
    <DialogHeader>
      <DialogTitle>Rename pipeline</DialogTitle>
      <DialogDescription>The webhook path stays the same.</DialogDescription>
    </DialogHeader>
    <div className="px-6 py-4"><Input defaultValue="auth/login" /></div>
    <DialogFooter>
      <Button variant="ghost" onClick={() => setOpen(false)}>Cancel</Button>
      <Button onClick={() => setOpen(false)}>Rename</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>`}
      >
        <div className="flex flex-wrap items-center gap-3">
          {DIALOG_SIZES.map((size) => (
            <Button key={size} variant="outline" size="sm" onClick={() => setDialogSize(size)}>
              open {size}
            </Button>
          ))}
        </div>
        <Dialog open={dialogSize !== null} onOpenChange={(v) => { if (!v) setDialogSize(null); }}>
          <DialogContent size={dialogSize || "md"}>
            <DialogHeader>
              <DialogTitle>Rename pipeline</DialogTitle>
              <DialogDescription>size="{dialogSize}". The webhook path stays the same.</DialogDescription>
            </DialogHeader>
            <div className="px-6 py-4">
              <Input defaultValue="auth/login" />
            </div>
            <DialogFooter>
              <Button variant="ghost" onClick={() => setDialogSize(null)}>Cancel</Button>
              <Button onClick={() => setDialogSize(null)}>Rename</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </Entry>

      <Entry
        name="ConfirmDialog"
        file="confirm-dialog.tsx"
        description="A yes/no on top of Dialog. `variant='destructive'` paints the confirm button red; `busy` locks it while the action runs."
        code={`<ConfirmDialog
  open={open}
  onClose={() => setOpen(false)}
  onConfirm={() => { doIt(); setOpen(false); }}
  title="Delete pipeline?"
  message="auth/login and its 12 runs. This cannot be undone."
  confirmLabel="Delete"
  variant="destructive"
/>`}
      >
        <div className="flex flex-wrap items-center gap-3">
          <Button variant="outline" onClick={() => setConfirmOpen(true)}>Confirm…</Button>
          <Button variant="destructive" onClick={() => setDangerOpen(true)}>Delete…</Button>
          <span className="font-mono text-xs text-muted-foreground">last: {last}</span>
        </div>
        <ConfirmDialog
          open={confirmOpen}
          onClose={() => setConfirmOpen(false)}
          onConfirm={() => { setLast("activated"); setConfirmOpen(false); }}
          title="Activate pipeline?"
          message="The webhook goes live immediately."
          confirmLabel="Activate"
        />
        <ConfirmDialog
          open={dangerOpen}
          onClose={() => setDangerOpen(false)}
          onConfirm={() => { setLast("deleted"); setDangerOpen(false); }}
          title="Delete pipeline?"
          message="auth/login and its 12 runs. This cannot be undone."
          confirmLabel="Delete"
          variant="destructive"
        />
      </Entry>

      <Entry
        name="CommitDialog"
        file="commit-dialog.tsx"
        description="Asks for a commit message before a settings section is written. Empty falls back to `defaultMessage`."
        code={`<CommitDialog
  open={open}
  section="Runtime defaults"
  defaultMessage="chore: update runtime defaults"
  onConfirm={(message) => save(message)}
  onCancel={() => setOpen(false)}
/>`}
      >
        <div className="flex flex-wrap items-center gap-3">
          <Button variant="outline" onClick={() => setCommitOpen(true)}>Save with message…</Button>
          <span className="font-mono text-xs text-muted-foreground">last: {last}</span>
        </div>
        <CommitDialog
          open={commitOpen}
          section="Runtime defaults"
          defaultMessage="chore: update runtime defaults"
          onConfirm={(message) => { setLast(`commit: ${message}`); setCommitOpen(false); }}
          onCancel={() => setCommitOpen(false)}
        />
      </Entry>

      <Entry
        name="HelpTooltip"
        file="help-tooltip.tsx"
        description="A ? that explains on hover or focus. Portalled to the body so it escapes overflow: hidden."
        code={`<span className="inline-flex items-center gap-1.5">Max depth <HelpTooltip text="Deeper objects and arrays are summarized." /></span>`}
      >
        <div className="flex flex-wrap items-center gap-8 text-sm">
          <span className="inline-flex items-center gap-1.5">Max depth <HelpTooltip text="Deeper objects and arrays are summarized." /></span>
          <span className="inline-flex items-center gap-1.5">Capture level <HelpTooltip text="none records nothing; on-error records failing nodes; full records everything." /></span>
        </div>
      </Entry>

      <Entry
        name="Sonner"
        file="sonner.tsx"
        description="A stack of short notices. Fixed to the corner by default; `inline` renders in place. Variants: info | success | warning | error."
        code={`<Sonner toasts={[{ id: 1, msg: "Pipeline registered.", variant: "success" }]} />`}
      >
        <div className="grid gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <Button size="sm" variant="outline" onClick={() => setToasts((t) => [...t, { id: Date.now(), msg: `Toast #${t.length + 1}`, variant: ["info", "success", "warning", "error"][t.length % 4] }])}>
              add one
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setToasts([])}>clear</Button>
          </div>
          <div className="max-w-sm">
            <Sonner inline toasts={toasts} />
          </div>
        </div>
      </Entry>
    </div>
  );
}
