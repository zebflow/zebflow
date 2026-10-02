import StorageBackendPanel from "@/pages/project-studio/files/components/storage-backend-panel";
import { useState } from "zeb/react";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import { StorageRow, isDefaultStorage } from "@/pages/project-studio/files/components/storage-row";
import { useFileBrowser } from "@/pages/project-studio/files/components/use-file-browser";
import { useUploadQueue } from "@/pages/project-studio/files/components/use-upload-queue";
import { useExposure } from "@/pages/project-studio/files/components/use-exposure";
import { ExplorerView } from "@/pages/project-studio/files/components/explorer-view";
import { UploadDialog } from "@/pages/project-studio/files/components/upload-dialog";
import { ServeDialog } from "@/pages/project-studio/files/components/serve-dialog";

export const page = {
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export const app = {
  hydration: "reactive",
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
  const activeTab = input?.active_tab ?? "storages";
  const selectedStorage = input?.selected_storage ?? "default";
  const base = `/projects/${input.owner}/${input.project}/files`;
  const api = input?.api ?? {};
  // The default namespace is pinned first, whatever order the server sent.
  const storages = (Array.isArray(input?.storages) ? input.storages : [])
    .slice()
    .sort((a, b) => Number(isDefaultStorage(b)) - Number(isDefaultStorage(a)));

  const exposure = useExposure(api);
  const browser = useFileBrowser({
    api,
    browser: input?.browser ?? { path: "", folders: [], files: [] },
    onChanged: exposure.reload,
  });
  const queue = useUploadQueue({ api, browser });
  const [pendingDelete, setPendingDelete] = useState(null);
  const [serveTarget, setServeTarget] = useState(null);

  const nav = {
    api,
    base,
    selectedStorage,
    returnTo: `${base}/${selectedStorage}${browser.currentPath ? `?path=${encodeURIComponent(browser.currentPath)}` : ""}`,
  };

  return (
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu="Files"
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          {activeTab === "explorer" ? (
            <StudioTabLink href={`${base}/${selectedStorage}`} active>Explorer</StudioTabLink>
          ) : (
            <StudioTabLink href={base} active>Storages</StudioTabLink>
          )}
        </StudioTabNav>

        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-background">
          {activeTab === "storages" ? (
            <div className="project-content-wrap">
              <div className="flex flex-col gap-3">
                <div>
                  <h2 className="text-[0.95rem] font-semibold text-foreground">Storages</h2>
                  <p className="text-[0.76rem] text-muted-foreground mt-1">
                    Project artifact storage. Every project starts with a default ZebFS namespace.
                  </p>
                </div>
                <div className="overflow-hidden rounded-md border border-border bg-card">
                  <table className="w-full border-collapse text-[0.78rem]">
                    <thead className="bg-muted text-muted-foreground">
                      <tr>
                        <th className="text-left font-medium px-3 py-2 border-b border-border">Name</th>
                        <th className="text-left font-medium px-3 py-2 border-b border-border">Backend</th>
                        <th className="text-left font-medium px-3 py-2 border-b border-border">Namespace</th>
                        <th className="text-left font-medium px-3 py-2 border-b border-border">Tags</th>
                        <th className="text-right font-medium px-3 py-2 border-b border-border">Action</th>
                      </tr>
                    </thead>
                    <tbody>
                      {storages.map((storage) => (
                        <StorageRow key={storage.name} storage={storage} />
                      ))}
                    </tbody>
                  </table>
                </div>
                {/* The backend behind those namespaces. It used to be shown in
                    Settings, where you could read it but do nothing with it. */}
                <StorageBackendPanel storage={input?.storage} />
              </div>
            </div>
          ) : null}

          {activeTab === "explorer" ? (
            <ExplorerView
              nav={nav}
              browser={browser}
              queue={queue}
              exposure={exposure}
              onDelete={setPendingDelete}
              onServe={setServeTarget}
            />
          ) : null}
        </section>
      </div>

      <ConfirmDialog
        open={!!pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={() => browser.deletePath(pendingDelete)}
        title={pendingDelete?.kind === "folder" ? "Delete Folder" : "Delete File"}
        message={pendingDelete ? `Delete "${pendingDelete.name}"? This cannot be undone.` : ""}
        confirmLabel="Delete"
        variant="destructive"
      />
      <UploadDialog queue={queue} browser={browser} />
      <ServeDialog target={serveTarget} browser={browser} onClose={() => setServeTarget(null)} />
    </ProjectStudioShell>
  );
}
