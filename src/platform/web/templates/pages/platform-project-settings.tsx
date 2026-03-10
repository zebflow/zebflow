import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initProjectSettingsBehavior } from "@/components/behavior/project-settings";
import Input from "@/components/ui/input";
import Badge from "@/components/ui/badge";
import Separator from "@/components/ui/separator";
import TabsList from "@/components/ui/tabs-list";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardDescription from "@/components/ui/card-description";
import { cx, Link } from "rwe";

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

export default function Page(input) {
  initProjectSettingsBehavior();
  const tabFlags = input?.tab_flags ?? {};
  const settingsTabs = Array.isArray(input?.settings_tabs) ? input.settings_tabs : [];
  const assistant = input?.assistant ?? {};
  const assistantCredentials = Array.isArray(assistant?.credentials) ? assistant.credentials : [];
  const assistantConfig = assistant?.config ?? {};
  const mcpCapabilities = Array.isArray(input?.mcp?.capabilities) ? input.mcp.capabilities : [];
  const nodeGroups = Array.isArray(input?.node_groups) ? input.node_groups : [];
  const nodeCount = input?.node_count ?? 0;

  return (
<Page>
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
            <Link key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} className={cx("project-tab-link", item?.classes)}>{item?.label}</Link>
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

                  <article className="project-settings-panel">
                    <header className="project-settings-panel-head">
                      <div>
                        <h3 className="project-card-title">Reactive Web Engine</h3>
                        <p className="project-card-copy">No configuration available yet.</p>
                      </div>
                      <span className="project-inline-chip">RWE</span>
                    </header>
                  </article>

                  <div className="project-card-grid cols-2">
                    {renderCardGrid(input?.cards_policy)}
                  </div>
                </div>
              </section>
            ) : null}

            {tabFlags?.automatons ? (
              <section className="project-content-section">
                <div className="project-content-body">
                  <article className="project-settings-panel" data-assistant-settings="true" data-api-config={assistant?.api?.config ?? ""}>
                    <header className="project-settings-panel-head">
                      <div>
                        <h3 className="project-card-title">Project Assistant</h3>
                        <p className="project-card-copy">Bind credential profiles for assistant reasoning tiers.</p>
                      </div>
                      <span className="project-inline-chip">Automaton</span>
                    </header>

                    <form className="project-settings-form" data-assistant-settings-form="true">
                      <label className="pipeline-editor-field">
                        <span>High Model</span>
                        <select name="llm_high_credential_id" data-assistant-high="true" defaultValue={assistantConfig?.llm_high_credential_id ?? ""}>
                          <option value="">None</option>
                          {assistantCredentials.map((item, index) => (
                            <option key={`${item?.credential_id ?? "credential"}-${index}`} value={item?.credential_id ?? ""}>
                              {item?.title} · {item?.credential_id}
                            </option>
                          ))}
                        </select>
                        <small className="pipeline-editor-field-help">Planning and decomposition model.</small>
                      </label>

                      <label className="pipeline-editor-field">
                        <span>General Model</span>
                        <select name="llm_general_credential_id" data-assistant-general="true" defaultValue={assistantConfig?.llm_general_credential_id ?? ""}>
                          <option value="">None</option>
                          {assistantCredentials.map((item, index) => (
                            <option key={`${item?.credential_id ?? "credential-general"}-${index}`} value={item?.credential_id ?? ""}>
                              {item?.title} · {item?.credential_id}
                            </option>
                          ))}
                        </select>
                        <small className="pipeline-editor-field-help">Default model for regular project chat requests.</small>
                      </label>

                      <label className="pipeline-editor-field">
                        <span>Max Steps</span>
                        <input name="max_steps" type="number" min="1" max="1000" defaultValue={assistantConfig?.max_steps ?? 50} />
                        <small className="pipeline-editor-field-help">Upper bound for future multi-step agent execution.</small>
                      </label>

                      <label className="pipeline-editor-field">
                        <span>Max Replans</span>
                        <input name="max_replans" type="number" min="0" max="64" defaultValue={assistantConfig?.max_replans ?? 2} />
                        <small className="pipeline-editor-field-help">Maximum replanning attempts before stopping.</small>
                      </label>

                      <label className="pipeline-editor-field">
                        <span>Chat History Pairs</span>
                        <input name="chat_history_pairs" type="number" min="0" max="50" defaultValue={assistantConfig?.chat_history_pairs ?? 10} />
                        <small className="pipeline-editor-field-help">Number of previous user/assistant exchanges kept as context (0 = no history).</small>
                      </label>

                      <label className="project-settings-checkbox">
                        <input name="enabled" type="checkbox" value="true" defaultChecked={Boolean(assistantConfig?.enabled)} />
                        <span>Enable assistant for this project</span>
                      </label>

                      <div className="project-settings-actions">
                        <button type="submit" className="project-inline-chip project-inline-chip-accent" data-assistant-save="true">Save Assistant Config</button>
                        <span className="project-settings-status" data-assistant-status="true">Ready.</span>
                      </div>
                    </form>
                  </article>

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
                  <div className="node-registry-shell" data-node-registry="true">

                    {/* Toolbar: search + install */}
                    <div className="node-registry-toolbar">
                      <Input placeholder="Search nodes by name or kind..." data-node-search="true" />
                      <Button variant="outline" size="sm" label="+ Install" data-node-install-btn="true" />
                    </div>

                    {/* Tab bar */}
                    <div className="node-registry-tabs">
                      <TabsList>
                        <Button variant="ghost" size="sm" data-node-tab-btn="installed" className="node-tab-active"
                                label={`Installed · ${nodeCount}`} />
                        <Button variant="ghost" size="sm" data-node-tab-btn="discover" label="Discover" />
                        <Button variant="ghost" size="sm" data-node-tab-btn="updates" label="Updates" />
                      </TabsList>
                    </div>

                    <Separator />

                    {/* Installed panel */}
                    <div data-node-tab-panel="installed">
                      <p className="node-registry-summary" data-node-summary="true">
                        {nodeCount} nodes · {nodeCount} built-in
                      </p>
                      <div className="node-registry-list">
                        {nodeGroups.length === 0 ? (
                          <p className="node-registry-empty">No nodes installed.</p>
                        ) : nodeGroups.map((group, gi) => (
                          <div key={`grp-${gi}`} data-node-group={group?.prefix ?? ""}>
                            <div className="node-registry-group-head">
                              {group?.prefix ? <span className="node-registry-group-label">{group.prefix}</span> : null}
                              <div className="node-registry-group-rule" />
                            </div>
                            {(Array.isArray(group?.nodes) ? group.nodes : []).map((node, ni) => (
                              <div
                                key={`${node?.kind ?? "node"}-${ni}`}
                                className="node-registry-item"
                                data-node-item="true"
                                data-search-text={`${node?.title ?? ""} ${node?.kind ?? ""} ${node?.description ?? ""}`.toLowerCase()}
                              >
                                <div className="node-registry-item-stripe" />
                                <div className="node-registry-item-body">
                                  <div className="node-registry-item-main">
                                    <div className="node-registry-item-title">{node?.title}</div>
                                    <div className="node-registry-item-kind">{node?.kind}</div>
                                    <div className="node-registry-item-desc">{node?.description}</div>
                                  </div>
                                  <div className="node-registry-item-caps">
                                    {node?.script_available ? <Badge label="n.script access" variant="outline" className="node-badge-cap" /> : null}
                                    {node?.ai_registered ? <Badge label="agent tool" variant="outline" className="node-badge-cap" /> : null}
                                    <span className="project-inline-chip node-badge-installed">● installed</span>
                                  </div>
                                </div>
                              </div>
                            ))}
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Discover panel */}
                    <div data-node-tab-panel="discover" hidden={true}>
                      <div className="node-registry-empty-panel">
                        <Card>
                          <CardContent>
                            <CardDescription label="Community registry — coming soon." className="node-registry-empty-text" />
                            <CardDescription label="Install custom nodes from a Git URL using + Install above." className="node-registry-empty-sub" />
                          </CardContent>
                        </Card>
                      </div>
                    </div>

                    {/* Updates panel */}
                    <div data-node-tab-panel="updates" hidden={true}>
                      <div className="node-registry-empty-panel">
                        <Card>
                          <CardContent>
                            <CardDescription label={`All ${nodeCount} built-in nodes are current.`} className="node-registry-empty-text" />
                          </CardContent>
                        </Card>
                      </div>
                    </div>

                  </div>
                </div>
              </section>
            ) : null}
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}
