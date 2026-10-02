import { cx } from "zeb/react";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import { formatBytes } from "@/components/lib/format";
import { FsTrashIcon, FsUploadIcon } from "@/pages/project-studio/files/components/file-icons";
import { FilesMessage } from "@/pages/project-studio/files/components/files-message";

/** Staging and sending uploads. `queue` owns the files; `browser` owns the status line. */
export function UploadDialog({ queue, browser }) {
  const uploading = browser.busy === "upload";
  const pasting = browser.busy === "paste";
  const items = queue.items;
  const hasItems = items.length > 0;
  const close = () => {
    queue.setOpen(false);
    queue.setDragActive(false);
  };
  const hold = (active: boolean) => (event: any) => {
    event.preventDefault();
    event.stopPropagation();
    queue.setDragActive(active);
  };
  return (
    <Dialog open={queue.open} onOpenChange={(v) => { if (!v) close(); }}>
      <DialogContent className="max-w-xl overflow-hidden max-h-[calc(100dvh-2rem)]" onPaste={queue.handlePaste}>
        <DialogHeader className="shrink-0">
          <DialogTitle>Upload Files</DialogTitle>
          <p className="text-xs text-muted-foreground mt-0.5">
            Destination: <span className="font-mono text-foreground">{queue.targetPath}/</span>
          </p>
        </DialogHeader>

        <div className="px-6 py-5">
          <div
            className={cx(
              "flex min-h-[15rem] flex-col items-center justify-center rounded-md border border-dashed px-6 py-8 text-center transition-colors",
              queue.dragActive ? "border-primary bg-primary/10 text-foreground" : "border-border bg-muted text-muted-foreground",
            )}
            onDragEnter={hold(true)}
            onDragOver={hold(true)}
            onDragLeave={hold(false)}
            onDrop={queue.handleDrop}
          >
            <div className="mb-3 flex h-11 w-11 items-center justify-center rounded-md border border-border bg-card text-foreground">
              <FsUploadIcon />
            </div>
            <p className="text-[0.9rem] font-semibold text-foreground">
              {uploading ? "Uploading..." : queue.dragActive ? "Drop to upload" : "Drop files here"}
            </p>
            <p className="mt-1 max-w-sm text-[0.76rem] text-muted-foreground">
              Choose files, drag them into this panel, or paste an image from the clipboard. Files are not uploaded until you confirm.
            </p>
            <div className="mt-4 flex flex-wrap items-center justify-center gap-2">
              <Button as="label" size="sm" className="cursor-pointer" disabled={uploading || pasting}>
                Choose Files
                <input
                  type="file"
                  multiple
                  className="sr-only"
                  onChange={(event) => queue.stage((event.target as HTMLInputElement).files, "file")}
                />
              </Button>
              <Button variant="outline" size="sm" onClick={queue.pasteFromClipboard} disabled={uploading || pasting}>
                {pasting ? "Reading..." : "Paste Image"}
              </Button>
            </div>
          </div>

          {hasItems ? (
            <div className="mt-4 rounded-md border border-border bg-card">
              <div className="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
                <p className="text-[0.78rem] font-semibold text-foreground">Pending upload</p>
                <Badge variant="outline" className="text-[0.68rem]">
                  {items.length} file{items.length === 1 ? "" : "s"}
                </Badge>
              </div>
              <div className="max-h-[13rem] overflow-y-auto">
                {items.map((item) => (
                  <div key={item.id} className="grid grid-cols-[1fr_auto_auto] items-center gap-2 border-b border-border px-3 py-2 last:border-b-0">
                    <Input
                      value={item.name}
                      onInput={(event) => queue.rename(item.id, event.currentTarget.value)}
                      disabled={uploading}
                      className="min-w-0"
                    />
                    <span className="whitespace-nowrap text-[0.7rem] text-muted-foreground">{formatBytes(item.size)}</span>
                    <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => queue.remove(item.id)} disabled={uploading} aria-label={`Remove ${item.name}`} title="Remove">
                      <FsTrashIcon />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          <FilesMessage text={browser.message} tone={browser.messageTone} className="mt-3" />
        </div>

        <DialogFooter className="shrink-0">
          {hasItems ? (
            <Button variant="ghost" size="sm" onClick={queue.clear} disabled={uploading || pasting}>Clear</Button>
          ) : null}
          <Button variant="ghost" size="sm" onClick={close} disabled={uploading || pasting}>Close</Button>
          <Button size="sm" onClick={queue.upload} disabled={!hasItems || uploading || pasting}>
            {uploading ? "Uploading..." : "Upload"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
