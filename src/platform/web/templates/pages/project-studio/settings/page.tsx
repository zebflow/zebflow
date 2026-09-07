import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import ProjectTransferPanel from "@/pages/project-studio/settings/components/project-transfer-panel";
import DangerZone from "@/pages/project-studio/settings/components/danger-zone";
import GitBranchPanel from "@/pages/project-studio/settings/components/git-branch-panel";
import AssistantPanel from "@/pages/project-studio/settings/components/assistant-panel";
import ProjectConfigurationStatus from "@/pages/project-studio/settings/components/project-configuration-status";
import ReIndexPanel from "@/pages/project-studio/settings/components/reindex-panel";
import ProfilePanel from "@/pages/project-studio/settings/components/profile-panel";
import DistributionPanel from "@/pages/project-studio/settings/components/distribution-panel";
import RuntimeDefaultsPanel from "@/pages/project-studio/settings/components/runtime-defaults-panel";
import RwePanel from "@/pages/project-studio/settings/components/rwe-panel";
import GitPanel from "@/pages/project-studio/settings/components/git-panel";
import LoggingPanel from "@/pages/project-studio/settings/components/logging-panel";

export const page = {
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

// ─── Shared helpers ────────────────────────────────────────────────────────

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
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          <StudioTabNav>
            {settingsTabs.map((item, index) => (
              <StudioTabLink
                key={`${item?.href ?? "tab"}-${index}`}
                href={item?.href ?? "#"}
                active={item?.classes === "is-active"}
              >
                {item?.label}
              </StudioTabLink>
            ))}
          </StudioTabNav>

          <section className="flex-1 min-h-0 overflow-auto flex flex-col">
            <div className="flex min-h-full flex-col">
              <section
                className="border-b border-dark-border px-4 py-3"
              >
                <div>
                  <p className="text-[0.68rem] font-medium uppercase tracking-[0.08em] text-body-soft">
                    {input.page_title}
                  </p>
                  <p className="mt-1 text-[0.78rem] text-body-soft">
                    {input.page_subtitle}
                  </p>
                </div>
              </section>

              {tabFlags?.general ? (
                <section className="flex flex-col">
                  <div className="flex flex-col">
                    <ProjectConfigurationStatus config={input?.project_configuration ?? {}} />
                    <ProfilePanel api={input?.profile?.api ?? ""} initialConfig={input?.profile?.config ?? {}} />
                    <DistributionPanel api={input?.distribution?.api ?? ""} initialConfig={input?.distribution?.config ?? {}} />
                    <ProjectTransferPanel
                      owner={input?.owner}
                      project={input?.project}
                      api={input?.transfer?.api ?? {}}
                      initialOperations={input?.transfer?.operations ?? []}
                    />
                    <RuntimeDefaultsPanel
                      api={input?.assets?.settings_api ?? ""}
                      initialConfig={input?.assets?.config ?? {}}
                    />
                    <ReIndexPanel api={input?.reindex_api ?? ""} />
                    <DangerZone owner={input.owner} project={input.project} />

                    {/* Signposts to configuration that lives with its feature.
                        People look in Settings first; saying where a thing went
                        costs one card and saves the hunt. */}
                    <div className="border-b border-dark-border px-4 py-4" data-settings-signposts="true">
                      <p className="mb-3 text-[0.68rem] font-medium uppercase tracking-[0.08em] text-body-soft">
                        Configured elsewhere
                      </p>
                      <div className="project-card-grid cols-2">
                        {renderCardGrid(input?.cards_general)}
                      </div>
                    </div>
                  </div>
                </section>
              ) : null}

              {/* Git is its own subject, not a footnote to General. The `GIT`
                  button in the studio chrome links straight here. */}
              {tabFlags?.git ? (
                <section className="flex flex-col">
                  <div className="flex flex-col">
                    <GitPanel
                      remoteApi={input?.git?.remote_api ?? ""}
                      initialConfig={input?.git?.config ?? {}}
                      healthApi={input?.git?.health_api ?? ""}
                      repairApi={input?.git?.repair_api ?? ""}
                    />
                    <GitBranchPanel owner={input.owner} project={input.project} />
                  </div>
                </section>
              ) : null}

              {tabFlags?.policy ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <RwePanel
                      api={input?.rwe?.api ?? ""}
                      initialConfig={input?.rwe?.config ?? {}}
                      owner={input?.owner}
                      project={input?.project}
                    />
                    <div className="project-card-grid cols-2">
                      {renderCardGrid(input?.cards_policy)}
                    </div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.logs ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <LoggingPanel
                      api={input?.logging?.api ?? ""}
                      invocationsApi={input?.logging?.invocations_api ?? ""}
                      initialConfig={input?.logging?.config ?? {}}
                    />
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

                    <article className="border border-border rounded-lg bg-surface p-[0.85rem] mb-[0.9rem]">
                      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
                        <div>
                          <h3 className="project-card-title">MCP Session</h3>
                          <p className="project-card-copy">Remote control channel for external agents.</p>
                        </div>
                        <span className="project-inline-chip">{input?.mcp?.status_label}</span>
                      </header>
                      <div className="flex flex-wrap items-center gap-[0.45rem]">
                        <p className="project-card-copy">Allowed capabilities:</p>
                        {mcpCapabilities.map((item, index) => (
                          <span key={`${item}-${index}`} className="project-inline-chip">{item}</span>
                        ))}
                      </div>
                    </article>
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
