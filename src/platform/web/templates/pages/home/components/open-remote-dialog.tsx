import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";

const STORAGE_PREFIX = "zebflow.office-address.";

/** The address this browser used for an office last time, if any. */
function rememberedAddress(officeId) {
  try {
    return window.localStorage.getItem(STORAGE_PREFIX + officeId) || "";
  } catch (_) {
    return "";
  }
}

function remember(officeId, address) {
  try {
    if (address) window.localStorage.setItem(STORAGE_PREFIX + officeId, address);
    else window.localStorage.removeItem(STORAGE_PREFIX + officeId);
  } catch (_) {}
}

/**
 * Opening a project that lives on another office (`offices.md` §3a).
 *
 * The controller mints a vouch for that office; the browser carries it to the
 * office and lands in the project. Where the browser goes is the operator's
 * choice: the office's own address, or one they give — a local tunnel to an
 * office inside a private network. That choice is kept in this browser only.
 */
export default function OpenRemoteDialog({ target, onClose }) {
  const office = target?.office;
  const project = target?.project;
  const [mode, setMode] = useState("office");
  const [ownAddress, setOwnAddress] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState("");

  useEffect(() => {
    if (!office) return;
    const saved = rememberedAddress(office.id);
    setOwnAddress(saved);
    setMode(saved ? "own" : "office");
    setProblem("");
  }, [office?.id, project?.project]);

  async function open() {
    const base = (mode === "own" ? ownAddress : office?.address || "").trim().replace(/\/+$/, "");
    if (!base) {
      setProblem("Give the address this browser can reach the office at.");
      return;
    }
    setBusy(true);
    setProblem("");
    try {
      const res = await requestJson(`/api/platform/cluster/offices/${encodeURIComponent(office.id)}/vouch`, {
        method: "POST",
      });
      const vouch = res?.vouch?.vouch;
      if (!vouch) throw new Error("The controller did not return a vouch.");
      remember(office.id, mode === "own" ? base : "");
      window.location.href = `${base}/office/vouch?v=${encodeURIComponent(vouch)}&next=${encodeURIComponent(project.path)}`;
    } catch (err) {
      setProblem(String(err?.message || err));
      setBusy(false);
    }
  }

  const radio = "mt-1 h-4 w-4 accent-[var(--primary)]";

  return (
    <Dialog open={Boolean(target)} onOpenChange={(open) => (open ? null : onClose())}>
      <DialogContent>
        <div className="space-y-4 px-6 pt-6 pb-2">
          <DialogHeader>
            <DialogTitle>Open {project?.title || project?.project}</DialogTitle>
            <DialogDescription>
              This project lives on office {office?.label || office?.id}. Choose where this browser reaches it.
            </DialogDescription>
          </DialogHeader>
          <label className="flex cursor-pointer items-start gap-3">
            <input type="radio" name="office-address" className={radio} checked={mode === "office"} onChange={() => setMode("office")} />
            <span className="min-w-0">
              <span className="block text-sm font-medium text-foreground">The office's address</span>
              <span className="block truncate font-mono text-[11.5px] text-muted-foreground">
                {office?.address || "no address advertised"}
              </span>
            </span>
          </label>
          <label className="flex cursor-pointer items-start gap-3">
            <input type="radio" name="office-address" className={radio} checked={mode === "own"} onChange={() => setMode("own")} />
            <span className="min-w-0 flex-1">
              <span className="block text-sm font-medium text-foreground">My own address</span>
              <span className="block text-[12px] text-muted-foreground">
                A tunnel or port-forward on this machine. Remembered in this browser.
              </span>
            </span>
          </label>
          {mode === "own" ? (
            <Field label="Address" id="office-own-address">
              <Input
                type="text"
                id="office-own-address"
                placeholder="http://localhost:20617"
                value={ownAddress}
                onInput={(e) => setOwnAddress(e.target.value)}
              />
            </Field>
          ) : null}
          {problem ? <p className="text-[13px] text-destructive">{problem}</p> : null}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button type="button" variant="primary" disabled={busy} onClick={open}>
            {busy ? "Opening…" : "Open"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
