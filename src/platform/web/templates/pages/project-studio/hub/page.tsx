import ProjectStudioShell from "@/pages/project-studio/components/shell";
import Button from "@/components/ui/button";
import { useState } from "zeb/react";
import HubBrowser from "@/components/hub/hub-browser";
import NodeRegistryPanel from "@/pages/project-studio/hub/components/node-registry-panel";
import DependenciesPanel from "@/pages/project-studio/hub/components/dependencies-panel";
import HubTabBar from "@/pages/project-studio/hub/components/hub-tab-bar";
import BrowseConfirmDialogs from "@/pages/project-studio/hub/components/browse-confirm-dialogs";
import InstallFromFilePanel from "@/pages/project-studio/hub/components/install-from-file-panel";
import UploadLibraryDialog from "@/pages/project-studio/hub/components/upload-library-dialog";
import HubSourcesPanel from "@/pages/project-studio/hub/components/hub-sources-panel";
import MyPackagesTable from "@/pages/project-studio/hub/components/my-packages-table";
import PublishPanel from "@/pages/project-studio/hub/components/publish-panel";
import { useStatus } from "@/pages/project-studio/hub/components/use-status";
import { useHubData } from "@/pages/project-studio/hub/components/use-hub-data";
import { useBrowseActions } from "@/pages/project-studio/hub/components/use-browse-actions";
import { usePublishForm } from "@/pages/project-studio/hub/components/use-publish-form";

export const page = {
  head: {
    links: [{ rel: "stylesheet", href: "/assets/platform/db-suite.css" }],
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
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

export default function Page(input) {
  const tabs = Array.isArray(input?.hub_tabs) ? input.hub_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const api = input?.hub_api ?? {};
  const [fromFileOpen, setFromFileOpen] = useState(false);
  const [uploadOpen, setUploadOpen] = useState(false);
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const { status, showStatus } = useStatus();
  const { browseItems, myPacks, hubSources, refresh } = useHubData(input, api);
  const browse = useBrowseActions({ api, refresh, showStatus });
  // Always called, whatever the tab: My Packages reads its publisher token.
  const publish = usePublishForm({ input, api, refresh, showStatus });

  return (
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu="Hub"
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          <HubTabBar tabs={tabs} />

          <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-background">
            <div className="project-content-wrap">
              <section className="project-content-section">
                <div className="project-content-head">
                  <div>
                    <p className="project-content-title">Project Hub</p>
                    <p className="project-content-copy">Browse packages and add them to this project.</p>
                  </div>
                  <div className="flex items-center gap-2">
                    {/* The rare paths, behind buttons: as permanent blocks they
                        pushed the catalogue below the fold. */}
                    <Button type="button" variant="outline" size="sm" onClick={() => setSourcesOpen(true)}>
                      Sources
                    </Button>
                    <Button type="button" variant="outline" size="sm" onClick={() => setFromFileOpen(true)}>
                      Install from file
                    </Button>
                    <Button type="button" variant="outline" size="sm" onClick={() => setUploadOpen(true)}>
                      Upload library
                    </Button>
                    <Button type="button" variant="outline" size="sm" onClick={() => refresh().then(() => showStatus("Refreshed")).catch((err) => showStatus(err?.message || err))}>
                      Refresh
                    </Button>
                  </div>
                </div>
              </section>

              <section className="project-content-section">
                <div className="project-content-body space-y-4">
                  {status ? (
                    <div className="rounded-lg border border-border bg-accent/30 px-4 py-3 text-sm text-muted-foreground">
                      <span className="font-medium text-foreground">Status:</span> {status}
                    </div>
                  ) : null}

                  <BrowseConfirmDialogs browse={browse} />

                  <UploadLibraryDialog
                    open={uploadOpen}
                    onClose={() => setUploadOpen(false)}
                    api={api}
                    onUploaded={() => refresh().catch(() => {})}
                  />

                  <InstallFromFilePanel
                    open={fromFileOpen}
                    onClose={() => setFromFileOpen(false)}
                    api={api}
                    showStatus={showStatus}
                  />

                  {tabFlags?.packs ? (
                    <>
                      <HubSourcesPanel
                        open={sourcesOpen}
                        onClose={() => setSourcesOpen(false)}
                        api={api}
                        hubSources={hubSources}
                        refresh={refresh}
                        showStatus={showStatus}
                      />

                      <div className="flex flex-col rounded-lg border border-border bg-popover">
                        <HubBrowser
                          items={browseItems}
                          owner={input?.owner}
                          project={input?.project}
                          initialState="available"
                          destination={browse.destination}
                          onDestinationChange={browse.setDestination}
                          busy={browse.busy}
                          onAct={browse.onAct}
                        />
                      </div>
                    </>
                  ) : null}

                  {tabFlags?.my_packs ? (
                    <MyPackagesTable
                      myPacks={myPacks}
                      api={api}
                      publisherToken={publish.form.publisher_token}
                      refresh={refresh}
                      showStatus={showStatus}
                    />
                  ) : null}

                  {tabFlags?.publish ? <PublishPanel publish={publish} /> : null}

                  {/* What this project has installed. These sit with the Hub
                      because the Hub is where installing happens — the
                      inventory used to live in Settings while the install
                      button lived here. */}
                  {tabFlags?.nodes ? (
                    <NodeRegistryPanel
                      groups={input?.installed?.node_groups ?? []}
                      count={input?.installed?.node_count ?? 0}
                    />
                  ) : null}

                  {tabFlags?.dependencies ? (
                    <DependenciesPanel
                      api={input?.installed?.dependencies?.api ?? ""}
                      initialStatus={input?.installed?.dependencies?.status ?? {}}
                    />
                  ) : null}

                </div>
              </section>
            </div>
          </section>
        </div>
      </ProjectStudioShell>
  );
}
