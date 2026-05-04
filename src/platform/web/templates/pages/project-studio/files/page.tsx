import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import { initFilesBehavior } from "@/pages/project-studio/files/files-behavior";

export const page = {
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
  initFilesBehavior();

  const activeTab = input?.active_tab ?? "files";
  const base = `/projects/${input.owner}/${input.project}/files`;
  const api = input?.api ?? {};

  const browser = input?.browser ?? { path: "", folders: [], files: [] };
  const folders = Array.isArray(browser?.folders) ? browser.folders : [];
  const files   = Array.isArray(browser?.files)   ? browser.files   : [];
  const currentPath: string = browser?.path ?? "";

  // Build breadcrumb segments from current path
  const crumbs: Array<{ label: string; path: string }> = [];
  if (currentPath) {
    const parts = currentPath.split("/");
    let acc = "";
    for (const p of parts) {
      acc = acc ? `${acc}/${p}` : p;
      crumbs.push({ label: p, path: acc });
    }
  }

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
          <StudioTabLink href={base} active={activeTab === "files"}>Browser</StudioTabLink>
          <StudioTabLink href={`${base}/s3`} active={activeTab === "s3"}>S3</StudioTabLink>
        </StudioTabNav>

        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg">
          {activeTab === "files" ? (
            <div
              className="flex flex-col flex-1 min-h-0"
              data-files-browser="true"
              data-owner={input.owner ?? ""}
              data-project={input.project ?? ""}
              data-api-list={api.list ?? ""}
              data-api-mkdir={api.mkdir ?? ""}
              data-api-upload={api.upload ?? ""}
              data-api-rm={api.rm ?? ""}
              data-current-path={currentPath}
            >
              {/* Toolbar */}
              <div className="flex items-center gap-3 px-3.5 py-2.5 border-b border-border bg-surface">
                {/* Breadcrumbs */}
                <div className="flex flex-1 min-w-0 flex-wrap items-center gap-1 text-[0.78rem]">
                  <button
                    type="button"
                    className="text-body-soft hover:text-body transition-colors bg-transparent border-0 p-0 cursor-pointer"
                    data-crumb-path=""
                  >
                    files/
                  </button>
                  {crumbs.map((crumb) => (
                    <span key={crumb.path} className="flex items-center gap-1">
                      <span className="text-border">/</span>
                      <button
                        type="button"
                        className="text-body-soft hover:text-body transition-colors bg-transparent border-0 p-0 cursor-pointer"
                        data-crumb-path={crumb.path}
                      >
                        {crumb.label}
                      </button>
                    </span>
                  ))}
                </div>
                <div className="flex items-center gap-1.5 shrink-0">
                  <input type="file" hidden data-file-upload-input="true" />
                  <Button variant="outline" size="xs" data-file-upload-trigger="true">Upload</Button>
                  <Button variant="outline" size="xs" data-new-folder-toggle="true">+ Folder</Button>
                </div>
              </div>

              {/* New folder inline form */}
              <div
                hidden
                data-new-folder-form="true"
                className="flex items-center gap-2 px-3 py-2 border-b border-border-soft flex-wrap"
              >
                <Input
                  name="folder_name"
                  type="text"
                  placeholder="folder-name"
                  className="pipeline-registry-inline-input"
                  data-new-folder-input="true"
                />
                <Button size="xs" data-new-folder-submit="true">Create Folder</Button>
                <Button variant="outline" size="xs" data-new-folder-cancel="true">Cancel</Button>
              </div>

              {/* File + folder list */}
              <div className="flex flex-col py-2 px-3 gap-0.5">
                {folders.length === 0 && files.length === 0 ? (
                  <p className="px-2 py-6 text-[0.78rem] text-body-muted">
                    {currentPath
                      ? "Empty folder"
                      : <>No files yet. Upload here or via a pipeline using <code className="font-mono text-[0.75rem]">n.file.save</code>.</>
                    }
                  </p>
                ) : null}

                {folders.map((folder) => (
                  <FolderRow key={folder.path} folder={folder} />
                ))}

                {files.map((file) => (
                  <FileRow key={file.path} file={file} />
                ))}
              </div>
            </div>
          ) : null}

          {activeTab === "s3" ? (
            <div className="project-content-wrap">
              <S3Panel />
            </div>
          ) : null}
        </section>
      </div>
    </ProjectStudioShell>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────────

function FolderRow({ folder }) {
  return (
    <div
      className="group flex items-center gap-2 min-h-[2.1rem] px-2 py-1.5 rounded-md border border-dashed border-border-soft text-body-soft text-[0.8rem] cursor-pointer hover:bg-surface-2 hover:text-body hover:border-border transition-colors"
      data-folder-path={folder.path}
    >
      <FolderIcon />
      <span className="flex-1 min-w-0 truncate font-medium text-[0.78rem] text-body">{folder.name}</span>
      {folder.protected
        ? <Badge variant="outline" className="text-[0.65rem] shrink-0">protected</Badge>
        : (
          <button
            type="button"
            className="hidden group-hover:flex items-center justify-center w-6 h-6 rounded shrink-0 text-body-muted hover:text-red-400 hover:bg-red-400/10 transition-colors"
            data-delete-btn
            data-delete-path={folder.path}
            title="Delete folder"
          >
            <TrashIcon />
          </button>
        )
      }
    </div>
  );
}

function FileRow({ file }) {
  const ext = (file.name?.split(".").pop() ?? "").toLowerCase();
  const isImage = ["jpg","jpeg","png","gif","webp","svg","avif","bmp"].includes(ext);

  return (
    <div className="group flex items-center gap-2 min-h-[2.1rem] px-2 py-1.5 rounded-md border border-border-soft bg-surface-2 hover:border-border transition-colors">
      {isImage ? <ImageFileIcon /> : <GenericFileIcon />}
      <a
        className="flex-1 min-w-0 truncate font-medium text-[0.78rem] text-body hover:text-accent hover:underline"
        href={file.url}
        target="_blank"
        rel="noopener"
      >
        {file.name}
      </a>
      <span className="text-[0.7rem] text-body-muted whitespace-nowrap shrink-0">
        {formatBytes(file.size)}
        {file.modified ? ` · ${new Date(file.modified * 1000).toLocaleDateString()}` : ""}
      </span>
      <button
        type="button"
        className="hidden group-hover:flex items-center justify-center w-6 h-6 rounded shrink-0 text-body-muted hover:text-red-400 hover:bg-red-400/10 transition-colors"
        data-delete-btn
        data-delete-path={file.path}
        title="Delete file"
      >
        <TrashIcon />
      </button>
    </div>
  );
}

function FolderIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" fillOpacity="0.15" className="w-4 h-4 shrink-0 text-amber-400" aria-hidden="true">
      <path d="M4 6h6l2 2h8v10H4z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
    </svg>
  );
}

function ImageFileIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4 shrink-0 text-sky-400" aria-hidden="true">
      <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" strokeWidth="1.6" />
      <circle cx="8.5" cy="8.5" r="1.5" stroke="currentColor" strokeWidth="1.4" />
      <path d="M21 15l-5-5L5 21" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function GenericFileIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4 shrink-0 text-body-soft" aria-hidden="true">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" />
      <path d="M14 2v6h6" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5" aria-hidden="true">
      <path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function S3Panel() {
  return (
    <div className="project-settings-panel">
      <div className="project-settings-panel-head">
        <p className="project-card-label">S3 / Object Storage</p>
        <Badge variant="outline">Coming soon</Badge>
      </div>
      <div className="project-settings-panel-body flex flex-col gap-6 pt-2">
        <Card className="opacity-60">
          <CardContent className="flex items-start gap-4 pt-5">
            <div className="mt-0.5 rounded bg-[color-mix(in_srgb,var(--color-accent)_12%,transparent)] p-2 text-accent">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 8V16a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8"/>
                <path d="M3 8l9-5 9 5"/>
                <path d="M12 3v18"/>
              </svg>
            </div>
            <div className="flex-1">
              <p className="text-[0.88rem] font-semibold text-body">Amazon S3 / S3-Compatible</p>
              <p className="mt-0.5 text-[0.78rem] text-body-soft">
                Connect an S3 bucket (AWS S3, Cloudflare R2, MinIO, Backblaze B2) as the primary
                file backend. Files stored in the bucket and served via public or pre-signed URLs.
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                {["AWS S3", "Cloudflare R2", "MinIO", "Backblaze B2", "Tigris"].map((label) => (
                  <Badge key={label} variant="secondary" className="text-[0.72rem]">{label}</Badge>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
