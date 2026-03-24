import { useState, useEffect, useRef, useNavigate, cx } from "zeb";
import { useWindowEvent } from "zeb/use";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Dialog from "@/components/ui/dialog";
import { GitBranchIcon } from "@/pages/project-studio/components/icons";
import { GitFileTree } from "@/pages/project-studio/components/git-file-tree";
import { useStudioChrome } from "@/pages/project-studio/components/studio-chrome-context";

const credLabelCx = "text-[0.65rem] font-semibold uppercase tracking-[0.07em] text-[var(--studio-text-soft)] mb-[0.3rem] block";
const credInputCx = "w-full bg-[var(--studio-panel-2)] border border-[var(--studio-border)] rounded-[0.35rem] text-[var(--studio-text)] text-[0.68rem] font-mono px-[0.4rem] h-7 outline-none focus:border-green-500";
const credSelectCx = "w-full bg-[var(--studio-panel-2)] border border-[var(--studio-border)] rounded-[0.35rem] text-[var(--studio-text)] text-[0.72rem] px-[0.4rem] h-7 outline-none cursor-pointer";

export function GitRepoPanel({ owner, project }) {
  const nav = useNavigate();
  const { repoEpoch, activePanel, openHeaderPanel } = useStudioChrome();
  // ── Git state ────────────────────────────────────────────────────────────
  const [open, setOpen] = useState(false);
  const [files, setFiles] = useState([]);
  const [gitLoading, setGitLoading] = useState(false);
  const [synced, setSynced] = useState(true);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [commitError, setCommitError] = useState("");

  // ── Repo state ───────────────────────────────────────────────────────────
  const [creds, setCreds] = useState([]);
  const [selectedId, setSelectedId] = useState("");
  const [slug, setSlug] = useState("");
  const [branch, setBranch] = useState("main");
  const [saveState, setSaveState] = useState("idle");
  const [saveMsg, setSaveMsg] = useState("");
  const [repoLoading, setRepoLoading] = useState(false);

  // ── Inline credential dialog state ─────────────────────────────────────
  const [showCredDialog, setShowCredDialog] = useState(false);
  const [newCredId, setNewCredId] = useState("");
  const [newCredKind, setNewCredKind] = useState("github");
  const [newCredTitle, setNewCredTitle] = useState("");
  const [newCredUsername, setNewCredUsername] = useState("");
  const [newCredToken, setNewCredToken] = useState("");
  const [newCredGitName, setNewCredGitName] = useState("");
  const [newCredGitEmail, setNewCredGitEmail] = useState("");
  const [newCredGitlabUrl, setNewCredGitlabUrl] = useState("https://gitlab.com");
  const [credSaving, setCredSaving] = useState(false);
  const [credSaveError, setCredSaveError] = useState("");

  const storageKey = `zf-repo-${owner}-${project}`;
  const statusUrl = `/api/projects/${owner}/${project}/git/status`;
  const commitUrl = `/api/projects/${owner}/${project}/git/commit`;
  const credApiUrl = `/api/projects/${owner}/${project}/credentials`;

  // ── Git helpers ──────────────────────────────────────────────────────────
  async function fetchStatus() {
    if (!owner || !project) return;
    setGitLoading(true);
    try {
      const res = await fetch(statusUrl, { headers: { Accept: "application/json" } });
      if (res.status === 401) { nav("/login"); return; }
      const data = await res.json().catch(() => []);
      setFiles(Array.isArray(data) ? data.map((f) => ({ ...f, checked: true })) : []);
    } catch (_) {}
    setGitLoading(false);
  }

  const repoEpochBoot = useRef(false);
  useEffect(() => {
    if (!owner || !project) return;
    fetchStatus();
    if (repoEpochBoot.current) setSynced(false);
    repoEpochBoot.current = true;
  }, [owner, project, repoEpoch]);

  useEffect(() => {
    if (activePanel !== "git-repo" && open) setOpen(false);
  }, [activePanel]);

  useWindowEvent("rwe:nav", () => setOpen(false));

  async function doCommit(push) {
    const checked = files.filter((f) => f.checked).map((f) => f.rel_path);
    if (!checked.length || !message.trim()) return;
    setBusy(true);
    setCommitError("");

    let pushExtra: Record<string, string> = {};
    if (push) {
      try {
        const cfg = JSON.parse(localStorage.getItem(storageKey) || "null");
        if (cfg) {
          if (cfg.credential_id) pushExtra.credential_id = cfg.credential_id;
          if (cfg.repo_url)      pushExtra.repo_url = cfg.repo_url;
          if (cfg.branch)        pushExtra.branch = cfg.branch;
        }
      } catch (_) {}
    }

    try {
      const res = await fetch(commitUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ files: checked, message: message.trim(), push, ...pushExtra }),
      });
      if (res.status === 401) { nav("/login"); return; }
      const body = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(body?.error?.message || body?.message || "Failed");
      setMessage("");
      if (push) setSynced(true);
      await fetchStatus();
    } catch (e) {
      setCommitError(e.message || "Error");
    }
    setBusy(false);
  }

  // ── Repo helpers ─────────────────────────────────────────────────────────
  function getCredHost(cred) {
    if (!cred) return "";
    if (cred.kind === "github") return "github.com";
    if (cred.kind === "gitlab") return cred.secret?.url || "gitlab.com";
    return "";
  }

  function loadSaved() {
    try {
      const saved = JSON.parse(localStorage.getItem(storageKey) || "null");
      if (saved) {
        setSelectedId(saved.credential_id ?? "");
        if (saved.slug) {
          setSlug(saved.slug);
        } else if (saved.repo_url) {
          const m = String(saved.repo_url).match(/^https?:\/\/[^/]+\/(.+?)(?:\.git)?$/);
          setSlug(m ? m[1] : saved.repo_url);
        }
        setBranch(saved.branch ?? "main");
      }
    } catch (_) {}
  }

  async function fetchCreds() {
    if (!owner || !project) return;
    setRepoLoading(true);
    try {
      const res = await fetch(credApiUrl, { headers: { Accept: "application/json" } });
      const data = await res.json().catch(() => ({}));
      const items = (Array.isArray(data?.items) ? data.items : []).filter(
        (c) => c.kind === "github" || c.kind === "gitlab"
      );
      setCreds(items);
    } catch (_) {}
    setRepoLoading(false);
  }

  function handleConnect() {
    if (!selectedId || !slug.trim()) {
      setSaveMsg("Select a credential and enter the repo path.");
      setSaveState("error");
      return;
    }
    const host = getCredHost(creds.find((c) => c.credential_id === selectedId) ?? null);
    const repoUrl = host ? `https://${host}/${slug.trim()}.git` : slug.trim();
    try {
      localStorage.setItem(storageKey, JSON.stringify({
        credential_id: selectedId,
        slug: slug.trim(),
        repo_url: repoUrl,
        branch: branch.trim() || "main",
      }));
      setSaveState("saved");
      setSaveMsg("Connected.");
      setTimeout(() => { setSaveState("idle"); setSaveMsg(""); }, 2500);
    } catch (_) {
      setSaveState("error");
      setSaveMsg("Failed to save.");
    }
  }

  function handleDisconnect() {
    try { localStorage.removeItem(storageKey); } catch (_) {}
    setSelectedId("");
    setSlug("");
    setBranch("main");
    setSaveState("idle");
    setSaveMsg("");
  }

  async function handleCreateCred() {
    if (!newCredId.trim() || !newCredTitle.trim() || !newCredUsername.trim() || !newCredToken.trim()) {
      setCredSaveError("ID, Title, Username and Token are required.");
      return;
    }
    setCredSaving(true);
    setCredSaveError("");
    const createdId = newCredId.trim();
    const secret = { username: newCredUsername, token: newCredToken, git_name: newCredGitName, git_email: newCredGitEmail };
    if (newCredKind === "gitlab") (secret as any).url = newCredGitlabUrl || "https://gitlab.com";
    const payload = { credential_id: createdId, title: newCredTitle.trim(), kind: newCredKind, notes: "", secret };
    try {
      const res = await fetch(credApiUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(payload),
      });
      if (res.status === 401) { nav("/login"); return; }
      const body = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(body?.error?.message || body?.message || "Failed");
      setShowCredDialog(false);
      setNewCredId(""); setNewCredTitle(""); setNewCredKind("github");
      setNewCredUsername(""); setNewCredToken(""); setNewCredGitName(""); setNewCredGitEmail("");
      setNewCredGitlabUrl("https://gitlab.com");
      await fetchCreds();
      setSelectedId(createdId);
    } catch (e) {
      setCredSaveError((e as any)?.message || "Error");
    }
    setCredSaving(false);
  }

  function toggle() {
    if (!open) {
      fetchStatus();
      fetchCreds();
      loadSaved();
      openHeaderPanel("git-repo");
    }
    setOpen((o) => !o);
  }

  // ── Derived ──────────────────────────────────────────────────────────────
  const staged = files.filter((f) => { const x = (f.code ?? " ")[0]; return x !== " " && x !== "?"; });
  const unstaged = files.filter((f) => { const x = (f.code ?? " ")[0]; const y = (f.code ?? " ")[1]; return x === "?" || (y && y !== " "); });
  const count = files.length;
  const checkedCount = files.filter((f) => f.checked).length;
  const connected = (() => { try { return !!JSON.parse(localStorage.getItem(storageKey) || "null"); } catch (_) { return false; } })();
  const selectedCred = creds.find((c) => c.credential_id === selectedId) ?? null;
  const credHost = getCredHost(selectedCred);

  return (
    <div
      className="relative"
      tw-variants="absolute -top-1 -right-1 min-w-4 h-4 px-[0.22rem] bg-orange-500 text-[0.58rem] leading-4 pointer-events-none fixed inset-0 z-40 top-[calc(100%+6px)] w-[640px] shadow-[0_8px_24px_rgba(0,0,0,0.25)] z-50 py-[0.55rem] gap-[0.35rem] bg-slate-500 max-h-[440px] py-[0.5rem] pb-[0.3rem] border-[var(--studio-border-soft)] tracking-[0.07em] py-[0.4rem] min-h-[2.5rem] pt-[0.3rem] pb-[0.15rem] py-[0.6rem] text-[0.74rem] px-[0.6rem] gap-[0.4rem] px-[0.1rem] text-red-400 text-green-400 w-[260px] gap-[0.85rem] leading-[1.5] text-[0.72rem] underline text-[var(--studio-accent)] rounded-[0.35rem] focus-within:border-green-500 opacity-50 whitespace-nowrap bg-[var(--studio-panel-3)] border-r h-7 bg-transparent border-none text-[0.68rem] text-green-500 border-green-500 text-[0.75rem] text-[0.7rem] text-[0.65rem] text-[0.6rem] text-blue-400 w-3.5 h-3.5 gap-1.5 w-px"
    >
      {/* Trigger button */}
      <Button
        variant="outline"
        size="icon"
        onClick={toggle}
        title={count > 0 ? `${count} change${count !== 1 ? "s" : ""}` : "Git"}
        className="relative"
      >
        <GitBranchIcon />
        {count > 0 && (
          <span className="absolute -top-1 -right-1 min-w-4 h-4 px-[0.22rem] rounded-full bg-orange-500 text-white text-[0.58rem] font-bold leading-4 text-center pointer-events-none">
            {count}
          </span>
        )}
        {count === 0 && (
          <span
            className={cx(
              "absolute -bottom-[3px] -right-[3px] w-2 h-2 rounded-full pointer-events-none",
              (connected || synced) ? "bg-green-500" : "bg-slate-400",
            )}
            tw-variants="bg-green-500 bg-slate-400"
          />
        )}
      </Button>

      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />

          {/* Panel */}
          <div className="absolute top-[calc(100%+6px)] right-0 w-[640px] flex flex-col bg-[var(--studio-panel)] border border-[var(--studio-border)] rounded-lg shadow-[0_8px_24px_rgba(0,0,0,0.25)] overflow-hidden z-50">

            {/* Header */}
            <div className="flex items-center justify-between px-3 py-[0.55rem] border-b border-[var(--studio-border)] gap-2">
              <div className="text-[0.75rem] font-semibold text-[var(--studio-text)] flex items-center gap-[0.35rem]">
                <GitBranchIcon className="w-3.5 h-3.5 shrink-0" />
                <span>Git</span>
              </div>
              <div className="flex items-center gap-2 text-[0.7rem] text-[var(--studio-text-soft)]">
                <span>{gitLoading ? "…" : `${count} change${count !== 1 ? "s" : ""}`}</span>
                <span
                  className={cx(
                    "w-2 h-2 rounded-full",
                    synced && count === 0 ? "bg-green-500" : "bg-slate-500",
                  )}
                  tw-variants="bg-green-500 bg-slate-500"
                />
                {connected && <span className="text-blue-400 text-xs">● remote</span>}
              </div>
            </div>

            {/* Two-column body */}
            <div className="flex flex-1 min-h-0 max-h-[440px]">

              {/* Left: commit */}
              <div className="flex-1 min-w-0 flex flex-col overflow-hidden">
                <div className="text-[0.65rem] font-bold uppercase tracking-[0.07em] text-[var(--studio-text-soft)] px-3 py-[0.5rem] pb-[0.3rem] border-b border-[var(--studio-border-soft)]">
                  Commit
                </div>
                <div className="flex-1 overflow-y-auto py-[0.4rem] min-h-[2.5rem]">
                  {staged.length > 0 && (
                    <>
                      <p className="text-[0.6rem] font-bold tracking-[0.07em] text-[var(--studio-text-soft)] px-3 pt-[0.3rem] pb-[0.15rem] uppercase">STAGED</p>
                      <GitFileTree files={staged} setFiles={setFiles} />
                    </>
                  )}
                  {unstaged.length > 0 && (
                    <>
                      <p className="text-[0.6rem] font-bold tracking-[0.07em] text-[var(--studio-text-soft)] px-3 pt-[0.3rem] pb-[0.15rem] uppercase">CHANGES</p>
                      <GitFileTree files={unstaged} setFiles={setFiles} />
                    </>
                  )}
                  {count === 0 && !gitLoading && <p className="text-[0.74rem] text-[var(--studio-text-soft)] px-3 py-[0.6rem]">Working tree clean.</p>}
                  {gitLoading && <p className="text-[0.74rem] text-[var(--studio-text-soft)] px-3 py-[0.6rem]">Loading…</p>}
                </div>
                {count > 0 && (
                  <div className="border-t border-[var(--studio-border)] px-[0.6rem] py-[0.5rem] flex flex-col gap-[0.4rem]">
                    <Input
                      value={message}
                      onInput={(e) => setMessage(e.target.value)}
                      placeholder="Commit message…"
                      className="w-full text-[0.78rem]"
                    />
                    {commitError && <p className="text-[0.7rem] text-red-400 px-[0.1rem]">{commitError}</p>}
                    <div className="flex gap-[0.4rem] justify-end">
                      <Button size="xs" onClick={() => doCommit(false)} disabled={busy || !message.trim() || !checkedCount}>
                        Commit
                      </Button>
                      <Button
                        size="xs"
                        variant="outline"
                        className={cx(synced && "text-green-500 border-green-500")}
                        tw-variants="text-green-500 border-green-500"
                        onClick={() => doCommit(true)}
                        disabled={busy || !message.trim() || !checkedCount}
                      >
                        {synced ? "✓ Synced" : "↑ Sync"}
                      </Button>
                    </div>
                  </div>
                )}
              </div>

              {/* Divider */}
              <div className="w-px bg-[var(--studio-border)] shrink-0" />

              {/* Right: remote */}
              <div className="w-[260px] shrink-0 flex flex-col overflow-y-auto p-3 gap-[0.85rem]">
                <div
                  className="text-[0.65rem] font-bold uppercase tracking-[0.07em] text-[var(--studio-text-soft)] border-b border-[var(--studio-border-soft)] pb-[0.3rem]"
                  style={{ margin: "-.75rem -.75rem .6rem" }}
                >Remote</div>

                {repoLoading ? (
                  <p className="text-[0.72rem] text-[var(--studio-text-soft)] leading-[1.5]">Loading…</p>
                ) : creds.length === 0 ? (
                  <p className="text-[0.72rem] text-[var(--studio-text-soft)] leading-[1.5]">
                    No GitHub / GitLab credentials.{" "}
                    <a
                      href="#"
                      onClick={(e) => { e.preventDefault(); setShowCredDialog(true); }}
                      className="underline text-[var(--studio-accent)]"
                    >Create one</a>
                  </p>
                ) : (
                  <>
                    <div>
                      <label className={credLabelCx}>Credential</label>
                      <select
                        className={credSelectCx}
                        value={selectedId}
                        onChange={(e) => { setSelectedId(e.target.value); setSaveState("idle"); setSaveMsg(""); }}
                      >
                        <option value="">— select —</option>
                        {creds.map((c) => (
                          <option key={c.credential_id} value={c.credential_id}>{c.title} · {c.kind}</option>
                        ))}
                      </select>
                    </div>

                    <div>
                      <label className={credLabelCx}>Repository</label>
                      <div className={cx(
                        "flex items-center border border-[var(--studio-border)] rounded-[0.35rem] overflow-hidden bg-[var(--studio-panel-2)] focus-within:border-green-500",
                        !selectedId && "opacity-50",
                      )}>
                        {credHost && (
                          <span className="text-[0.65rem] font-mono text-[var(--studio-text-soft)] px-[0.4rem] whitespace-nowrap bg-[var(--studio-panel-3)] border-r border-[var(--studio-border)] h-7 flex items-center shrink-0">
                            {credHost}/
                          </span>
                        )}
                        <input
                          className="flex-1 min-w-0 bg-transparent border-none outline-none text-[var(--studio-text)] text-[0.68rem] font-mono px-[0.4rem] h-7"
                          type="text"
                          placeholder="username/repo-name"
                          value={slug}
                          disabled={!selectedId}
                          onInput={(e) => setSlug((e.target as HTMLInputElement).value)}
                        />
                      </div>
                    </div>

                    <div>
                      <label className={credLabelCx}>Branch</label>
                      <input
                        className={credInputCx}
                        type="text"
                        placeholder="main"
                        value={branch}
                        onInput={(e) => setBranch((e.target as HTMLInputElement).value)}
                      />
                    </div>

                    <div className="flex flex-col gap-1.5">
                      <Button
                        variant={saveState === "saved" ? "default" : "outline"}
                        size="xs"
                        onClick={handleConnect}
                        disabled={!selectedId || !slug.trim()}
                        className="w-full"
                      >
                        {saveState === "saved" ? "✓ Connected" : connected ? "Reconnect" : "Connect"}
                      </Button>
                      {connected && (
                        <Button variant="ghost" size="xs" onClick={handleDisconnect} className="w-full">
                          Disconnect
                        </Button>
                      )}
                      {saveMsg && (
                        <p className={cx("text-[0.68rem]", saveState === "error" ? "text-red-400" : "text-green-400")}>
                          {saveMsg}
                        </p>
                      )}
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>
        </>
      )}

      <Dialog
        open={showCredDialog}
        onClose={() => setShowCredDialog(false)}
        title="Add Git Credential"
        wClass="max-w-2xl"
        footer={
          <>
            <Button size="xs" onClick={handleCreateCred} disabled={credSaving} className="flex-1">
              {credSaving ? "Saving…" : "Save"}
            </Button>
            <Button size="xs" variant="ghost" onClick={() => setShowCredDialog(false)} disabled={credSaving} className="flex-1">
              Cancel
            </Button>
          </>
        }
      >
        <div>
          <label className={credLabelCx}>Kind</label>
          <select
            className={credSelectCx}
            value={newCredKind}
            onChange={(e) => setNewCredKind(e.target.value)}
          >
            <option value="github">GitHub</option>
            <option value="gitlab">GitLab</option>
          </select>
        </div>

        {newCredKind === "gitlab" && (
          <div>
            <label className={credLabelCx}>Instance URL</label>
            <input
              className={credInputCx}
              type="text"
              placeholder="https://gitlab.com"
              value={newCredGitlabUrl}
              onInput={(e) => setNewCredGitlabUrl((e.target as HTMLInputElement).value)}
            />
          </div>
        )}

        <div>
          <label className={credLabelCx}>Credential ID</label>
          <input className={credInputCx} type="text" placeholder="my-github" value={newCredId}
            onInput={(e) => setNewCredId((e.target as HTMLInputElement).value)} />
        </div>

        <div>
          <label className={credLabelCx}>Title</label>
          <input className={credInputCx} type="text" placeholder="My GitHub Account" value={newCredTitle}
            onInput={(e) => setNewCredTitle((e.target as HTMLInputElement).value)} />
        </div>

        <div>
          <label className={credLabelCx}>Username</label>
          <input className={credInputCx} type="text"
            placeholder={newCredKind === "github" ? "github-username" : "gitlab-username"}
            value={newCredUsername}
            onInput={(e) => setNewCredUsername((e.target as HTMLInputElement).value)} />
        </div>

        <div>
          <label className={credLabelCx}>Personal Access Token</label>
          <input className={credInputCx} type="password"
            placeholder={newCredKind === "github" ? "ghp_…" : "glpat-…"}
            value={newCredToken}
            onInput={(e) => setNewCredToken((e.target as HTMLInputElement).value)} />
        </div>

        <div>
          <label className={credLabelCx}>Git Name (optional)</label>
          <input className={credInputCx} type="text" placeholder="Your Name" value={newCredGitName}
            onInput={(e) => setNewCredGitName((e.target as HTMLInputElement).value)} />
        </div>

        <div>
          <label className={credLabelCx}>Git Email (optional)</label>
          <input className={credInputCx} type="text" placeholder="you@example.com" value={newCredGitEmail}
            onInput={(e) => setNewCredGitEmail((e.target as HTMLInputElement).value)} />
        </div>

        {credSaveError && <p className="text-[0.68rem] text-red-400">{credSaveError}</p>}
      </Dialog>
    </div>
  );
}
