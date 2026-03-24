import { useState, useEffect, useRef, cx } from "zeb";
import { useWindowEvent } from "zeb/use";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import Toggle from "@/components/ui/toggle";
import DropdownMenuContent from "@/components/ui/dropdown-menu-content";
import { SessionIcon } from "@/pages/project-studio/components/icons";
import { useStudioChrome } from "@/pages/project-studio/components/studio-chrome-context";

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

export function SessionPanel({ owner, project }) {
  const { activePanel, openHeaderPanel } = useStudioChrome();
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
    const d = sessionDetailsRef.current;
    if (activePanel !== "session" && d?.open) {
      d.open = false;
      setOpen(false);
    }
  }, [activePanel]);

  useWindowEvent("rwe:nav", () => {
    const d = sessionDetailsRef.current;
    if (d?.open) {
      d.open = false;
      setOpen(false);
    }
  });

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
    <details ref={sessionDetailsRef} className="relative inline-block group" data-dropdown-menu="true">
      <summary
        className={cx(
          "list-none cursor-pointer outline-none",
          "inline-flex items-center gap-1.5 h-8 px-2.5 rounded-lg border transition-all text-sm",
          "border-[var(--studio-border)] bg-[var(--studio-panel-2)]",
          "text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] hover:bg-[var(--studio-panel-3)]",
          enabled && "border-green-800/60 text-green-400 hover:text-green-300"
        )}
        onClick={() => {
          if (!open) openHeaderPanel("session");
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
