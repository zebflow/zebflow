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

// Only capabilities that are actually enforced by a real MCP tool
const DEFAULT_CAPABILITIES = [
  // Core
  { key: "project.read",       label: "Project Read",      defaultOn: true  },
  // Pipelines
  { key: "pipelines.read",     label: "Pipelines Read",    defaultOn: true  },
  { key: "pipelines.write",    label: "Pipelines Write",   defaultOn: false },
  { key: "pipelines.execute",  label: "Pipelines Execute", defaultOn: false },
  // Templates
  { key: "templates.read",     label: "Templates Read",    defaultOn: true  },
  { key: "templates.write",    label: "Templates Write",   defaultOn: false },
  { key: "templates.create",   label: "Templates Create",  defaultOn: false },
  { key: "templates.delete",   label: "Templates Delete",  defaultOn: false },
  // Files
  { key: "files.write",        label: "Files Write",       defaultOn: false },
  // Tables / DB
  { key: "tables.read",        label: "Tables Read",       defaultOn: true  },
  { key: "tables.write",       label: "Tables Write",      defaultOn: false },
  // Credentials
  { key: "credentials.read",   label: "Credentials Read",  defaultOn: false },
  { key: "credentials.write",  label: "Credentials Write", defaultOn: false },
  // Settings
  { key: "settings.read",      label: "Settings Read",     defaultOn: true  },
  { key: "settings.write",     label: "Settings Write",    defaultOn: false },
];

export function SessionPanel({ owner, project }) {
  const { activePanel, openHeaderPanel } = useStudioChrome();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [activating, setActivating] = useState(false);
  const [enabled, setEnabled] = useState(false);
  const [token, setToken] = useState(null);
  const [mcpUrl, setMcpUrl] = useState(null);
  const [capabilities, setCapabilities] = useState(
    DEFAULT_CAPABILITIES.filter((c) => c.defaultOn).map((c) => c.key)
  );
  const [resetting, setResetting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [urlCopied, setUrlCopied] = useState(false);
  const [error, setError] = useState("");
  const [guideClient, setGuideClient] = useState("claude-code");
  const [guideCopied, setGuideCopied] = useState(false);
  const [deactivating, setDeactivating] = useState(false);
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
    fetchSession();
  }, []);

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
        setEnabled(!next);
        setError("Failed to update session");
      }
    }
  }

  async function handleDeactivate() {
    if (deactivating) return;
    setDeactivating(true);
    setError("");
    try {
      await fetch(sessionUrl, {
        method: "PUT",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ enabled: false }),
      });
      setEnabled(false);
    } catch (_) {
      setError("Failed to deactivate session");
    }
    setDeactivating(false);
  }

  async function handleActivate() {
    if (activating) return;
    setActivating(true);
    setError("");
    try {
      await fetch(sessionUrl, {
        method: "PUT",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ enabled: true }),
      });
      await fetchSession();
    } catch (_) {
      setError("Failed to activate session");
    }
    setActivating(false);
  }

  async function handleCapabilityChange(key, checked) {
    const next = checked
      ? [...capabilities, key]
      : capabilities.filter((c) => c !== key);
    setCapabilities(next);

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

  async function handleCopyToken() {
    if (!token) return;
    try {
      await navigator.clipboard.writeText(token);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (_) {}
  }

  async function handleCopyUrl() {
    if (!mcpUrl) return;
    try {
      await navigator.clipboard.writeText(mcpUrl);
      setUrlCopied(true);
      setTimeout(() => setUrlCopied(false), 2000);
    } catch (_) {}
  }

  const labelCls = "text-[0.65rem] font-semibold tracking-widest text-body-soft uppercase";
  const sectionCls = "px-4 py-3 border-b border-border";

  return (
    <details ref={sessionDetailsRef} className="relative inline-block group" data-dropdown-menu="true">
      <summary
        className="list-none cursor-pointer outline-none relative flex items-center justify-center h-9 w-9 rounded-none bg-dark-accent1 !text-dark-menus"
        onClick={() => {
          if (!open) openHeaderPanel("session");
          setOpen((o) => !o);
        }}
      >
        <SessionIcon />
        {enabled && <span className="absolute bottom-0 right-0 w-[7px] h-[7px] bg-green-500 pointer-events-none" />}
      </summary>

      <DropdownMenuContent
        align="right"
        className="w-[760px] border-border bg-surface"
      >
        {/* ── Header ── */}
        <div className="flex items-center justify-between gap-3 px-4 py-3 border-b border-border">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <p className="text-sm font-semibold text-body">MCP Session</p>
              <span className={cx(
                "inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[0.6rem] font-semibold tracking-wide",
                enabled
                  ? "bg-green-900/40 text-green-400 border border-green-800/60"
                  : "bg-surface-3 text-body-soft border border-border"
              )}>
                <span className={cx("w-1.5 h-1.5 rounded-full", enabled ? "bg-green-500" : "bg-gray-500")} />
                {enabled ? "Active" : "Inactive"}
              </span>
            </div>
            <p className="text-[0.68rem] text-body-soft mt-0.5 leading-snug">
              Remote control for LLM agents (Cursor, Claude Code, etc.)
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {!enabled && (
              <Button
                variant="outline"
                size="xs"
                onClick={handleActivate}
                disabled={activating}
                className="text-green-400 border-green-800/50 hover:bg-green-900/20 hover:text-green-300"
              >
                {activating ? "Activating…" : "Activate"}
              </Button>
            )}
            {enabled && token && (
              <Button
                variant="outline"
                size="xs"
                onClick={handleDeactivate}
                disabled={deactivating}
                className="text-red-400 border-red-800/50 hover:bg-red-900/20 hover:text-red-300"
              >
                {deactivating ? "Deactivating…" : "Deactivate"}
              </Button>
            )}
            <Toggle
              checked={enabled}
              onChange={handleToggle}
              disabled={loading}
              aria-label="Enable MCP session"
            />
          </div>
        </div>

        {/* ── Split body ── */}
        <div className="flex items-stretch">

          {/* Left column */}
          <div className="flex-1 min-w-0 flex flex-col">

            {/* Token */}
            <div className={sectionCls}>
              <div className="flex items-center justify-between mb-1.5">
                <label htmlFor="mcp-token-input" className={labelCls}>Token</label>
                {token && (
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={handleResetToken}
                    disabled={resetting}
                    className="text-[0.68rem] text-body-soft hover:text-body"
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
                  onClick={handleCopyToken}
                  disabled={!token}
                  className="shrink-0"
                  aria-label="Copy token to clipboard"
                >
                  {copied ? "✓" : "Copy"}
                </Button>
              </div>
            </div>

            {/* MCP URL */}
            <div className={sectionCls}>
              <label htmlFor="mcp-url-input" className={cx(labelCls, "block mb-1.5")}>MCP URL</label>
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
                  variant={urlCopied ? "default" : "outline"}
                  size="sm"
                  onClick={handleCopyUrl}
                  disabled={!mcpUrl}
                  className="shrink-0"
                  aria-label="Copy MCP URL"
                >
                  {urlCopied ? "✓" : "URL"}
                </Button>
              </div>
            </div>

            {/* Install Guide */}
            <div className={cx(sectionCls, "border-b-0 flex-1")}>
              <p className={cx(labelCls, "mb-2")}>Install Guide</p>
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

          </div>{/* /left */}

          {/* Divider */}
          <div className="w-px bg-border flex-shrink-0" />

          {/* Right column: Permissions */}
          <div className="w-[340px] flex-shrink-0 p-3 flex flex-col">
            <p className={cx(labelCls, "mb-2.5")}>Permissions</p>
            <div className="grid grid-cols-2 gap-x-2 gap-y-0.5 overflow-y-auto max-h-[380px]">
              {DEFAULT_CAPABILITIES.map((cap) => (
                <Checkbox
                  key={cap.key}
                  label={cap.label}
                  checked={capabilities.includes(cap.key)}
                  onChange={(e) => handleCapabilityChange(cap.key, e.target.checked)}
                />
              ))}
            </div>
          </div>

        </div>{/* /split */}

        {/* Error */}
        {error && (
          <p className="px-4 pb-3 text-[0.68rem] text-red-400 border-t border-border" role="alert">{error}</p>
        )}
      </DropdownMenuContent>
    </details>
  );
}
