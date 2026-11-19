import { useState, useEffect, cx } from "zeb/react";
import { Select, SelectOption } from "@/components/ui/select";
import { requestJson } from "@/components/lib/http";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import HmacFields from "@/pages/project-studio/credentials/components/hmac/hmac-fields";
import ControlFields from "@/pages/project-studio/credentials/components/zebflow-control/control-fields";
import SecretFields from "@/pages/project-studio/credentials/components/secret-fields";
import { FALLBACK_KINDS, defaultSecretForKind } from "@/pages/project-studio/credentials/components/credential-lib";

export default function CredentialDialog({ open, onClose, mode, editItem, apiList, apiItemBase, onSaved, credentialTypes }) {
  const [credentialId, setCredentialId] = useState("");
  const [title, setTitle] = useState("");
  const [kind, setKind] = useState("postgres");
  const [notes, setNotes] = useState("");
  const [secret, setSecret] = useState<Record<string, any>>({});
  const [status, setStatus] = useState("");
  const [statusTone, setStatusTone] = useState("info");
  const [busy, setBusy] = useState(false);

  // Load existing credential when editing
  useEffect(() => {
    if (!open) return;
    if (mode === "create") {
      setCredentialId(""); setTitle(""); setKind("postgres");
      setNotes(""); setSecret(defaultSecretForKind("postgres", credentialTypes)); setStatus("Fill fields and save."); setStatusTone("info");
      return;
    }
    if (!editItem) return;
    setCredentialId(editItem.credential_id || "");
    setTitle(editItem.title || "");
    setKind(editItem.kind || "custom");
    setNotes(editItem.notes || "");
    setSecret({});
    setStatus("Loading…"); setStatusTone("info");
    setBusy(true);
    requestJson(`${apiItemBase}/${encodeURIComponent(editItem.credential_id)}`).then((payload) => {
      const item = payload?.credential || payload?.item || {};
      setSecret(item.secret && typeof item.secret === "object" ? item.secret : {});
      setStatus("Loaded. Update fields and save."); setStatusTone("ok");
    }).catch((err) => {
      setStatus(`Failed to load: ${err?.message || err}`); setStatusTone("error");
    }).finally(() => setBusy(false));
  }, [open, mode, editItem?.credential_id]);

  function setSecretField(key: string, val: any) {
    if (key === "__json__") { setSecret(val); return; }
    if (key === "__json_raw__") return; // invalid JSON, ignore
    setSecret((prev) => ({ ...prev, [key]: val }));
  }

  async function handleSave(e) {
    e.preventDefault();
    if (busy) return;
    const id = mode === "edit" ? credentialId : credentialId.trim().toLowerCase().replace(/[^a-z0-9._-]+/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, "");
    if (!id) { setStatus("Credential ID is required."); setStatusTone("error"); return; }
    if (!title.trim()) { setStatus("Title is required."); setStatusTone("error"); return; }

    setBusy(true); setStatus("Saving…"); setStatusTone("info");
    try {
      const payload = { credential_id: id, title: title.trim(), kind, notes: notes.trim(), secret };
      if (mode === "edit") {
        await requestJson(`${apiItemBase}/${encodeURIComponent(id)}`, { method: "PUT", body: JSON.stringify(payload) });
      } else {
        await requestJson(apiList, { method: "POST", body: JSON.stringify(payload) });
      }
      onSaved();
      onClose();
    } catch (err: any) {
      setStatus(`Save failed: ${err?.message || err}`); setStatusTone("error");
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    if (mode !== "edit" || !credentialId || busy) return;
    if (!confirm(`Delete credential "${credentialId}"?`)) return;
    setBusy(true); setStatus("Deleting…"); setStatusTone("info");
    try {
      await requestJson(`${apiItemBase}/${encodeURIComponent(credentialId)}`, { method: "DELETE" });
      onSaved(); onClose();
    } catch (err: any) {
      setStatus(`Delete failed: ${err?.message || err}`); setStatusTone("error");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent
        className="max-w-2xl"
        style={{ overflow: "hidden", maxHeight: "calc(100dvh - 2rem)" }}
      >
        <DialogHeader className="shrink-0">
          <DialogTitle>{mode === "edit" ? `Edit — ${credentialId}` : "Create Credential"}</DialogTitle>
          <p className={cx("text-xs mt-0.5", statusTone === "error" ? "text-danger" : statusTone === "ok" ? "text-success" : "text-muted-foreground")}>{status}</p>
        </DialogHeader>

        <form onSubmit={handleSave} className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto overscroll-contain px-6 py-4">
            {/* Identity */}
            <div className="grid grid-cols-2 gap-3">
              <Field label="Credential ID">
                <Input
                  value={credentialId}
                  onChange={(e) => setCredentialId(e.target.value)}
                  placeholder="pg-main"
                  disabled={mode === "edit" || busy}
                  required
                />
              </Field>
              <Field label="Kind">
                <Select value={kind} onChange={(e) => { const nextKind = e.target.value; setKind(nextKind); setSecret(defaultSecretForKind(nextKind, credentialTypes)); }} disabled={busy}>
                  {credentialTypes.length > 0
                    ? credentialTypes.map((t) => <SelectOption key={t.kind} value={t.kind} label={t.title} />)
                    : FALLBACK_KINDS.map((k) => <SelectOption key={k} value={k} label={k} />)
                  }
                </Select>
              </Field>
              <Field label="Title" className="col-span-2">
                <Input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Main Postgres" required disabled={busy} />
              </Field>
            </div>

            {/* Dynamic secret fields */}
            {kind === "hmac" ? (
              <HmacFields secret={secret} onChange={setSecret} apiItemBase={apiItemBase} credentialId={mode === "edit" ? credentialId : ""} />
            ) : kind === "zebflow_control" ? (
              <ControlFields secret={secret} onChange={setSecret} apiList={apiList} credentialTypes={credentialTypes} />
            ) : (
              <SecretFields kind={kind} secret={secret} onChange={setSecretField} credentialTypes={credentialTypes} />
            )}

            {/* Notes */}
            <Field label="Notes">
              <textarea
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                rows={2}
                placeholder="Optional operational notes (no secrets here)"
                disabled={busy}
                className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40 disabled:opacity-50"
              />
            </Field>
          </div>

          <DialogFooter className="shrink-0">
            {mode === "edit" && (
              <Button type="button" variant="destructive" size="sm" onClick={handleDelete} disabled={busy}>Delete</Button>
            )}
            {mode === "edit" && kind === "oauth2" && (
              <Button type="button" variant="outline" size="sm" disabled={busy} onClick={async () => {
                setBusy(true); setStatus("Redirecting to provider…"); setStatusTone("info");
                try {
                  const data = await requestJson(`${apiItemBase}/${encodeURIComponent(credentialId)}/oauth/authorize`);
                  if (data?.redirect_url) { window.location.href = data.redirect_url; }
                  else { setStatus("No redirect URL returned."); setStatusTone("error"); setBusy(false); }
                } catch (err: any) {
                  setStatus(`Authorize failed: ${err?.message || err}`); setStatusTone("error"); setBusy(false);
                }
              }}>Authorize</Button>
            )}
            <Button type="button" variant="ghost" size="sm" onClick={onClose} disabled={busy}>Cancel</Button>
            <Button type="submit" size="sm" disabled={busy}>{busy ? "Saving…" : "Save"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
