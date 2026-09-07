import { useEffect, useState } from "zeb/react";
import Button from "@/components/ui/button";

// ── Asset Manager ─────────────────────────────────────────────────────────────

function formatAssetBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export default function AssetManager({ api, subfolder = "" }: { api: string; subfolder?: string }) {
  const listUrl = subfolder ? `${api}?subfolder=${encodeURIComponent(subfolder)}` : api;
  const uploadUrl = subfolder ? `${api}?subfolder=${encodeURIComponent(subfolder)}` : api;
  const deleteBase = subfolder ? `${api}/${encodeURIComponent(subfolder)}` : api;

  const [files, setFiles] = useState([] as any[]);
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [errorMsg, setErrorMsg] = useState(null as string | null);
  const [copied, setCopied] = useState(null as string | null);
  const [pendingDelete, setPendingDelete] = useState(null as string | null);

  async function apiJson(url, options: any = {}) {
    const res = await fetch(url, {
      headers: { Accept: "application/json", ...(options.body ? { "Content-Type": "application/json" } : {}) },
      ...options,
    });
    if (res.status === 401) { window.location.href = "/login"; return null; }
    const payload = await res.json().catch(() => null);
    if (!res.ok) throw new Error(payload?.error || `${res.status} ${res.statusText}`);
    return payload;
  }

  async function loadFiles() {
    setLoading(true);
    setErrorMsg(null);
    try {
      const resp = await apiJson(listUrl);
      setFiles(Array.isArray(resp?.files) ? resp.files : []);
    } catch (err: any) {
      setErrorMsg(String(err?.message || err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { loadFiles(); }, []);

  async function processFiles(fileList) {
    if (!fileList || fileList.length === 0) return;
    setUploading(true);
    setErrorMsg(null);
    const errors: string[] = [];
    for (let i = 0; i < fileList.length; i++) {
      const file = fileList[i];
      const fd = new FormData();
      fd.append("file", file);
      try {
        const res = await fetch(uploadUrl, { method: "POST", body: fd });
        const payload = await res.json().catch(() => null);
        if (!res.ok) throw new Error(payload?.error || `${res.status}`);
      } catch (err: any) {
        errors.push(`${file.name}: ${err?.message || String(err)}`);
      }
    }
    if (errors.length > 0) setErrorMsg(errors.join(" | "));
    await loadFiles();
    setUploading(false);
  }


  async function handleDelete(name: string) {
    setErrorMsg(null);
    try {
      await apiJson(`${deleteBase}/${encodeURIComponent(name)}`, { method: "DELETE" });
      setFiles((prev) => prev.filter((f) => f.name !== name));
    } catch (err: any) {
      setErrorMsg(String(err?.message || err));
    }
  }

  function handleCopyUrl(url: string) {
    const abs = `${window.location.protocol}//${window.location.host}${url}`;
    navigator.clipboard.writeText(abs).catch(() => {});
    setCopied(url);
    setTimeout(() => setCopied(null), 2000);
  }

  const totalSize = files.reduce((sum, f) => sum + (f.size_bytes ?? 0), 0);
  const totalSizeStr = formatAssetBytes(totalSize);
  const fileCountLabel = files.length !== 1 ? "s" : "";

  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-auto">
      <div className="pipeline-editor-toolbar">
        <div className="pipeline-editor-toolbar-main">
          <p className="pipeline-editor-title">assets/</p>
          <p className="pipeline-editor-subtitle">
            {loading ? "Loading…" : `${files.length} file${fileCountLabel} · ${totalSizeStr}`}
          </p>
        </div>
        <div className="pipeline-editor-toolbar-actions">
          <Button as="label" variant="primary" size="xs" className="cursor-pointer" disabled={uploading}>
            {uploading ? "Uploading…" : "Upload"}
            <input
              type="file"
              multiple
              className="sr-only"
              onChange={(e) => processFiles((e.target as HTMLInputElement).files)}
            />
          </Button>
        </div>
      </div>

      {errorMsg ? (
        <p className="px-3 py-2 text-[0.72rem] text-red-300">{errorMsg}</p>
      ) : null}

      {!loading && files.length === 0 ? (
        <div className="flex flex-col items-center justify-center flex-1 gap-2 text-body-soft">
          <p className="text-[0.82rem]">No assets yet.</p>
          <p className="text-[0.75rem]">Click <strong>Upload</strong> to add files.</p>
        </div>
      ) : (
        <div className="px-3 py-3">
          <table className="w-full text-[0.78rem]">
            <thead>
              <tr className="text-left text-body-soft text-[0.68rem] uppercase tracking-wide border-b border-border">
                <th className="pb-[0.4rem] font-medium">Name</th>
                <th className="pb-[0.4rem] font-medium text-right">Size</th>
                <th className="pb-[0.4rem] font-medium text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {files.map((file) => {
                const fileSizeStr = formatAssetBytes(file.size_bytes);
                return (
                <tr key={file.name} className="border-b border-border-soft hover:bg-surface-2 transition-colors">
                  <td className="py-[0.45rem] font-mono text-[0.74rem] text-body truncate max-w-[22rem]">{file.name}</td>
                  <td className="py-[0.45rem] text-right text-body-soft tabular-nums">{fileSizeStr}</td>
                  <td className="py-[0.45rem] text-right">
                    <div className="flex items-center justify-end gap-1">
                      <Button variant="ghost" size="xs" onClick={() => handleCopyUrl(file.url)}>
                        {copied === file.url ? "Copied!" : "Copy URL"}
                      </Button>
                      <Button variant="ghost" size="xs" className="text-red-400" onClick={() => setPendingDelete(file.name)}>
                        Delete
                      </Button>
                    </div>
                  </td>
                </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    <ConfirmDialog
      open={pendingDelete !== null}
      onClose={() => setPendingDelete(null)}
      onConfirm={() => { if (pendingDelete) handleDelete(pendingDelete); }}
      title="Delete asset"
      message={pendingDelete ? `Delete "${pendingDelete}"? This cannot be undone.` : ""}
      confirmLabel="Delete"
      variant="destructive"
    />
    </div>
  );
}

// Sub-component: safe to call useFileSearch() here because it renders inside FileSearchProvider
// (as part of ProjectStudioShell's children tree, after the provider is established).
