import { useState, useEffect, useRef, cx, Link } from "rwe";
import PlatformSidebar from "@/components/platform-sidebar";
import {
  initProjectShellBehavior,
  subscribeConsole,
  subscribeOverlay,
  consoleLines,
  autoOverlayState,
  navigate,
} from "@/components/behavior/project-shell";
import ConsolePanel from "@/components/ui/console-panel";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Checkbox from "@/components/ui/checkbox";
import Toggle from "@/components/ui/toggle";
import DropdownMenuContent from "@/components/ui/dropdown-menu-content";

// ── Icons ────────────────────────────────────────────────────────────────────

function HomeIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5">
      <path d="M4 10.5L12 4l8 6.5V20H4z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M20 15.2A8 8 0 118.8 4 6.5 6.5 0 0020 15.2z" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function SunIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <circle cx="12" cy="12" r="4" stroke="currentColor" strokeWidth="1.8" />
      <path d="M12 2v2.5M12 19.5V22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M2 12h2.5M19.5 12H22M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

function TerminalIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" strokeWidth="1.8" />
      <path d="M7 9l3 3-3 3M13 15h4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function SessionIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M8 6h8M6 10h12M9 14h6M11 18h2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

function GitBranchIcon({ className = "w-4 h-4" }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" className={className}>
      <circle cx="6" cy="6" r="2" stroke="currentColor" strokeWidth="1.8" />
      <circle cx="6" cy="18" r="2" stroke="currentColor" strokeWidth="1.8" />
      <circle cx="18" cy="6" r="2" stroke="currentColor" strokeWidth="1.8" />
      <path d="M6 8v8M6 8c0 4 12 4 12-2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

function RepoIcon({ className = "w-4 h-4" }) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
      <path d="M9 18c-4.51 2-5-2-7-2" />
    </svg>
  );
}

// ── Panel mutex — close all other panels when one opens ───────────────────────
function dispatchPanelOpen(name) {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("zf:panel:opened", { detail: { panel: name } }));
  }
}

// ── ConsoleOutput ────────────────────────────────────────────────────────────

function ConsoleOutput() {
  const [lines, setLines] = useState(consoleLines);
  const bottomRef = useRef(null);

  useEffect(() => {
    subscribeConsole(() => setLines([...consoleLines]));
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "instant" });
  }, [lines]);

  return (
    <div className="cli-output-list" data-cli-mount>
      {lines.map((line) =>
        line.isLink ? (
          <div key={line.id} className={cx("cli-line", line.cls)}>
            <a
              href={line.isLink}
              className="cli-link"
              onClick={(e) => { e.preventDefault(); navigate(line.isLink); }}
            >
              {line.text}
            </a>
          </div>
        ) : (
          <div key={line.id} className={cx("cli-line", line.cls)}>{line.text}</div>
        )
      )}
      <div ref={bottomRef} />
    </div>
  );
}

// ── AutoOverlay ──────────────────────────────────────────────────────────────

function AutoOverlay() {
  const [s, setS] = useState(autoOverlayState);

  useEffect(() => {
    subscribeOverlay(() => setS({ ...autoOverlayState }));
  }, []);

  if (!s.active) return null;

  return (
    <div className="zf-auto-overlay">
      <div
        className={cx("zf-auto-cursor", s.clicking && "is-clicking")}
        style={{ transform: `translate(${s.cursorX}px, ${s.cursorY}px)` }}
      />
      <div className="zf-auto-label">{s.label}</div>
      <div className="zf-auto-loader" />
    </div>
  );
}

// ── Git Panel ────────────────────────────────────────────────────────────────

function gitStatusChar(code) {
  if (code === "??") return "U";
  const trimmed = String(code || "").replace(/\s/g, "");
  return trimmed[0] || "M";
}

function buildGitFileTree(files) {
  const byPath = new Map();
  const roots = [];

  files.forEach((f) => {
    const parts = String(f.rel_path || "").split("/").filter(Boolean);
    for (let i = 1; i < parts.length; i++) {
      const dirPath = parts.slice(0, i).join("/");
      if (!byPath.has(dirPath)) {
        byPath.set(dirPath, { id: dirPath, name: parts[i - 1], isDir: true, children: [] });
      }
    }
    byPath.set(f.rel_path, {
      id: f.rel_path,
      name: parts[parts.length - 1] || f.rel_path,
      isDir: false,
      file: f,
      children: [],
    });
  });

  byPath.forEach((node, path) => {
    const lastSlash = path.lastIndexOf("/");
    if (lastSlash > 0) {
      const parentPath = path.slice(0, lastSlash);
      if (byPath.has(parentPath)) {
        byPath.get(parentPath).children.push(node);
        return;
      }
    }
    roots.push(node);
  });

  return roots;
}

function sortGitNodes(nodes) {
  return [...nodes]
    .sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    })
    .map((n) => ({ ...n, children: sortGitNodes(n.children) }));
}

function GitTreeNodes({ nodes, setFiles }) {
  return (
    <>
      {nodes.map((node) =>
        node.isDir ? (
          <li key={node.id} className="project-tree-branch">
            <details className="project-tree-details" open>
              <summary className="project-tree-summary git-tree-dir">
                <span className="project-tree-caret">
                  <svg viewBox="0 0 24 24" fill="none" width="12" height="12" aria-hidden="true">
                    <path d="M7 10l5 5 5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </span>
                <span className="project-tree-segment">{node.name}</span>
              </summary>
              <div className="project-tree-body">
                <ul className="project-tree-list">
                  <GitTreeNodes nodes={sortGitNodes(node.children)} setFiles={setFiles} />
                </ul>
              </div>
            </details>
          </li>
        ) : (
          <li key={node.id} className="project-tree-leaf">
            <div className="project-tree-leaf-link git-tree-file">
              <Checkbox
                checked={node.file.checked}
                onChange={(e) =>
                  setFiles((prev) =>
                    prev.map((x) =>
                      x.rel_path === node.file.rel_path ? { ...x, checked: e.target.checked } : x
                    )
                  )
                }
                className="git-tree-check"
              />
              <span className="git-tree-file-name" title={node.file.rel_path}>{node.name}</span>
              <code className={cx(
                "git-tree-code",
                gitStatusChar(node.file.code) === "A" && "is-added",
                gitStatusChar(node.file.code) === "D" && "is-deleted",
                gitStatusChar(node.file.code) === "M" && "is-modified",
                gitStatusChar(node.file.code) === "U" && "is-untracked",
              )}>
                {gitStatusChar(node.file.code)}
              </code>
            </div>
          </li>
        )
      )}
    </>
  );
}

function GitFileTree({ files, setFiles }) {
  const roots = sortGitNodes(buildGitFileTree(files));
  if (roots.length === 0) return null;
  return (
    <ul className="project-tree-root git-tree-root">
      <GitTreeNodes nodes={roots} setFiles={setFiles} />
    </ul>
  );
}

function GitPanel({ owner, project }) {
  const [open, setOpen] = useState(false);
  const [files, setFiles] = useState([]);
  const [loading, setLoading] = useState(false);
  const [synced, setSynced] = useState(true);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const statusUrl = `/api/projects/${owner}/${project}/git/status`;
  const commitUrl = `/api/projects/${owner}/${project}/git/commit`;

  async function fetchStatus() {
    if (!owner || !project) return;
    setLoading(true);
    try {
      const res = await fetch(statusUrl, { headers: { Accept: "application/json" } });
      const data = await res.json().catch(() => []);
      setFiles(Array.isArray(data) ? data.map((f) => ({ ...f, checked: true })) : []);
    } catch (_) {}
    setLoading(false);
  }

  useEffect(() => {
    if (typeof window === "undefined") return;
    fetchStatus();
    const repoHandler = () => { fetchStatus(); setSynced(false); };
    const panelHandler = (e) => { if (e.detail?.panel !== "git") setOpen(false); };
    const navHandler = () => setOpen(false);
    window.addEventListener("zf:repo:changed", repoHandler);
    window.addEventListener("zf:panel:opened", panelHandler);
    window.addEventListener("rwe:nav", navHandler);
    return () => {
      window.removeEventListener("zf:repo:changed", repoHandler);
      window.removeEventListener("zf:panel:opened", panelHandler);
      window.removeEventListener("rwe:nav", navHandler);
    };
  }, []);

  function toggle() {
    if (!open) { fetchStatus(); dispatchPanelOpen("git"); }
    setOpen((o) => !o);
  }

  async function doCommit(push) {
    const checked = files.filter((f) => f.checked).map((f) => f.rel_path);
    if (!checked.length || !message.trim()) return;
    setBusy(true);
    setError("");
    try {
      const res = await fetch(commitUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ files: checked, message: message.trim(), push }),
      });
      const body = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(body?.error?.message || body?.message || "Failed");
      setMessage("");
      if (push) setSynced(true);
      await fetchStatus();
    } catch (e) {
      setError(e.message || "Error");
    }
    setBusy(false);
  }

  // Partition staged (X != ' ') vs unstaged/untracked
  const staged = files.filter((f) => {
    const x = (f.code ?? " ")[0];
    return x !== " " && x !== "?";
  });
  const unstaged = files.filter((f) => {
    const x = (f.code ?? " ")[0];
    const y = (f.code ?? " ")[1];
    return x === "?" || (y && y !== " ");
  });

  const count = files.length;
  const checkedCount = files.filter((f) => f.checked).length;

  return (
    <div className="relative">
      <Button
        variant="outline"
        size="icon"
        onClick={toggle}
        title={count > 0 ? `${count} change${count !== 1 ? "s" : ""}` : "Git — clean"}
        className="git-indicator-btn"
      >
        <GitBranchIcon />
        {count > 0 && <span className="git-indicator-badge">{count}</span>}
        {count === 0 && <span className="git-indicator-dot is-clean" />}
      </Button>

      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="git-panel z-50">
            {/* Header */}
            <div className="git-panel-header">
              <div className="git-panel-left">
                <GitBranchIcon className="w-3.5 h-3.5 shrink-0" />
                <span className="git-panel-branch">repo</span>
              </div>
              <div className="git-panel-right">
                <span className="git-panel-count">{loading ? "…" : `${count} change${count !== 1 ? "s" : ""}`}</span>
                <span className={cx("git-panel-sync-dot", synced && count === 0 && "is-synced")} />
              </div>
            </div>

            {/* File tree */}
            <div className="git-file-list">
              {staged.length > 0 && (
                <>
                  <p className="git-section-label">STAGED CHANGES</p>
                  <GitFileTree files={staged} setFiles={setFiles} />
                </>
              )}
              {unstaged.length > 0 && (
                <>
                  <p className="git-section-label">CHANGES</p>
                  <GitFileTree files={unstaged} setFiles={setFiles} />
                </>
              )}
              {count === 0 && !loading && <p className="git-empty">Working tree clean.</p>}
              {loading && <p className="git-empty">Loading…</p>}
            </div>

            {/* Commit area */}
            {count > 0 && (
              <div className="git-actions">
                <Input
                  value={message}
                  onInput={(e) => setMessage(e.target.value)}
                  placeholder="Commit message…"
                  className="git-message-input"
                />
                {error && <p className="git-error">{error}</p>}
                <div className="git-action-row">
                  <Button
                    size="xs"
                    onClick={() => doCommit(false)}
                    disabled={busy || !message.trim() || !checkedCount}
                  >
                    Commit
                  </Button>
                  <Button
                    size="xs"
                    variant="outline"
                    className={cx("git-sync-btn", synced && "is-synced")}
                    onClick={() => doCommit(true)}
                    disabled={busy || !message.trim() || !checkedCount}
                  >
                    {synced ? "✓ Synced" : "↑ Sync"}
                  </Button>
                </div>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

// ── Session Panel ─────────────────────────────────────────────────────────────

const GUIDE_CLIENTS = [
  { id: "claude-code", label: "Claude Code" },
  { id: "cursor",      label: "Cursor" },
  { id: "codex",       label: "Codex" },
  { id: "gemini",      label: "Gemini" },
  { id: "opencode",    label: "OpenCode" },
  { id: "cline",       label: "Cline" },
];

function buildGuideSnippet(clientId, mcpUrl, token) {
  const url = mcpUrl ?? "<MCP_URL>";
  const tok = token ?? "<TOKEN>";
  const jsonBlock = `{\n  "mcpServers": {\n    "zebflow": {\n      "url": "${url}",\n      "headers": {\n        "Authorization": "Bearer ${tok}"\n      }\n    }\n  }\n}`;
  switch (clientId) {
    case "claude-code":
      return `claude mcp add --transport http zebflow \\\n  ${url} \\\n  --header "Authorization: Bearer ${tok}"`;
    case "cursor":
      return `// ~/.cursor/mcp.json\n${jsonBlock}`;
    case "codex":
      return `// ~/.codex/config.json\n${jsonBlock}`;
    case "gemini":
      return `// ~/.gemini/settings.json\n${jsonBlock}`;
    case "opencode":
      return `# ~/.config/opencode/config.toml\n[mcp.zebflow]\ntype   = "remote"\nurl    = "${url}"\n\n[mcp.zebflow.headers]\nAuthorization = "Bearer ${tok}"`;
    case "cline":
      return `// Cline → MCP Servers → Add Server (HTTP)\n// or in VS Code settings.json:\n${JSON.stringify({ "cline.mcpServers": { zebflow: { url, headers: { Authorization: `Bearer ${tok}` } } } }, null, 2)}`;
    default:
      return jsonBlock;
  }
}

const DEFAULT_CAPABILITIES = [
  { key: "project.read",      label: "Project Read",      defaultOn: true  },
  { key: "pipelines.read",    label: "Pipelines Read",    defaultOn: true  },
  { key: "pipelines.write",   label: "Pipelines Write",   defaultOn: false },
  { key: "pipelines.execute", label: "Pipelines Execute", defaultOn: false },
  { key: "templates.read",    label: "Templates Read",    defaultOn: true  },
  { key: "templates.write",   label: "Templates Write",   defaultOn: false },
  { key: "templates.create",  label: "Templates Create",  defaultOn: false },
  { key: "settings.read",     label: "Settings Read",     defaultOn: true  },
  { key: "settings.write",    label: "Settings Write",    defaultOn: false },
  { key: "credentials.read",  label: "Credentials Read",  defaultOn: false },
  { key: "tables.read",       label: "Tables Read",       defaultOn: true  },
];

function SessionPanel({ owner, project }) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [enabled, setEnabled] = useState(false);
  const [token, setToken] = useState(null);
  const [mcpUrl, setMcpUrl] = useState(null);
  const [capabilities, setCapabilities] = useState(
    DEFAULT_CAPABILITIES.filter((c) => c.defaultOn).map((c) => c.key)
  );
  const [resetting, setResetting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");
  const [guideClient, setGuideClient] = useState("claude-code");
  const [guideCopied, setGuideCopied] = useState(false);
  const sessionDetailsRef = useRef(null);

  const sessionUrl = `/api/projects/${owner}/${project}/mcp/session`;

  async function fetchSession() {
    if (!owner || !project) return;
    setLoading(true);
    setError("");
    try {
      const res = await fetch(sessionUrl, { headers: { Accept: "application/json" } });
      const data = await res.json().catch(() => ({}));
      if (data?.ok && data?.session) {
        const s = data.session;
        setEnabled(!!s.enabled);
        setToken(s.token ?? null);
        setMcpUrl(s.mcp_url ?? null);
        if (s.capabilities?.length) setCapabilities(s.capabilities);
      }
    } catch (_) {}
    setLoading(false);
  }

  useEffect(() => {
    if (open) fetchSession();
  }, [open]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const panelHandler = (e) => {
      if (e.detail?.panel !== "session" && sessionDetailsRef.current) {
        sessionDetailsRef.current.open = false;
        setOpen(false);
      }
    };
    window.addEventListener("zf:panel:opened", panelHandler);
    return () => window.removeEventListener("zf:panel:opened", panelHandler);
  }, []);

  async function handleToggle(e) {
    const next = e.target.checked;
    setError("");

    if (!token) {
      // First enable — create the session
      setLoading(true);
      try {
        const res = await fetch(sessionUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json", Accept: "application/json" },
          body: JSON.stringify({ capabilities }),
        });
        const data = await res.json().catch(() => ({}));
        if (data?.ok && data?.session) {
          setEnabled(true);
          setToken(data.session.token ?? null);
          setMcpUrl(data.session.mcp_url ?? null);
        } else {
          setError(data?.error?.message ?? "Failed to create session");
        }
      } catch (_) {
        setError("Network error");
      }
      setLoading(false);
    } else {
      // Soft toggle
      setEnabled(next);
      try {
        await fetch(sessionUrl, {
          method: "PUT",
          headers: { "Content-Type": "application/json", Accept: "application/json" },
          body: JSON.stringify({ enabled: next }),
        });
      } catch (_) {
        setEnabled(!next); // revert on error
        setError("Failed to update session");
      }
    }
  }

  async function handleCapabilityChange(key, checked) {
    const next = checked
      ? [...capabilities, key]
      : capabilities.filter((c) => c !== key);
    setCapabilities(next);

    // If session exists, update capabilities immediately (idempotent POST keeps token)
    if (token) {
      try {
        await fetch(sessionUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json", Accept: "application/json" },
          body: JSON.stringify({ capabilities: next }),
        });
      } catch (_) {}
    }
  }

  async function handleResetToken() {
    if (resetting) return;
    setResetting(true);
    setError("");
    try {
      const res = await fetch(`${sessionUrl}/reset-token`, {
        method: "POST",
        headers: { Accept: "application/json" },
      });
      const data = await res.json().catch(() => ({}));
      if (data?.ok && data?.session) {
        setToken(data.session.token ?? null);
        setEnabled(true);
      } else {
        setError(data?.error?.message ?? "Failed to reset token");
      }
    } catch (_) {
      setError("Network error");
    }
    setResetting(false);
  }

  async function handleCopy() {
    if (!token) return;
    try {
      await navigator.clipboard.writeText(token);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (_) {}
  }

  return (
    <details ref={sessionDetailsRef} className="relative inline-block group project-shell-session" data-dropdown-menu="true">
      <summary
        className={cx(
          "list-none cursor-pointer outline-none",
          "inline-flex items-center gap-1.5 h-8 px-2.5 rounded-lg border transition-all text-sm",
          "border-[var(--studio-border)] bg-[var(--studio-panel-2)]",
          "text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] hover:bg-[var(--studio-panel-3)]",
          enabled && "border-green-800/60 text-green-400 hover:text-green-300"
        )}
        onClick={() => {
          if (!open) dispatchPanelOpen("session");
          setOpen((o) => !o);
        }}
      >
        <SessionIcon />
        <span className="text-xs">MCP</span>
        {enabled && <span className="w-1.5 h-1.5 rounded-full bg-green-500 shrink-0" />}
      </summary>

      <DropdownMenuContent
        align="right"
        className="mcp-session-panel w-[540px] border-[var(--studio-border)] bg-[var(--studio-panel)]"
      >
        {/* Header — full width */}
        <div className="mcp-session-header">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="text-sm font-semibold text-[var(--studio-text)]">MCP Session</p>
              <span className={cx(
                "inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[0.6rem] font-semibold tracking-wide",
                enabled
                  ? "bg-green-900/40 text-green-400 border border-green-800/60"
                  : "bg-[var(--studio-panel-3)] text-[var(--studio-text-soft)] border border-[var(--studio-border)]"
              )}>
                <span className={cx("w-1.5 h-1.5 rounded-full", enabled ? "bg-green-500" : "bg-slate-500")} />
                {enabled ? "Active" : "Inactive"}
              </span>
            </div>
            <p className="text-[0.68rem] text-[var(--studio-text-soft)] mt-0.5 leading-snug">
              Remote control for LLM agents (Cursor, Claude Code, etc.)
            </p>
          </div>
          <Toggle
            checked={enabled}
            onChange={handleToggle}
            disabled={loading}
            aria-label="Enable MCP session"
          />
        </div>

        {/* Split body — left: connection + guide · right: permissions */}
        <div className="mcp-split-body">

          {/* ── Left column ── */}
          <div className="mcp-split-left">

            {/* Token */}
            <div className="mcp-session-section">
              <div className="flex items-center justify-between mb-1.5">
                <label htmlFor="mcp-token-input" className="mcp-session-label">Token</label>
                {token && (
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={handleResetToken}
                    disabled={resetting}
                    className="mcp-reset-btn"
                  >
                    {resetting ? "Resetting…" : "↺ Reset"}
                  </Button>
                )}
              </div>
              <div className="flex gap-1.5">
                <input
                  id="mcp-token-input"
                  readOnly
                  value={token ?? ""}
                  placeholder={loading ? "Loading…" : "Enable to generate token"}
                  title={token ?? undefined}
                  className="mcp-token-input flex-1"
                />
                <Button
                  variant={copied ? "default" : "outline"}
                  size="sm"
                  onClick={handleCopy}
                  disabled={!token}
                  className="mcp-copy-btn shrink-0"
                  aria-label="Copy token to clipboard"
                >
                  {copied ? "✓" : "Copy"}
                </Button>
              </div>
            </div>

            {/* MCP URL */}
            <div className="mcp-session-section">
              <label htmlFor="mcp-url-input" className="mcp-session-label block mb-1.5">MCP URL</label>
              <div className="flex gap-1.5">
                <input
                  id="mcp-url-input"
                  readOnly
                  value={mcpUrl ?? ""}
                  title={mcpUrl ?? undefined}
                  placeholder="Enable session to get URL"
                  className="mcp-token-input flex-1"
                />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={async () => {
                    if (!mcpUrl) return;
                    try { await navigator.clipboard.writeText(mcpUrl); } catch (_) {}
                  }}
                  disabled={!mcpUrl}
                  className="mcp-copy-btn shrink-0"
                  aria-label="Copy MCP URL"
                >URL</Button>
              </div>
            </div>

            {/* Install Guide */}
            <div className="mcp-session-section mcp-guide-section">
              <p className="mcp-session-label mb-2">Install Guide</p>
              <div className="mcp-guide-tabs">
                {GUIDE_CLIENTS.map((c) => (
                  <button
                    key={c.id}
                    onClick={() => setGuideClient(c.id)}
                    className={cx("mcp-guide-tab", guideClient === c.id && "is-active")}
                  >{c.label}</button>
                ))}
              </div>
              <pre className="mcp-guide-snippet">{buildGuideSnippet(guideClient, mcpUrl, token)}</pre>
              <Button
                variant="outline"
                size="xs"
                className="mcp-guide-copy-btn"
                onClick={async () => {
                  try {
                    await navigator.clipboard.writeText(buildGuideSnippet(guideClient, mcpUrl, token));
                    setGuideCopied(true);
                    setTimeout(() => setGuideCopied(false), 2000);
                  } catch (_) {}
                }}
              >{guideCopied ? "✓ Copied" : "Copy"}</Button>
            </div>

          </div>{/* /mcp-split-left */}

          {/* ── Divider ── */}
          <div className="mcp-split-divider" />

          {/* ── Right column: Permissions ── */}
          <div className="mcp-split-right">
            <p className="mcp-session-label mb-2.5">Permissions</p>
            <div className="flex flex-col gap-0.5">
              {DEFAULT_CAPABILITIES.map((cap) => (
                <Checkbox
                  key={cap.key}
                  label={cap.label}
                  checked={capabilities.includes(cap.key)}
                  onChange={(e) => handleCapabilityChange(cap.key, e.target.checked)}
                  className="mcp-cap-check"
                />
              ))}
            </div>
          </div>

        </div>{/* /mcp-split-body */}

        {/* Error — full width */}
        {error && (
          <p className="px-4 pb-3 text-[0.68rem] text-red-400 border-t border-[var(--studio-border)]" role="alert">{error}</p>
        )}
      </DropdownMenuContent>
    </details>
  );
}

// ── RepoPanel ─────────────────────────────────────────────────────────────────

function RepoPanel({ owner, project }) {
  const [open, setOpen] = useState(false);
  const [creds, setCreds] = useState([]);
  const [selectedId, setSelectedId] = useState("");
  const [repoUrl, setRepoUrl] = useState("");
  const [branch, setBranch] = useState("main");
  const [saveState, setSaveState] = useState("idle"); // idle | saved | error
  const [saveMsg, setSaveMsg] = useState("");
  const [loading, setLoading] = useState(false);
  const repoPanelRef = useRef(null);

  const storageKey = `zf-repo-${owner}-${project}`;
  const credApiUrl = `/api/projects/${owner}/${project}/credentials`;

  function loadSaved() {
    try {
      const saved = JSON.parse(localStorage.getItem(storageKey) || "null");
      if (saved) {
        setSelectedId(saved.credential_id ?? "");
        setRepoUrl(saved.repo_url ?? "");
        setBranch(saved.branch ?? "main");
      }
    } catch (_) {}
  }

  async function fetchCreds() {
    if (!owner || !project) return;
    setLoading(true);
    try {
      const res = await fetch(credApiUrl, { headers: { Accept: "application/json" } });
      const data = await res.json().catch(() => ({}));
      const items = (Array.isArray(data?.items) ? data.items : []).filter(
        (c) => c.kind === "github" || c.kind === "gitlab"
      );
      setCreds(items);
    } catch (_) {}
    setLoading(false);
  }

  useEffect(() => {
    if (typeof window === "undefined") return;
    const panelHandler = (e) => {
      if (e.detail?.panel !== "repo" && repoPanelRef.current) {
        repoPanelRef.current.open = false;
        setOpen(false);
      }
    };
    window.addEventListener("zf:panel:opened", panelHandler);
    return () => window.removeEventListener("zf:panel:opened", panelHandler);
  }, []);

  function toggle() {
    if (!open) {
      loadSaved();
      fetchCreds();
      dispatchPanelOpen("repo");
    }
    setOpen((o) => !o);
  }

  function handleConnect() {
    if (!selectedId || !repoUrl.trim()) {
      setSaveMsg("Select a credential and enter a clone URL.");
      setSaveState("error");
      return;
    }
    try {
      localStorage.setItem(storageKey, JSON.stringify({
        credential_id: selectedId,
        repo_url: repoUrl.trim(),
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
    setRepoUrl("");
    setBranch("main");
    setSaveState("idle");
    setSaveMsg("");
  }

  const connected = (() => {
    try { return !!JSON.parse(localStorage.getItem(storageKey) || "null"); } catch (_) { return false; }
  })();

  const selectedCred = creds.find((c) => c.credential_id === selectedId) ?? null;

  const credHost = selectedCred?.kind === "github"
    ? "github.com"
    : selectedCred?.kind === "gitlab"
      ? (selectedCred?.secret?.url || "gitlab.com")
      : "";

  return (
    <details ref={repoPanelRef} className="relative inline-block group project-shell-repo" data-dropdown-menu="true">
      <summary
        className={cx(
          "list-none cursor-pointer outline-none",
          "inline-flex items-center gap-1.5 h-8 px-2.5 rounded-lg border transition-all text-sm",
          "border-[var(--studio-border)] bg-[var(--studio-panel-2)]",
          "text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] hover:bg-[var(--studio-panel-3)]",
          connected && "border-blue-800/60 text-blue-400 hover:text-blue-300"
        )}
        onClick={() => toggle()}
      >
        <RepoIcon />
        <span className="text-xs">Repo</span>
        {connected && <span className="w-1.5 h-1.5 rounded-full bg-blue-500 shrink-0" />}
      </summary>

      <DropdownMenuContent
        align="right"
        className="repo-panel w-[480px] border-[var(--studio-border)] bg-[var(--studio-panel)]"
      >
        {/* Header */}
        <div className="mcp-session-header">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="text-sm font-semibold text-[var(--studio-text)]">Git Repository</p>
              <span className={cx(
                "inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[0.6rem] font-semibold tracking-wide",
                connected
                  ? "bg-blue-900/40 text-blue-400 border border-blue-800/60"
                  : "bg-[var(--studio-panel-3)] text-[var(--studio-text-soft)] border border-[var(--studio-border)]"
              )}>
                <span className={cx("w-1.5 h-1.5 rounded-full", connected ? "bg-blue-500" : "bg-slate-500")} />
                {connected ? "Connected" : "Not connected"}
              </span>
            </div>
            <p className="text-[0.68rem] text-[var(--studio-text-soft)] mt-0.5 leading-snug">
              Link to a GitHub or GitLab repository credential.
            </p>
          </div>
          {connected && (
            <Button variant="ghost" size="xs" onClick={handleDisconnect} className="mcp-reset-btn shrink-0">
              Disconnect
            </Button>
          )}
        </div>

        {/* Split body */}
        <div className="mcp-split-body">

          {/* Left: credential selector + preview */}
          <div className="mcp-split-left">
            <div className="mcp-session-section">
              <label className="mcp-session-label block mb-1.5">Remote Credential</label>
              {loading ? (
                <p className="text-[0.7rem] text-[var(--studio-text-muted)]">Loading…</p>
              ) : creds.length === 0 ? (
                <p className="text-[0.7rem] text-[var(--studio-text-soft)] leading-snug">
                  No GitHub / GitLab credentials.{" "}
                  <a
                    href={`/projects/${owner}/${project}/settings`}
                    className="underline underline-offset-2 text-[var(--studio-accent)]"
                  >Create one</a>
                </p>
              ) : (
                <select
                  className="mcp-token-input w-full"
                  value={selectedId}
                  onChange={(e) => setSelectedId(e.target.value)}
                >
                  <option value="">— select —</option>
                  {creds.map((c) => (
                    <option key={c.credential_id} value={c.credential_id}>
                      {c.title} · {c.kind}
                    </option>
                  ))}
                </select>
              )}
            </div>

            {/* Credential preview card */}
            {selectedCred && (
              <div className="mcp-session-section">
                <div className="repo-cred-card">
                  <span className={cx("repo-cred-badge", `is-${selectedCred.kind}`)}>
                    {selectedCred.kind === "github" ? "GitHub" : "GitLab"}
                  </span>
                  <div className="repo-cred-info">
                    <span className="repo-cred-host">{credHost}</span>
                    {selectedCred.secret?.username && (
                      <span className="repo-cred-user">@{selectedCred.secret.username}</span>
                    )}
                    <span className="repo-cred-token-status">
                      {selectedCred.has_secret ? "Token: configured" : "Token: not set"}
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Divider */}
          <div className="mcp-split-divider" />

          {/* Right: clone URL + branch + connect */}
          <div className="mcp-split-right">
            <div className="flex flex-col gap-2.5">
              <div>
                <label className="mcp-session-label block mb-1">Clone URL</label>
                <input
                  className="mcp-token-input w-full"
                  type="text"
                  placeholder="https://github.com/org/repo.git"
                  value={repoUrl}
                  onInput={(e) => setRepoUrl(e.target.value)}
                />
              </div>
              <div>
                <label className="mcp-session-label block mb-1">Branch</label>
                <input
                  className="mcp-token-input w-full"
                  type="text"
                  placeholder="main"
                  value={branch}
                  onInput={(e) => setBranch(e.target.value)}
                />
              </div>
              <Button
                variant={saveState === "saved" ? "default" : "outline"}
                size="sm"
                onClick={handleConnect}
                className="w-full mt-0.5"
              >
                {saveState === "saved" ? "✓ Connected" : "Connect"}
              </Button>
              {saveMsg && (
                <p className={cx(
                  "text-[0.68rem]",
                  saveState === "error" ? "text-red-400" : "text-green-400"
                )}>{saveMsg}</p>
              )}
            </div>
          </div>

        </div>{/* /mcp-split-body */}

      </DropdownMenuContent>
    </details>
  );
}

// ── Layout Shell ─────────────────────────────────────────────────────────────

export default function ProjectStudioShell(props) {
  const [theme, setTheme] = useState("dark");
  const nav = props?.nav ?? {};
  const owner = props?.owner ?? "";
  const project = props?.project ?? "";

  useEffect(() => {
    initProjectShellBehavior();
  }, []);

  return (
    <div className="project-studio-shell">
      <div className="project-studio-frame" data-theme={theme}>
        <PlatformSidebar nav={nav} />

        <main className="project-shell-main">
          <header className="project-shell-header">
            <div className="flex items-center justify-between px-4 h-10">
              {/* Breadcrumb */}
              <nav className="flex items-center gap-2 text-[0.78rem] leading-none min-w-0">
                <Link
                  href="/home"
                  className="text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] transition-colors"
                  aria-label="Go to home"
                >
                  <HomeIcon />
                </Link>
                <span className="text-[var(--studio-border)] select-none">/</span>
                <Link
                  href={props?.projectHref ?? "#"}
                  className="text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] transition-colors truncate"
                >
                  {props?.projectLabel ?? "Project"}
                </Link>
                <span className="text-[var(--studio-border)] select-none">/</span>
                <span className="text-[var(--studio-text)] font-medium" data-rwe-breadcrumb>
                  {props?.currentMenu ?? "Workspace"}
                </span>
              </nav>

              {/* Tool buttons */}
              <div className="flex items-center gap-1.5">
                {/* Theme toggle */}
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
                  title="Toggle theme"
                >
                  <span className="project-shell-theme-dark">
                    <MoonIcon />
                  </span>
                  <span className="project-shell-theme-light">
                    <SunIcon />
                  </span>
                </Button>

                {/* Console trigger */}
                <Button
                  variant="outline"
                  size="icon"
                  title="Console (` to toggle)"
                  data-console-trigger="true"
                  data-owner={owner}
                  data-project={project}
                >
                  <TerminalIcon />
                </Button>

                {/* Git */}
                <GitPanel owner={owner} project={project} />

                {/* Repo */}
                <RepoPanel owner={owner} project={project} />

                {/* MCP Session */}
                <SessionPanel owner={owner} project={project} />
              </div>
            </div>
          </header>

          <section className="project-shell-workspace" data-rwe-outlet>
            {props?.children}
          </section>
        </main>
      </div>

      {/* Console — teleported to document.body by behavior on first mount */}
      <ConsolePanel owner={owner} project={project}>
        <ConsoleOutput />
      </ConsolePanel>

      {/* AutoOverlay — activated by InteractionRunner via patchOverlay() */}
      <AutoOverlay />

    </div>
  );
}
