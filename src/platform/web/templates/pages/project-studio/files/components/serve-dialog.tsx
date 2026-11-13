import { useEffect, useState } from "zeb/react";
import Button from "@/components/ui/button";
import Textarea from "@/components/ui/textarea";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";

/**
 * Makes a folder run as a site (`public_execute`) on addresses written out in
 * full — one per line, each a host this project already has in Addressing. An
 * address with a path (`https://example.com/docs/`) gives the folder that path
 * and leaves the host's other paths to its pages; one without gives it the
 * whole host. Whoever can write into the folder decides what runs there; the
 * dialog says so, and says more when the folder would share a pages host.
 */
export function ServeDialog({ target, browser, onClose }) {
  const [origins, setOrigins] = useState("");
  useEffect(() => {
    const rule = target?.rule;
    setOrigins(Array.isArray(rule?.serve) ? rule.serve.join("\n") : "");
  }, [target?.path]);
  const serve = origins.split("\n").map((line) => line.trim()).filter(Boolean);
  const mountsAtPath = serve.some((line) => /^https?:\/\/[^/]+\/[^/\s]+/.test(line));
  const save = async () => {
    await browser.setAccess(target, "prefix", "public_execute", serve);
    onClose();
  };
  return (
    <Dialog open={!!target} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Serve “{target?.name}” as a site</DialogTitle>
          <p className="text-xs text-muted-foreground mt-0.5">
            Scripts in this folder will run on these addresses. Anyone who can write into the folder decides what runs there.
          </p>
        </DialogHeader>
        <div className="px-6 py-4 flex flex-col gap-2">
          <Textarea
            rows={4}
            value={origins}
            placeholder={"https://www.example.com\nhttps://example.com"}
            onInput={(event) => setOrigins(event.currentTarget.value)}
          />
          <p className="text-[0.72rem] text-muted-foreground">
            Full addresses, no wildcards; each host must already be added in Settings → Addressing. Add a path
            (https://example.com/docs/) to serve the folder there and keep the host's other pages.
          </p>
          {mountsAtPath ? (
            <p className="text-[0.72rem] text-destructive">
              A path on a host that runs your pages shares their sign-in: a script in this folder can act as any
              signed-in visitor of that host. Only let trusted pipelines and people write here.
            </p>
          ) : null}
        </div>
        <DialogFooter>
          <Button variant="ghost" size="sm" onClick={onClose}>Cancel</Button>
          <Button variant="destructive" size="sm" onClick={save} disabled={serve.length === 0}>
            Serve as site
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
