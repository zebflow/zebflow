import { useState, cx, Link } from "rwe";
import ProjectStudioShell from "@/components/layout/project-studio-shell";
import Checkbox from "@/components/ui/checkbox";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import Badge from "@/components/ui/badge";
import Separator from "@/components/ui/separator";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardDescription from "@/components/ui/card-description";
import CommitDialog from "@/components/ui/commit-dialog";
import { Select, SelectOption } from "@/components/ui/select";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

// ─── Shared helpers ────────────────────────────────────────────────────────

async function requestJson(url, options = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (response) => {
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const message =
        payload?.error?.message ||
        payload?.message ||
        payload?.error ||
        `${response.status} ${response.statusText}`;
      throw new Error(message);
    }
    return payload;
  });
}

function renderCardGrid(items) {
  const rows = Array.isArray(items) ? items : [];
  return rows.map((item, index) => (
    <a key={`${item?.href ?? "item"}-${index}`} href={item?.href ?? "#"} className="project-card block">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="project-card-title">{item?.title}</h3>
          <p className="project-card-copy">{item?.description}</p>
        </div>
        {item?.tag ? <span className="project-inline-chip">{item.tag}</span> : null}
      </div>
    </a>
  ));
}

// ─── RWE Settings Panel ─────────────────────────────────────────────────────

function RwePanel({ api, initialConfig }) {
  const [allowList, setAllowList] = useState(
    (initialConfig?.allow_list ?? []).join("\n")
  );
  const [minify, setMinify] = useState(Boolean(initialConfig?.minify_html));
  const [strict, setStrict] = useState(initialConfig?.strict_mode !== false);
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);

  function handleSubmit(e) {
    e.preventDefault();
    setPendingData({
      allow_list: allowList.split(/[\n,]/).map((s) => s.trim()).filter(Boolean),
      minify_html: minify,
      strict_mode: strict,
    });
    setCommitOpen(true);
  }

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const resp = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({ commit_message: commitMessage, data: pendingData }),
      });
      if (resp?.committed) {
        setStatusMsg("Saved & committed.");
        setStatusTone("ok");
      } else if (resp?.git_error) {
        setStatusMsg(`Saved (git: ${resp.git_error})`);
        setStatusTone("info");
      } else {
        setStatusMsg("Saved.");
        setStatusTone("ok");
      }
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
      setPendingData(null);
    }
  }

  return (
    <article className="project-settings-panel">
      <CommitDialog
        open={commitOpen}
        section="rwe"
        defaultMessage="settings(rwe): update RWE config"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <header className="project-settings-panel-head">
        <div>
          <h3 className="project-card-title">Reactive Web Engine</h3>
          <p className="project-card-copy">
            Project-level compile and render controls for all <code>n.web.render</code> nodes.
          </p>
        </div>
        <span className="project-inline-chip">RWE</span>
      </header>

      <form className="project-settings-form" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>Script Allow List</span>
          <Textarea
            name="allow_list"
            rows={4}
            placeholder={"https://cdnjs.cloudflare.com/*\nhttps://cdn.jsdelivr.net/*"}
            value={allowList}
            onInput={(e) => setAllowList(e.currentTarget.value)}
          />
          <small className="pipeline-editor-field-help">
            One URL pattern per line (or comma-separated). Controls which external scripts and
            stylesheets <code>--load-scripts</code> may reference. Blessed <code>zeb/*</code>{" "}
            libraries are always allowed and do not appear here.
          </small>
        </label>

        <div className="flex flex-col gap-2 pt-1">
          <Checkbox
            name="minify_html"
            label="Minify HTML output"
            checked={minify}
            onChange={(e) => setMinify(e.target.checked)}
          />
          <Checkbox
            name="strict_mode"
            label="Strict compile-time checks"
            checked={strict}
            onChange={(e) => setStrict(e.target.checked)}
          />
        </div>

        <div className="project-settings-actions">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save RWE Config"}
          />
          <span className="project-settings-status" data-tone={statusTone}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}

// ─── Assistant Settings Panel ───────────────────────────────────────────────

function AssistantPanel({ api, credentials, initialConfig }) {
  const creds = Array.isArray(credentials) ? credentials : [];
  const [highModel, setHighModel] = useState(
    String(initialConfig?.llm_high_credential_id || "")
  );
  const [generalModel, setGeneralModel] = useState(
    String(initialConfig?.llm_general_credential_id || "")
  );
  const [maxSteps, setMaxSteps] = useState(Number(initialConfig?.max_steps ?? 50));
  const [maxReplans, setMaxReplans] = useState(Number(initialConfig?.max_replans ?? 2));
  const [historyPairs, setHistoryPairs] = useState(
    Number(initialConfig?.chat_history_pairs ?? 10)
  );
  const [enabled, setEnabled] = useState(Boolean(initialConfig?.enabled));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);

  async function handleSubmit(e) {
    e.preventDefault();
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({
          llm_high_credential_id: highModel.trim() || null,
          llm_general_credential_id: generalModel.trim() || null,
          max_steps: maxSteps,
          max_replans: maxReplans,
          chat_history_pairs: historyPairs,
          enabled,
        }),
      });
      setStatusMsg("Saved.");
      setStatusTone("ok");
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
    }
  }

  return (
    <article className="project-settings-panel">
      <header className="project-settings-panel-head">
        <div>
          <h3 className="project-card-title">Project Assistant</h3>
          <p className="project-card-copy">Bind credential profiles for assistant reasoning tiers.</p>
        </div>
        <span className="project-inline-chip">Automaton</span>
      </header>

      <form className="project-settings-form" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>High Model</span>
          <Select name="llm_high_credential_id" value={highModel} onChange={(e) => setHighModel(e.target.value)}>
            <SelectOption value="" label="None" />
            {creds.map((item, index) => (
              <SelectOption
                key={`${item?.credential_id ?? "credential"}-${index}`}
                value={item?.credential_id ?? ""}
                label={`${item?.title} · ${item?.credential_id}`}
              />
            ))}
          </Select>
          <small className="pipeline-editor-field-help">Planning and decomposition model.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>General Model</span>
          <Select name="llm_general_credential_id" value={generalModel} onChange={(e) => setGeneralModel(e.target.value)}>
            <SelectOption value="" label="None" />
            {creds.map((item, index) => (
              <SelectOption
                key={`${item?.credential_id ?? "credential-general"}-${index}`}
                value={item?.credential_id ?? ""}
                label={`${item?.title} · ${item?.credential_id}`}
              />
            ))}
          </Select>
          <small className="pipeline-editor-field-help">Default model for regular project chat requests.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Max Steps</span>
          <Input
            type="number"
            name="max_steps"
            min={1}
            max={1000}
            value={maxSteps}
            onChange={(e) => setMaxSteps(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">Upper bound for future multi-step agent execution.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Max Replans</span>
          <Input
            type="number"
            name="max_replans"
            min={0}
            max={64}
            value={maxReplans}
            onChange={(e) => setMaxReplans(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">Maximum replanning attempts before stopping.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Chat History Pairs</span>
          <Input
            type="number"
            name="chat_history_pairs"
            min={0}
            max={50}
            value={historyPairs}
            onChange={(e) => setHistoryPairs(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">
            Number of previous user/assistant exchanges kept as context (0 = no history).
          </small>
        </label>

        <div className="flex flex-col gap-2 pt-1">
          <Checkbox
            name="enabled"
            label="Enable assistant for this project"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
          />
        </div>

        <div className="project-settings-actions">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Assistant Config"}
          />
          <span className="project-settings-status" data-tone={statusTone}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}

// ─── Node Registry Panel ─────────────────────────────────────────────────────

function NodeRegistryPanel({ groups, count }) {
  const [searchQuery, setSearchQuery] = useState("");
  const [activeTab, setActiveTab] = useState("installed");

  const nodeGroups = Array.isArray(groups) ? groups : [];
  const query = searchQuery.toLowerCase().trim();

  const filteredGroups = query
    ? nodeGroups
        .map((group) => ({
          ...group,
          nodes: (Array.isArray(group?.nodes) ? group.nodes : []).filter((node) =>
            `${node?.title ?? ""} ${node?.kind ?? ""} ${node?.description ?? ""}`
              .toLowerCase()
              .includes(query)
          ),
        }))
        .filter((group) => group.nodes.length > 0)
    : nodeGroups;

  const visibleCount = filteredGroups.reduce(
    (sum, g) => sum + (g.nodes?.length ?? 0),
    0
  );

  return (
    <div className="node-registry-shell">
      {/* Toolbar */}
      <div className="node-registry-toolbar">
        <Input
          placeholder="Search nodes by name or kind..."
          value={searchQuery}
          onInput={(e) => setSearchQuery(e.currentTarget.value)}
        />
        <Button variant="outline" size="sm" label="+ Install" disabled />
      </div>

      {/* Tabs */}
      <div className="node-registry-tabs flex gap-1">
        {(["installed", "discover", "updates"] as const).map((tab) => (
          <Button
            key={tab}
            variant="ghost"
            size="sm"
            className={cx(tab === activeTab && "node-tab-active")}
            label={tab === "installed" ? `Installed · ${count}` : tab === "discover" ? "Discover" : "Updates"}
            onClick={() => setActiveTab(tab)}
          />
        ))}
      </div>

      <Separator />

      {/* Installed panel */}
      {activeTab === "installed" ? (
        <div>
          <p className="node-registry-summary">
            {visibleCount === count
              ? `${count} nodes · ${count} built-in`
              : `${visibleCount} of ${count} nodes · ${count} built-in`}
          </p>
          <div className="node-registry-list">
            {filteredGroups.length === 0 ? (
              <p className="node-registry-empty">No nodes found.</p>
            ) : (
              filteredGroups.map((group, gi) => (
                <div key={`grp-${gi}`}>
                  <div className="node-registry-group-head">
                    {group?.prefix ? (
                      <span className="node-registry-group-label">{group.prefix}</span>
                    ) : null}
                    <div className="node-registry-group-rule" />
                  </div>
                  {(Array.isArray(group?.nodes) ? group.nodes : []).map((node, ni) => (
                    <div
                      key={`${node?.kind ?? "node"}-${ni}`}
                      className="node-registry-item"
                    >
                      <div className="node-registry-item-stripe" />
                      <div className="node-registry-item-body">
                        <div className="node-registry-item-main">
                          <div className="node-registry-item-title">{node?.title}</div>
                          <div className="node-registry-item-kind">{node?.kind}</div>
                          <div className="node-registry-item-desc">{node?.description}</div>
                        </div>
                        <div className="node-registry-item-caps">
                          {node?.script_available ? (
                            <Badge label="n.script access" variant="outline" className="node-badge-cap" />
                          ) : null}
                          {node?.ai_registered ? (
                            <Badge label="agent tool" variant="outline" className="node-badge-cap" />
                          ) : null}
                          <span className="project-inline-chip node-badge-installed">● installed</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ))
            )}
          </div>
        </div>
      ) : null}

      {/* Discover panel */}
      {activeTab === "discover" ? (
        <div className="node-registry-empty-panel">
          <Card>
            <CardContent>
              <CardDescription label="Community registry — coming soon." className="node-registry-empty-text" />
              <CardDescription label="Install custom nodes from a Git URL using + Install above." className="node-registry-empty-sub" />
            </CardContent>
          </Card>
        </div>
      ) : null}

      {/* Updates panel */}
      {activeTab === "updates" ? (
        <div className="node-registry-empty-panel">
          <Card>
            <CardContent>
              <CardDescription label={`All ${count} built-in nodes are current.`} className="node-registry-empty-text" />
            </CardContent>
          </Card>
        </div>
      ) : null}
    </div>
  );
}

// ─── Page ────────────────────────────────────────────────────────────────────

export default function Page(input) {
  const tabFlags = input?.tab_flags ?? {};
  const settingsTabs = Array.isArray(input?.settings_tabs) ? input.settings_tabs : [];
  const assistant = input?.assistant ?? {};
  const mcpCapabilities = Array.isArray(input?.mcp?.capabilities) ? input.mcp.capabilities : [];

  return (
    <>
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu="Settings"
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
        <div className="project-workspace">
          <nav className="project-tab-strip">
            {settingsTabs.map((item, index) => (
              <Link
                key={`${item?.href ?? "tab"}-${index}`}
                href={item?.href ?? "#"}
                className={cx("project-tab-link", item?.classes)}
              >
                {item?.label}
              </Link>
            ))}
          </nav>

          <section className="project-workspace-body">
            <div className="project-content-wrap">
              <section className="project-content-section">
                <div className="project-content-head">
                  <div>
                    <p className="project-content-title">{input.page_title}</p>
                    <p className="project-content-copy">{input.page_subtitle}</p>
                  </div>
                </div>
              </section>

              {tabFlags?.general ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <div className="project-card-grid cols-2">{renderCardGrid(input?.cards_general)}</div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.policy ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <RwePanel
                      api={input?.rwe?.api ?? ""}
                      initialConfig={input?.rwe?.config ?? {}}
                    />
                    <div className="project-card-grid cols-2">
                      {renderCardGrid(input?.cards_policy)}
                    </div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.automatons ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <AssistantPanel
                      api={assistant?.api?.config ?? ""}
                      credentials={Array.isArray(assistant?.credentials) ? assistant.credentials : []}
                      initialConfig={assistant?.config ?? {}}
                    />

                    <article className="project-settings-panel">
                      <header className="project-settings-panel-head">
                        <div>
                          <h3 className="project-card-title">MCP Session</h3>
                          <p className="project-card-copy">Remote control channel for external agents.</p>
                        </div>
                        <span className="project-inline-chip">{input?.mcp?.status_label}</span>
                      </header>
                      <div className="project-settings-inline-list">
                        <p className="project-card-copy">Allowed capabilities:</p>
                        {mcpCapabilities.map((item, index) => (
                          <span key={`${item}-${index}`} className="project-inline-chip">{item}</span>
                        ))}
                      </div>
                    </article>
                  </div>
                </section>
              ) : null}

              {tabFlags?.libraries ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <div className="project-card-grid cols-2">{renderCardGrid(input?.cards_libraries)}</div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.nodes ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <NodeRegistryPanel
                      groups={input?.node_groups ?? []}
                      count={input?.node_count ?? 0}
                    />
                  </div>
                </section>
              ) : null}
            </div>
          </section>
        </div>
      </ProjectStudioShell>
    </>
  );
}
