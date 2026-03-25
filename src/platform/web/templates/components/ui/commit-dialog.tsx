import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";

export default function CommitDialog({ open, section, defaultMessage, onConfirm, onCancel }: any) {
  const [message, setMessage] = useState(defaultMessage ?? "");

  useEffect(() => {
    if (open) setMessage(defaultMessage ?? "");
  }, [open, defaultMessage]);

  function handleSubmit(e: any) {
    e.preventDefault();
    onConfirm((message ?? "").trim() || (defaultMessage ?? "chore: update settings"));
  }

  return (
    <Dialog open={open} onOpenChange={(v: boolean) => !v && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            Commit changes{section ? <span className="project-inline-chip ml-2">{section}</span> : null}
          </DialogTitle>
        </DialogHeader>
        <p className="text-xs text-body-soft">
          Your changes will be saved and committed to the project repository.
        </p>
        <form onSubmit={handleSubmit} className="flex flex-col gap-3">
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-body-soft">Commit message</label>
            <Input
              name="commit_message"
              value={message}
              onInput={(e: any) => setMessage(e.currentTarget.value)}
              autoComplete="off"
              className="font-mono"
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" size="sm" label="Cancel" onClick={onCancel} />
            <Button type="submit" variant="primary" size="sm" label="Commit & Save" />
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
