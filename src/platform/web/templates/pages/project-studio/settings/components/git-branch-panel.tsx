import { cx, useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** The working branch, and switching it. */
export default function GitBranchPanel({ owner, project }) {
  const branchesUrl = `/api/projects/${owner}/${project}/git/branches`;
  const [current, setCurrent] = useState("");
  const [branches, setBranches] = useState([]);
  const [selected, setSelected] = useState("");
  const [newBranch, setNewBranch] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState("");
  const [msgTone, setMsgTone] = useState("info");

  async function fetchBranches() {
    try {
      const data = await requestJson(branchesUrl);
      if (!data) return;
      setCurrent(data.current ?? "");
      setBranches(Array.isArray(data.branches) ? data.branches : []);
      setSelected(data.current ?? "");
    } catch (_) {}
  }

  useEffect(() => { fetchBranches(); }, []);

  async function handleSwitch() {
    if (!selected || selected === current) return;
    setBusy(true); setMsg("");
    try {
      await requestJson(branchesUrl, { method: "POST", body: JSON.stringify({ branch: selected, create: false }) });
      setMsg(`Switched to ${selected}.`); setMsgTone("ok");
      await fetchBranches();
    } catch (e) { setMsg((e as any)?.message || "Failed"); setMsgTone("error"); }
    setBusy(false);
  }

  async function handleCreate() {
    const name = newBranch.trim();
    if (!name) return;
    setBusy(true); setMsg("");
    try {
      await requestJson(branchesUrl, { method: "POST", body: JSON.stringify({ branch: name, create: true }) });
      setMsg(`Branch "${name}" created and checked out.`); setMsgTone("ok");
      setNewBranch("");
      await fetchBranches();
    } catch (e) { setMsg((e as any)?.message || "Failed"); setMsgTone("error"); }
    setBusy(false);
  }

  return (
    <SettingsSection
      id="git"
      title="Git Branches"
      description="Switch or create local branches for this project."
      tag={current || null}
    >

      {branches.length > 1 && (
        <div className="flex items-end gap-[0.65rem] mb-[0.65rem]">
          <Field label="Switch to branch" className="flex-1">
            <Select value={selected} onChange={(e) => setSelected(e.target.value)}>
              {branches.map((b) => <SelectOption key={b} value={b}>{b}</SelectOption>)}
            </Select>
          </Field>
          <Button size="sm" variant="outline" onClick={handleSwitch} disabled={busy || selected === current}>
            Switch
          </Button>
        </div>
      )}

      <div className="flex items-end gap-[0.65rem]">
        <Field label="Create new branch" className="flex-1">
          <Input
            placeholder="branch-name"
            value={newBranch}
            onInput={(e) => setNewBranch(e.currentTarget.value)}
          />
        </Field>
        <Button size="sm" variant="primary" onClick={handleCreate} disabled={busy || !newBranch.trim()}>
          Create
        </Button>
      </div>

      {msg && (
        <p className={cx("text-[0.72rem] mt-[0.5rem]", msgTone === "ok" ? "text-accent" : msgTone === "error" ? "text-red-400" : "text-body-soft")}>
          {msg}
        </p>
      )}
    </SettingsSection>
  );
}
