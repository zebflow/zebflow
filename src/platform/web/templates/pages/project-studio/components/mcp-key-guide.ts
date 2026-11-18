/**
 * What the MCP key panel knows without asking the server: the install guide
 * per agent, and the rights an MCP tool can use, with their labels.
 */

/** The agents the install guide covers, in the order its tabs show. */
export const GUIDE_CLIENTS = [
  { id: "claude-code", label: "Claude Code" },
  { id: "cursor", label: "Cursor" },
  { id: "codex", label: "Codex" },
  { id: "gemini", label: "Gemini" },
  { id: "opencode", label: "OpenCode" },
  { id: "cline", label: "Cline" },
];

/** How one agent is told about this project's MCP and a key's secret. */
export function buildGuideSnippet(clientId: string, mcpUrl: string, secret: string): string {
  const url = mcpUrl || "<MCP_URL>";
  const key = secret || "<KEY>";
  const jsonBlock = `{\n  "mcpServers": {\n    "zebflow": {\n      "url": "${url}",\n      "headers": {\n        "Authorization": "Bearer ${key}"\n      }\n    }\n  }\n}`;
  switch (clientId) {
    case "claude-code":
      return `claude mcp add --transport http zebflow \\\n  ${url} \\\n  --header "Authorization: Bearer ${key}"`;
    case "cursor":
      return `// ~/.cursor/mcp.json\n${jsonBlock}`;
    case "codex":
      return `// ~/.codex/config.json\n${jsonBlock}`;
    case "gemini":
      return `// ~/.gemini/settings.json\n${jsonBlock}`;
    case "opencode":
      return `# ~/.config/opencode/config.toml\n[mcp.zebflow]\ntype   = "remote"\nurl    = "${url}"\n\n[mcp.zebflow.headers]\nAuthorization = "Bearer ${key}"`;
    case "cline":
      return `// Cline → MCP Servers → Add Server (HTTP)\n// or in VS Code settings.json:\n${JSON.stringify({ "cline.mcpServers": { zebflow: { url, headers: { Authorization: `Bearer ${key}` } } } }, null, 2)}`;
    default:
      return jsonBlock;
  }
}

/** Every right some MCP tool asks for, in the order the key form lists them. */
export const MCP_RIGHTS = [
  { key: "project.read", label: "Read the project" },
  { key: "pipelines.read", label: "Read pipelines" },
  { key: "pipelines.write", label: "Write pipelines" },
  { key: "pipelines.delete", label: "Delete pipelines" },
  { key: "pipelines.execute", label: "Run pipelines" },
  { key: "templates.read", label: "Read templates" },
  { key: "templates.write", label: "Write templates" },
  { key: "templates.create", label: "Create templates" },
  { key: "templates.delete", label: "Delete templates" },
  { key: "files.write", label: "Write files" },
  { key: "tables.read", label: "Read data" },
  { key: "tables.write", label: "Write data" },
  { key: "settings.read", label: "Read settings" },
  { key: "settings.write", label: "Write settings" },
  { key: "credentials.read", label: "Read credentials" },
  { key: "credentials.write", label: "Write credentials" },
];

/** What a new key starts with: every MCP right the person holds, credentials left out. */
export function startingRights(held: string[]): string[] {
  return MCP_RIGHTS.map((right) => right.key).filter(
    (key) => held.includes(key) && !key.startsWith("credentials."),
  );
}
