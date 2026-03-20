import { useState, useEffect } from "zeb";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import Textarea from "@/components/ui/textarea";
import type { GitFile } from "@/components/pipeline-editor/types";

interface GitCommitDialogProps {
  open: boolean;
  files: GitFile[];
  gitCommitUrl: string;
  redirectUrl: string;
  onClose: () => void;
}

export default function GitCommitDialog({
  open,
  files,
  gitCommitUrl,
  redirectUrl,
  onClose,
}: GitCommitDialogProps) {
  const [checkedFiles, setCheckedFiles] = useState<Set<string>>(new Set());
  const [message, setMessage] = useState("");
  const [push, setPush] = useState(false);
  const [error, setError] = useState("");
  const [committing, setCommitting] = useState(false);

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setCheckedFiles(new Set(files.map((f) => f.rel_path)));
      setMessage("");
      setPush(false);
      setError("");
      setCommitting(false);
    }
  }, [open]);

  if (!open) return null;

  const canSubmit = checkedFiles.size > 0 && message.trim().length > 0 && !committing;

  function toggleFile(relPath: string, checked: boolean) {
    setCheckedFiles((prev) => {
      const next = new Set(prev);
      if (checked) next.add(relPath);
      else next.delete(relPath);
      return next;
    });
  }

  async function handleCommit() {
    if (!canSubmit) return;
    setCommitting(true);
    setError("");
    try {
      const res = await fetch(gitCommitUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          files: Array.from(checkedFiles),
          message: message.trim(),
          push,
        }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        setError(data?.error?.message || data?.message || "Commit failed");
        setCommitting(false);
        return;
      }
      closeAndRedirect();
    } catch (err: any) {
      setError(err?.message || "Network error");
      setCommitting(false);
    }
  }

  function closeAndRedirect() {
    if (typeof window !== "undefined") {
      window.dispatchEvent(new CustomEvent("zf:repo:changed"));
      window.location.href = redirectUrl;
    }
    onClose();
  }

  return (
    <div className="git-commit-overlay">
      <div className="git-commit-backdrop" onClick={closeAndRedirect} />
      <div className="git-commit-box">
        <div className="git-commit-header">
          <h3 className="git-commit-title">Commit Pipeline Changes</h3>
          <Button variant="ghost" size="icon" aria-label="Close" onClick={closeAndRedirect}>✕</Button>
        </div>

        {/* File list */}
        <div className="git-commit-file-list">
          {files.map((f) => (
            <label key={f.rel_path} className="git-commit-file-row">
              <Checkbox
                checked={checkedFiles.has(f.rel_path)}
                onChange={(e) => toggleFile(f.rel_path, e.currentTarget.checked)}
              />
              <code
                className={`git-status-code ${f.code === "??" ? "git-status-untracked" : `git-status-${String(f.code).trim()}`}`}
              >
                {f.code}
              </code>
              <span className="git-commit-file-path">{f.rel_path}</span>
            </label>
          ))}
        </div>

        {/* Commit message */}
        <Textarea
          placeholder="Commit message…"
          rows={3}
          value={message}
          onInput={(e) => setMessage(e.currentTarget.value)}
          className="git-commit-message"
        />

        {/* Push option */}
        <label className="git-commit-push-row">
          <Checkbox
            checked={push}
            onChange={(e) => setPush(e.currentTarget.checked)}
            label="Push after commit"
          />
        </label>

        {error && <p className="git-commit-error">{error}</p>}

        <div className="git-commit-actions">
          <Button size="xs" onClick={handleCommit} disabled={!canSubmit}>
            {committing ? "Committing…" : "Commit"}
          </Button>
          <Button variant="outline" size="xs" onClick={closeAndRedirect}>
            Skip
          </Button>
        </div>
      </div>
    </div>
  );
}
