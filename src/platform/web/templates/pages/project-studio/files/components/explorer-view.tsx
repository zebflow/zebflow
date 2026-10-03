import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { buildCrumbs } from "@/pages/project-studio/files/components/upload-names";
import { FilesMessage } from "@/pages/project-studio/files/components/files-message";
import { ExposureSummary } from "@/pages/project-studio/files/components/exposure-summary";
import { FolderRow } from "@/pages/project-studio/files/components/folder-row";
import { FileRow } from "@/pages/project-studio/files/components/file-row";
import { ownRule } from "@/pages/project-studio/files/components/exposure-mark";

/**
 * The explorer tab: where you are, what is exposed, and the folder's entries.
 * `nav` is the page's addressing (base link, storage, return link, API).
 */
export function ExplorerView({ nav, browser, queue, exposure, onDelete, onServe }) {
  const [newFolderOpen, setNewFolderOpen] = useState(false);
  const [folderName, setFolderName] = useState("");
  const crumbs = buildCrumbs(browser.currentPath);
  const ctx = { busy: browser.busy, accessAction: nav.api.access, returnTo: nav.returnTo, rules: exposure.rules };
  const crumbClass = "text-muted-foreground hover:text-foreground transition-colors bg-transparent border-0 p-0 cursor-pointer";
  const createFolder = async () => {
    await browser.createFolder(folderName);
    setFolderName("");
    setNewFolderOpen(false);
  };
  return (
    <div className="flex flex-col flex-1 min-h-0" onPaste={queue.handlePaste}>
      <div className="flex items-center gap-3 px-3.5 py-2.5 border-b border-border bg-card">
        <div className="flex flex-1 min-w-0 flex-wrap items-center gap-1 text-[0.78rem]">
          <a href={nav.base} className="text-muted-foreground hover:text-foreground transition-colors">storages</a>
          <span className="text-border">/</span>
          <span className="text-foreground font-medium">{nav.selectedStorage}</span>
          <span className="text-border">/</span>
          <button type="button" className={crumbClass} onClick={() => browser.navigate("")}>files/</button>
          {crumbs.map((crumb) => (
            <span key={crumb.path} className="flex items-center gap-1">
              <span className="text-border">/</span>
              <button type="button" className={crumbClass} onClick={() => browser.navigate(crumb.path)}>{crumb.label}</button>
            </span>
          ))}
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          <Button variant="outline" size="xs" onClick={() => queue.setOpen(true)} disabled={browser.busy === "upload"}>Upload</Button>
          <Button variant="outline" size="xs" onClick={() => setNewFolderOpen(true)}>+ Folder</Button>
        </div>
      </div>

      {newFolderOpen ? (
        <div className="flex items-center gap-2 px-3 py-2 border-b border-border flex-wrap">
          <Input
            name="folder_name"
            type="text"
            placeholder="folder-name"
            className="pipeline-registry-inline-input"
            value={folderName}
            onInput={(event) => setFolderName(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") createFolder();
              if (event.key === "Escape") setNewFolderOpen(false);
            }}
          />
          <Button size="xs" onClick={createFolder} disabled={browser.busy === "mkdir"}>Create Folder</Button>
          <Button variant="outline" size="xs" onClick={() => { setNewFolderOpen(false); setFolderName(""); }}>Cancel</Button>
        </div>
      ) : null}

      <ExposureSummary exposure={exposure} onOpen={browser.navigate} />
      <FilesMessage text={browser.message} tone={browser.messageTone} className="mx-3 mt-2" />

      <div className="flex flex-col py-2 px-3 gap-0.5">
        {browser.folders.length === 0 && browser.files.length === 0 ? (
          <p className="px-2 py-6 text-[0.78rem] text-muted-foreground">
            {browser.currentPath
              ? "Empty folder"
              : <>No objects yet. Upload here or via a pipeline using <code className="font-mono text-[0.75rem]">fs.file.put</code>.</>}
          </p>
        ) : null}
        {browser.folders.map((folder) => (
          <FolderRow
            key={folder.path}
            folder={folder}
            ctx={ctx}
            actions={{
              open: () => browser.navigate(folder.path),
              toggleAccess: () => browser.toggleAccess(folder, "prefix"),
              remove: () => onDelete({ ...folder, kind: "folder" }),
              serve: () => onServe({ ...folder, rule: ownRule(exposure.rules, folder.path) }),
            }}
          />
        ))}
        {browser.files.map((file) => (
          <FileRow
            key={file.path}
            file={file}
            ctx={ctx}
            actions={{
              toggleAccess: () => browser.toggleAccess(file, "object"),
              remove: () => onDelete({ ...file, kind: "file" }),
            }}
          />
        ))}
      </div>
    </div>
  );
}
