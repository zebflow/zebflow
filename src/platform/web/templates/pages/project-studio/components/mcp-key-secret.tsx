import { useState, cx } from "zeb/react";
import Button from "@/components/ui/button";
import { GUIDE_CLIENTS, buildGuideSnippet } from "@/pages/project-studio/components/mcp-key-guide";

const labelCls = "block text-[0.65rem] font-semibold tracking-widest text-muted-foreground uppercase mb-1.5";

/** The one time a key's secret is shown, with how to give it to an agent. */
export function McpKeySecret({ created, onDone }) {
  const [client, setClient] = useState("claude-code");
  const [copied, setCopied] = useState("");
  const snippet = buildGuideSnippet(client, created.mcp_url, created.secret);

  async function copy(what, text) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(what);
      setTimeout(() => setCopied(""), 2000);
    } catch (_) {}
  }

  return (
    <div className="p-4 space-y-3">
      <p className="text-[0.75rem] text-foreground">
        Key <b>{created.key.name}</b> acts as <b>{created.key.person}</b>. Copy it now: it is not shown again.
      </p>
      <div>
        <label htmlFor="mcp-key-secret" className={labelCls}>Key</label>
        <div className="flex gap-1.5">
          <input id="mcp-key-secret" readOnly value={created.secret} className="mcp-token-input flex-1" />
          <Button variant={copied === "key" ? "default" : "outline"} size="sm" onClick={() => copy("key", created.secret)}>
            {copied === "key" ? "✓" : "Copy"}
          </Button>
        </div>
      </div>
      <div>
        <label htmlFor="mcp-key-url" className={labelCls}>MCP URL</label>
        <div className="flex gap-1.5">
          <input id="mcp-key-url" readOnly value={created.mcp_url} className="mcp-token-input flex-1" />
          <Button variant={copied === "url" ? "default" : "outline"} size="sm" onClick={() => copy("url", created.mcp_url)}>
            {copied === "url" ? "✓" : "Copy"}
          </Button>
        </div>
      </div>
      <div>
        <p className={labelCls}>Install guide</p>
        <div className="mcp-guide-tabs">
          {GUIDE_CLIENTS.map((item) => (
            <button key={item.id} type="button" onClick={() => setClient(item.id)} className={cx("mcp-guide-tab", client === item.id && "is-active")}>
              {item.label}
            </button>
          ))}
        </div>
        <pre className="mcp-guide-snippet">{snippet}</pre>
        <Button variant="outline" size="xs" className="mcp-guide-copy-btn" onClick={() => copy("guide", snippet)}>
          {copied === "guide" ? "✓ Copied" : "Copy"}
        </Button>
      </div>
      <Button size="sm" onClick={onDone}>Done</Button>
    </div>
  );
}
