import { cx, useEffect, useRef, useState } from "zeb";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import { LockIcon, LockOpenIcon } from "@/pages/project-studio/components/icons";

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
  const storages = Array.isArray(input?.storages) ? input.storages : [];
  const browser = input?.browser ?? { path: "", folders: [], files: [] };
  const fileInputRef = useRef(null);

  const [currentPath, setCurrentPath] = useState(browser?.path ?? "");
  const [folders, setFolders] = useState(Array.isArray(browser?.folders) ? browser.folders : []);
  const [files, setFiles] = useState(Array.isArray(browser?.files) ? browser.files : []);
  const [newFolderOpen, setNewFolderOpen] = useState(false);
  const [folderName, setFolderName] = useState("");
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");
  const [messageTone, setMessageTone] = useState("muted");
  const [pendingDelete, setPendingDelete] = useState(null);
  const [uploadDialogOpen, setUploadDialogOpen] = useState(false);
  const [uploadDragActive, setUploadDragActive] = useState(false);
  const [uploadItems, setUploadItems] = useState([]);

  const crumbs = buildCrumbs(currentPath);
  const fallbackReturnTo = `${base}/${selectedStorage}${currentPath ? `?path=${encodeURIComponent(currentPath)}` : ""}`;
  const uploadTargetPath = currentPath || "uploads";

  async function requestJson(url: string, options: any = {}) {
    const response = await fetch(url, {
      credentials: "same-origin",
      ...options,
      headers: {
        ...(options.body instanceof FormData ? {} : { "Content-Type": "application/json" }),
        ...(options.headers ?? {}),
      },
    });
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      throw new Error(
        payload?.error?.message ||
          payload?.message ||
          payload?.error ||
          `${response.status} ${response.statusText}`,
      );
    }
    return payload;
  }

  async function refresh(path = currentPath) {
    if (!api.list) return;
    const suffix = path ? `?path=${encodeURIComponent(path)}` : "";
    const payload = await requestJson(`${api.list}${suffix}`);
    setCurrentPath(payload?.path ?? path ?? "");
    setFolders(Array.isArray(payload?.folders) ? payload.folders : []);
    setFiles(Array.isArray(payload?.files) ? payload.files : []);
  }

  async function navigate(path: string) {
    setBusy("list");
    setMessage("");
    try {
      await refresh(path);
    } catch (err) {
      setMessage(`Load failed: ${err?.message || String(err)}`);
      setMessageTone("error");
    } finally {
      setBusy("");
    }
  }

  async function createFolder() {
    const name = folderName.trim();
    if (!name || !api.mkdir) return;
    const fullPath = currentPath ? `${currentPath}/${name}` : name;
    setBusy("mkdir");
    setMessage("");
    try {
      await requestJson(api.mkdir, {
        method: "POST",
        body: JSON.stringify({ path: fullPath }),
      });
      setFolderName("");
      setNewFolderOpen(false);
      await refresh(currentPath);
      setMessage("Folder created.");
      setMessageTone("ok");
    } catch (err) {
      setMessage(`Create failed: ${err?.message || String(err)}`);
      setMessageTone("error");
    } finally {
      setBusy("");
    }
  }

  function stageUploadFiles(rawFiles: any, source = "file") {
    const picked = Array.from(rawFiles ?? []).filter(Boolean) as any[];
    if (picked.length === 0) return;
    setUploadItems((prev) => {
      const used = new Set(
        [...files.map((file) => file?.name), ...prev.map((item) => item?.name)]
          .filter(Boolean)
          .map((name) => String(name).toLowerCase()),
      );
      const now = Date.now();
      const staged = picked.map((file, index) => {
        const ext = extensionForUploadFile(file);
        const fallback = source === "paste"
          ? `pasted-content.${ext}`
          : `upload-${now}-${index}.${ext}`;
        const candidate = source === "paste" ? fallback : (file?.name || fallback);
        const name = uniqueUploadName(sanitizeUploadName(candidate, fallback), used);
        used.add(name.toLowerCase());
        return {
          id: `${now}-${index}-${Math.random().toString(36).slice(2)}`,
          file,
          name,
          originalName: file?.name || name,
          size: file?.size || 0,
          type: file?.type || "",
          source,
        };
      });
      return [...prev, ...staged];
    });
    setUploadDialogOpen(true);
    setMessage(`${picked.length} file${picked.length === 1 ? "" : "s"} ready. Review the filename${picked.length === 1 ? "" : "s"} before uploading.`);
    setMessageTone("muted");
    if (fileInputRef.current) fileInputRef.current.value = "";
  }

  function updateUploadItemName(id: string, name: string) {
    setUploadItems((prev) => prev.map((item) => item.id === id ? { ...item, name } : item));
  }

  function removeUploadItem(id: string) {
    setUploadItems((prev) => prev.filter((item) => item.id !== id));
  }

  async function uploadFiles() {
    const picked = uploadItems;
    if (picked.length === 0 || !api.upload) return;
    const targetPath = currentPath || "uploads";
    const normalized = [];
    const used = new Set();
    for (const item of picked) {
      const name = sanitizeUploadName(item.name, item.originalName || "upload.bin");
      const key = name.toLowerCase();
      if (!name || used.has(key)) {
        setMessage("Upload needs unique filenames before saving.");
        setMessageTone("error");
        return;
      }
      used.add(key);
      normalized.push({ ...item, name });
    }
    setBusy("upload");
    setMessage(`Uploading ${picked.length} file${picked.length === 1 ? "" : "s"}...`);
    setMessageTone("muted");
    try {
      for (const item of normalized) {
        const form = new FormData();
        form.append("file", item.file, item.name);
        const url = `${api.upload}?path=${encodeURIComponent(targetPath)}`;
        const response = await fetch(url, {
          method: "POST",
          body: form,
          credentials: "same-origin",
        });
        const payload = await response.json().catch(() => null);
        if (!response.ok) {
          throw new Error(
            payload?.error?.message ||
              payload?.message ||
              payload?.error ||
              `${response.status} ${response.statusText}`,
          );
        }
      }
      await refresh(targetPath);
      setMessage(`Uploaded ${picked.length} file${picked.length === 1 ? "" : "s"}.`);
      setMessageTone("ok");
      setUploadItems([]);
      setUploadDialogOpen(false);
    } catch (err) {
      setMessage(`Upload failed: ${err?.message || String(err)}`);
      setMessageTone("error");
    } finally {
      setBusy("");
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  }

  async function uploadClipboardItems(items: any) {
    const files = [];
    for (const item of Array.from(items ?? []) as any[]) {
      if (item?.kind === "file") {
        const file = item.getAsFile?.();
        if (file) files.push(file);
      }
    }
    if (files.length > 0) {
      stageUploadFiles(files, "paste");
    }
  }

  async function handlePaste(event: any) {
    const items = event?.clipboardData?.items;
    if (!items || items.length === 0) return;
    event?.preventDefault?.();
    await uploadClipboardItems(items);
  }

  useEffect(() => {
    if (!uploadDialogOpen) return;
    const listener = (event: any) => {
      const items = event?.clipboardData?.items;
      if (!items || items.length === 0) return;
      event.preventDefault?.();
      uploadClipboardItems(items);
    };
    window.addEventListener("paste", listener);
    return () => window.removeEventListener("paste", listener);
  }, [uploadDialogOpen, currentPath]);

  async function handleUploadDrop(event: any) {
    event?.preventDefault?.();
    event?.stopPropagation?.();
    setUploadDragActive(false);
    const dropped = event?.dataTransfer?.files;
    if (!dropped || dropped.length === 0) return;
    stageUploadFiles(dropped, "drop");
  }

  async function pasteFromClipboard() {
    if (!navigator?.clipboard?.read) {
      setMessage("Focus the files panel and paste a screenshot.");
      setMessageTone("muted");
      return;
    }
    setBusy("paste");
    setMessage("");
    try {
      const files = [];
      const items = await navigator.clipboard.read();
      for (const item of items) {
        for (const type of item.types ?? []) {
          if (!String(type).startsWith("image/")) continue;
          const blob = await item.getType(type);
          const ext = extensionForMime(type);
          files.push(new globalThis.File([blob], `screenshot-${Date.now()}.${ext}`, { type }));
        }
      }
      if (files.length === 0) {
        setMessage("Clipboard has no image file.");
        setMessageTone("muted");
        return;
      }
      stageUploadFiles(files, "paste");
    } catch (err) {
      const detail = err?.message || String(err);
      if (/not allowed|denied|permission/i.test(detail)) {
        setMessage("Clipboard permission is blocked here. Press Cmd+V or Ctrl+V while this dialog is open.");
        setMessageTone("muted");
      } else {
        setMessage(`Paste failed: ${detail}`);
        setMessageTone("error");
      }
    } finally {
      setBusy("");
    }
  }

  async function toggleAccess(item: any, scope: "object" | "prefix") {
    if (!item?.path) return;
    if (!api.access) {
      setMessage("Access control API is not available in this build.");
      setMessageTone("error");
      return;
    }
    const nextAccess = item.public ? "private" : "public_read";
    setBusy(`access:${item.path}`);
    setMessage("");
    try {
      const payload = await requestJson(api.access, {
        method: "PUT",
        body: JSON.stringify({ path: item.path, access: nextAccess, scope }),
      });
      const nextPublic = !!payload?.public;
      const nextAccessLabel = payload?.access || nextAccess;
      if (scope === "prefix") {
        setFolders((prev) =>
          prev.map((entry) =>
            entry.path === item.path
              ? { ...entry, public: nextPublic, access: nextAccessLabel }
              : entry,
          ),
        );
      } else {
        setFiles((prev) =>
          prev.map((entry) =>
            entry.path === item.path
              ? { ...entry, public: nextPublic, access: nextAccessLabel }
              : entry,
          ),
        );
      }
      await refresh(currentPath);
      setMessage(nextAccess === "public_read" ? "Public read enabled." : "Path is private.");
      setMessageTone("ok");
    } catch (err) {
      setMessage(`Access update failed: ${err?.message || String(err)}`);
      setMessageTone("error");
    } finally {
      setBusy("");
    }
  }

  async function deletePath(item: any) {
    if (!api.rm || !item?.path) return;
    setBusy(`delete:${item.path}`);
    setMessage("");
    try {
      await requestJson(api.rm, {
        method: "POST",
        body: JSON.stringify({ path: item.path }),
      });
      await refresh(currentPath);
      setMessage("Deleted.");
      setMessageTone("ok");
    } catch (err) {
      setMessage(`Delete failed: ${err?.message || String(err)}`);
      setMessageTone("error");
    } finally {
      setBusy("");
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
          {activeTab === "explorer" ? (
            <StudioTabLink href={`${base}/${selectedStorage}`} active>Explorer</StudioTabLink>
          ) : (
            <StudioTabLink href={base} active>Storages</StudioTabLink>
          )}
        </StudioTabNav>

        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg">
          {activeTab === "storages" ? (
            <div className="project-content-wrap">
              <div className="flex flex-col gap-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <h2 className="text-[0.95rem] font-semibold text-body">Storages</h2>
                    <p className="text-[0.76rem] text-body-muted mt-1">
                      Project artifact storage. Every project starts with a default ZebFS namespace.
                    </p>
                  </div>
                </div>

                <div className="overflow-hidden rounded-md border border-border bg-surface">
                  <table className="w-full border-collapse text-[0.78rem]">
                    <thead className="bg-surface-2 text-body-muted">
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
              </div>
            </div>
          ) : null}

          {activeTab === "explorer" ? (
            <div className="flex flex-col flex-1 min-h-0" onPaste={handlePaste}>
              <div className="flex items-center gap-3 px-3.5 py-2.5 border-b border-border bg-surface">
                <div className="flex flex-1 min-w-0 flex-wrap items-center gap-1 text-[0.78rem]">
                  <a href={base} className="text-body-soft hover:text-body transition-colors">
                    storages
                  </a>
                  <span className="text-border">/</span>
                  <span className="text-body font-medium">{selectedStorage}</span>
                  <span className="text-border">/</span>
                  <button
                    type="button"
                    className="text-body-soft hover:text-body transition-colors bg-transparent border-0 p-0 cursor-pointer"
                    onClick={() => navigate("")}
                  >
                    files/
                  </button>
                  {crumbs.map((crumb) => (
                    <span key={crumb.path} className="flex items-center gap-1">
                      <span className="text-border">/</span>
                      <button
                        type="button"
                        className="text-body-soft hover:text-body transition-colors bg-transparent border-0 p-0 cursor-pointer"
                        onClick={() => navigate(crumb.path)}
                      >
                        {crumb.label}
                      </button>
                    </span>
                  ))}
                </div>
                <div className="flex items-center gap-1.5 shrink-0">
                  <input
                    ref={fileInputRef}
                    type="file"
                    multiple
                    hidden
                    onChange={(event) => stageUploadFiles(event.currentTarget.files, "file")}
                  />
                  <Button variant="outline" size="xs" onClick={() => setUploadDialogOpen(true)} disabled={busy === "upload"}>
                    Upload
                  </Button>
                  <Button variant="outline" size="xs" onClick={() => setNewFolderOpen(true)}>
                    + Folder
                  </Button>
                </div>
              </div>

              {newFolderOpen ? (
                <div className="flex items-center gap-2 px-3 py-2 border-b border-border-soft flex-wrap">
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
                  <Button size="xs" onClick={createFolder} disabled={busy === "mkdir"}>Create Folder</Button>
                  <Button variant="outline" size="xs" onClick={() => { setNewFolderOpen(false); setFolderName(""); }}>Cancel</Button>
                </div>
              ) : null}

              {message ? (
                <div className={cx(
                  "mx-3 mt-2 rounded border px-3 py-2 text-[0.76rem]",
                  messageTone === "error" && "border-red-500/40 bg-red-500/10 text-red-300",
                  messageTone === "ok" && "border-emerald-500/40 bg-emerald-500/10 text-emerald-300",
                  messageTone === "muted" && "border-border bg-surface-2 text-body-soft",
                )}>
                  {message}
                </div>
              ) : null}

              <div className="flex flex-col py-2 px-3 gap-0.5">
                {folders.length === 0 && files.length === 0 ? (
                  <p className="px-2 py-6 text-[0.78rem] text-body-muted">
                    {currentPath
                      ? "Empty folder"
                      : <>No objects yet. Upload here or via a pipeline using <code className="font-mono text-[0.75rem]">n.fs.save</code>.</>
                    }
                  </p>
                ) : null}

                {folders.map((folder) => (
                  <FolderRow
                    key={folder.path}
                    folder={folder}
                    busy={busy}
                    accessAction={api.access}
                    returnTo={fallbackReturnTo}
                    onOpen={() => navigate(folder.path)}
                    onToggleAccess={() => toggleAccess(folder, "prefix")}
                    onDelete={() => setPendingDelete({ ...folder, kind: "folder" })}
                  />
                ))}

                {files.map((file) => (
                  <FileRow
                    key={file.path}
                    file={file}
                    busy={busy}
                    accessAction={api.access}
                    returnTo={fallbackReturnTo}
                    onToggleAccess={() => toggleAccess(file, "object")}
                    onDelete={() => setPendingDelete({ ...file, kind: "file" })}
                  />
                ))}
              </div>
            </div>
          ) : null}
        </section>
      </div>

      <ConfirmDialog
        open={!!pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={() => deletePath(pendingDelete)}
        title={pendingDelete?.kind === "folder" ? "Delete Folder" : "Delete File"}
        message={pendingDelete ? `Delete "${pendingDelete.name}"? This cannot be undone.` : ""}
        confirmLabel="Delete"
        variant="destructive"
      />

      <UploadDialog
        open={uploadDialogOpen}
        onClose={() => {
          setUploadDialogOpen(false);
          setUploadDragActive(false);
        }}
        targetPath={uploadTargetPath}
        busy={busy}
        message={message}
        messageTone={messageTone}
        dragActive={uploadDragActive}
        items={uploadItems}
        onChoose={() => fileInputRef.current?.click?.()}
        onPaste={pasteFromClipboard}
        onDrop={handleUploadDrop}
        onPasteEvent={handlePaste}
        onDragActiveChange={setUploadDragActive}
        onRename={updateUploadItemName}
        onRemove={removeUploadItem}
        onClear={() => setUploadItems([])}
        onUpload={uploadFiles}
      />
    </ProjectStudioShell>
  );
}

function UploadDialog({
  open,
  onClose,
  targetPath,
  busy,
  message,
  messageTone,
  dragActive,
  items,
  onChoose,
  onPaste,
  onDrop,
  onPasteEvent,
  onDragActiveChange,
  onRename,
  onRemove,
  onClear,
  onUpload,
}) {
  const uploading = busy === "upload";
  const pasting = busy === "paste";
  const hasItems = Array.isArray(items) && items.length > 0;
  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent
        className="max-w-xl"
        style={{ overflow: "hidden", maxHeight: "calc(100dvh - 2rem)" }}
        onPaste={onPasteEvent}
      >
        <DialogHeader className="shrink-0">
          <DialogTitle>Upload Files</DialogTitle>
          <p className="text-xs text-body-soft mt-0.5">
            Destination: <span className="font-mono text-body">{targetPath}/</span>
          </p>
        </DialogHeader>

        <div className="px-6 py-5">
          <div
            className={cx(
              "flex min-h-[15rem] flex-col items-center justify-center rounded-md border border-dashed px-6 py-8 text-center transition-colors",
              dragActive
                ? "border-accent bg-accent/10 text-body"
                : "border-border bg-surface-2 text-body-soft",
            )}
            onDragEnter={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onDragActiveChange(true);
            }}
            onDragOver={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onDragActiveChange(true);
            }}
            onDragLeave={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onDragActiveChange(false);
            }}
            onDrop={onDrop}
          >
            <div className="mb-3 flex h-11 w-11 items-center justify-center rounded-md border border-border bg-surface text-body">
              <UploadIcon />
            </div>
            <p className="text-[0.9rem] font-semibold text-body">
              {uploading ? "Uploading..." : dragActive ? "Drop to upload" : "Drop files here"}
            </p>
            <p className="mt-1 max-w-sm text-[0.76rem] text-body-muted">
              Choose files, drag them into this panel, or paste an image from the clipboard. Files are not uploaded until you confirm.
            </p>
            <div className="mt-4 flex flex-wrap items-center justify-center gap-2">
              <Button size="sm" onClick={onChoose} disabled={uploading || pasting}>
                Choose Files
              </Button>
              <Button variant="outline" size="sm" onClick={onPaste} disabled={uploading || pasting}>
                {pasting ? "Reading..." : "Paste Image"}
              </Button>
            </div>
          </div>

          {hasItems ? (
            <div className="mt-4 rounded-md border border-border bg-surface">
              <div className="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
                <p className="text-[0.78rem] font-semibold text-body">
                  Pending upload
                </p>
                <Badge variant="outline" className="text-[0.68rem]">
                  {items.length} file{items.length === 1 ? "" : "s"}
                </Badge>
              </div>
              <div className="max-h-[13rem] overflow-y-auto">
                {items.map((item) => (
                  <div key={item.id} className="grid grid-cols-[1fr_auto_auto] items-center gap-2 border-b border-border-soft px-3 py-2 last:border-b-0">
                    <Input
                      value={item.name}
                      onInput={(event) => onRename(item.id, event.currentTarget.value)}
                      disabled={uploading}
                      className="min-w-0"
                    />
                    <span className="whitespace-nowrap text-[0.7rem] text-body-muted">
                      {formatBytes(item.size)}
                    </span>
                    <button
                      type="button"
                      className="flex h-7 w-7 items-center justify-center rounded text-body-muted transition-colors hover:bg-red-400/10 hover:text-red-400 disabled:opacity-50"
                      onClick={() => onRemove(item.id)}
                      disabled={uploading}
                      aria-label={`Remove ${item.name}`}
                      title="Remove"
                    >
                      <TrashIcon />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {message ? (
            <div className={cx(
              "mt-3 rounded border px-3 py-2 text-[0.76rem]",
              messageTone === "error" && "border-red-500/40 bg-red-500/10 text-red-300",
              messageTone === "ok" && "border-emerald-500/40 bg-emerald-500/10 text-emerald-300",
              messageTone === "muted" && "border-border bg-surface-2 text-body-soft",
            )}>
              {message}
            </div>
          ) : null}
        </div>

        <DialogFooter className="shrink-0">
          {hasItems ? (
            <Button variant="ghost" size="sm" onClick={onClear} disabled={uploading || pasting}>
              Clear
            </Button>
          ) : null}
          <Button variant="ghost" size="sm" onClick={onClose} disabled={uploading || pasting}>
            Close
          </Button>
          <Button size="sm" onClick={onUpload} disabled={!hasItems || uploading || pasting}>
            {uploading ? "Uploading..." : "Upload"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function StorageRow({ storage }) {
  const tags = Array.isArray(storage.tags) ? storage.tags : [];
  return (
    <tr className="border-b border-border-soft last:border-b-0">
      <td className="px-3 py-2.5 text-body font-medium">{storage.name}</td>
      <td className="px-3 py-2.5 text-body-soft">{storage.backend}</td>
      <td className="px-3 py-2.5 text-body-soft font-mono text-[0.74rem]">{storage.namespace}</td>
      <td className="px-3 py-2.5">
        <div className="flex flex-wrap gap-1">
          {tags.map((tag) => (
            <Badge key={tag} variant="outline" className="text-[0.65rem]">{tag}</Badge>
          ))}
        </div>
      </td>
      <td className="px-3 py-2.5 text-right">
        <a
          href={storage.open_href}
          className="inline-flex items-center justify-center min-h-7 px-2.5 rounded border border-border bg-surface-2 text-body hover:border-accent hover:text-accent transition-colors"
        >
          Open
        </a>
      </td>
    </tr>
  );
}

function FolderRow({ folder, busy, accessAction, returnTo, onOpen, onToggleAccess, onDelete }) {
  const isPublic = !!folder.public;
  const accessBusy = busy === `access:${folder.path}`;
  const accessTitle = isPublic ? "Make folder private" : "Make folder public";
  return (
    <div className="group flex items-center gap-2 min-h-[2.1rem] px-2 py-1.5 rounded-md border border-dashed border-border-soft text-body-soft text-[0.8rem] hover:bg-surface-2 hover:text-body hover:border-border transition-colors">
      <FolderIcon />
      <button
        type="button"
        className="flex-1 min-w-0 truncate text-left font-medium text-[0.78rem] text-body bg-transparent border-0 p-0 cursor-pointer"
        onClick={onOpen}
      >
        {folder.name}
      </button>
      <Badge variant={isPublic ? "secondary" : "outline"} className="text-[0.65rem] shrink-0">
        <AccessToggleButton
          item={folder}
          scope="prefix"
          action={accessAction}
          returnTo={returnTo}
          title={accessTitle}
          busy={accessBusy}
          className="bg-transparent border-0 p-0 m-0 text-inherit font-inherit cursor-pointer disabled:cursor-not-allowed"
          onToggleAccess={onToggleAccess}
        >
          {isPublic ? "public" : "private"}
        </AccessToggleButton>
      </Badge>
      <AccessToggleButton
        item={folder}
        scope="prefix"
        action={accessAction}
        returnTo={returnTo}
        title={accessTitle}
        busy={accessBusy}
        className={cx(
          "flex items-center justify-center w-6 h-6 rounded shrink-0 text-body-muted transition-colors hover:text-accent hover:bg-accent/10",
          accessBusy && "opacity-50 pointer-events-none",
        )}
        onToggleAccess={onToggleAccess}
      >
        {isPublic ? <LockOpenIcon className="w-3.5 h-3.5" /> : <LockIcon className="w-3.5 h-3.5" />}
      </AccessToggleButton>
      {folder.protected ? (
        <Badge variant="outline" className="text-[0.65rem] shrink-0">protected</Badge>
      ) : (
        <IconButton title="Delete folder" tone="danger" onClick={onDelete}>
          <TrashIcon />
        </IconButton>
      )}
    </div>
  );
}

function FileRow({ file, busy, accessAction, returnTo, onToggleAccess, onDelete }) {
  const ext = (file.name?.split(".").pop() ?? "").toLowerCase();
  const isImage = ["jpg", "jpeg", "png", "gif", "webp", "svg", "avif", "bmp"].includes(ext);
  const isPublic = !!file.public;
  const accessBusy = busy === `access:${file.path}`;
  const accessTitle = isPublic ? "Make file private" : "Make file public";

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
      <Badge variant={isPublic ? "secondary" : "outline"} className="text-[0.65rem] shrink-0">
        <AccessToggleButton
          item={file}
          scope="object"
          action={accessAction}
          returnTo={returnTo}
          title={accessTitle}
          busy={accessBusy}
          className="bg-transparent border-0 p-0 m-0 text-inherit font-inherit cursor-pointer disabled:cursor-not-allowed"
          onToggleAccess={onToggleAccess}
        >
          {isPublic ? "public" : "private"}
        </AccessToggleButton>
      </Badge>
      <AccessToggleButton
        item={file}
        scope="object"
        action={accessAction}
        returnTo={returnTo}
        title={accessTitle}
        busy={accessBusy}
        className={cx(
          "flex items-center justify-center w-6 h-6 rounded shrink-0 text-body-muted transition-colors hover:text-accent hover:bg-accent/10",
          accessBusy && "opacity-50 pointer-events-none",
        )}
        onToggleAccess={onToggleAccess}
      >
        {isPublic ? <LockOpenIcon className="w-3.5 h-3.5" /> : <LockIcon className="w-3.5 h-3.5" />}
      </AccessToggleButton>
      <IconButton title="Delete file" tone="danger" onClick={onDelete}>
        <TrashIcon />
      </IconButton>
    </div>
  );
}

function AccessToggleButton({
  item,
  scope,
  action,
  returnTo,
  title,
  busy,
  className,
  onToggleAccess,
  children,
}) {
  const nextAccess = item?.public ? "private" : "public_read";
  return (
    <form method="post" action={action || ""} className="contents">
      <input type="hidden" name="path" value={item?.path ?? ""} />
      <input type="hidden" name="access" value={nextAccess} />
      <input type="hidden" name="scope" value={scope} />
      <input type="hidden" name="return_to" value={returnTo ?? ""} />
      <button
        type="submit"
        className={className}
        title={title}
        aria-label={title}
        disabled={busy}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onToggleAccess?.();
        }}
      >
        {children}
      </button>
    </form>
  );
}

function IconButton({ children, title, tone = "default", onClick, disabled = false }) {
  return (
    <button
      type="button"
      className={cx(
        "flex items-center justify-center w-6 h-6 rounded shrink-0 text-body-muted transition-colors",
        tone === "danger" && "hover:text-red-400 hover:bg-red-400/10",
        tone !== "danger" && "hover:text-accent hover:bg-accent/10",
        disabled && "opacity-50 pointer-events-none",
      )}
      title={title}
      aria-label={title}
      onClick={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onClick?.();
      }}
      disabled={disabled}
    >
      {children}
    </button>
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

function UploadIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5" aria-hidden="true">
      <path d="M12 16V5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <path d="M7.5 9.5L12 5l4.5 4.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M5 16v3h14v-3" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
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

function buildCrumbs(currentPath: string): Array<{ label: string; path: string }> {
  if (!currentPath) return [];
  const out: Array<{ label: string; path: string }> = [];
  let acc = "";
  for (const part of currentPath.split("/")) {
    acc = acc ? `${acc}/${part}` : part;
    out.push({ label: part, path: acc });
  }
  return out;
}

function extensionForMime(type: string): string {
  if (type === "image/jpeg") return "jpg";
  if (type === "image/webp") return "webp";
  if (type === "image/gif") return "gif";
  return "png";
}

function extensionForUploadFile(file: any): string {
  const name = String(file?.name || "");
  const match = name.match(/\.([A-Za-z0-9]{1,12})$/);
  if (match) return match[1].toLowerCase();
  return extensionForMime(String(file?.type || ""));
}

function sanitizeUploadName(name: string, fallback: string): string {
  const cleaned = String(name || "")
    .replace(/[\\/:*?"<>|]+/g, "-")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/^\.+/, "");
  return cleaned || fallback;
}

function splitFileName(name: string): { stem: string; ext: string } {
  const match = String(name).match(/^(.*?)(\.[^.]*)?$/);
  return {
    stem: match?.[1] || "file",
    ext: match?.[2] || "",
  };
}

function uniqueUploadName(name: string, used: Set<string>): string {
  const clean = sanitizeUploadName(name, "upload.bin");
  if (!used.has(clean.toLowerCase())) return clean;
  const { stem, ext } = splitFileName(clean);
  let index = 1;
  while (true) {
    const next = `${stem}-${index}${ext}`;
    if (!used.has(next.toLowerCase())) return next;
    index += 1;
  }
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
                file backend. Files stored in the bucket and served through the Zebflow FS contract.
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
