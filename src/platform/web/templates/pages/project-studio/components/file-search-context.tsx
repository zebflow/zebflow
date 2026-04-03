import { createContext, useContext, useEffect, useRef, useState } from "zeb";
import FileSearchDialog from "@/pages/project-studio/components/file-search-dialog";
import type { FileItem } from "@/pages/project-studio/components/file-search-dialog";
import { registerShortcut } from "@/pages/project-studio/components/keyboard-shortcuts";

export interface FileSearchOpenOpts {
  scope?: "pages" | "all";
  onSelect?: (relPath: string) => void;
}

interface FileSearchContextValue {
  openFileSearch: (opts?: FileSearchOpenOpts) => void;
}

const FileSearchContext = createContext(null as FileSearchContextValue | null);

export function FileSearchProvider({ children, owner, project }) {
  const [open, setOpen] = useState(false);
  const [opts, setOpts] = useState<FileSearchOpenOpts>({});
  const [items, setItems] = useState<FileItem[]>([]);

  // Stable ref to the open handler so shortcuts can reference it after registration
  const handleOpenRef = useRef<(o?: FileSearchOpenOpts) => void>(null);

  async function loadItems() {
    if (!owner || !project) return;
    try {
      const [tmplResp, plResp] = await Promise.all([
        fetch(`/api/projects/${owner}/${project}/templates/workspace`),
        fetch(`/api/projects/${owner}/${project}/pipelines?recursive=true`),
      ]);
      const tmplData = tmplResp.ok ? await tmplResp.json() : {};
      const plData = plResp.ok ? await plResp.json() : {};

      const templateItems: FileItem[] = (Array.isArray(tmplData?.items) ? tmplData.items : [])
        .filter((item: any) => item?.kind !== "folder" && !!item?.rel_path)
        .map((item: any) => ({
          rel_path: String(item.rel_path),
          name: String(item.name || item.rel_path),
        }));

      const pipelineItems: FileItem[] = (Array.isArray(plData?.items) ? plData.items : [])
        .filter((item: any) => !!item?.meta?.file_rel_path)
        .map((item: any) => ({
          rel_path: String(item.meta.file_rel_path),
          name: String(item.meta.name || item.meta.file_rel_path),
        }));

      setItems([...templateItems, ...pipelineItems]);
    } catch {
      // ignore
    }
  }

  function handleOpen(o: FileSearchOpenOpts = {}) {
    setOpts(o);
    setOpen(true);
    loadItems();
  }

  // Keep ref up to date so the shortcut action always calls the latest version
  handleOpenRef.current = handleOpen;

  // Register Cmd+K / Ctrl+K — once, via stable ref
  useEffect(() => {
    registerShortcut({
      key: "k",
      meta: true,
      description: "Open file search",
      action: () => handleOpenRef.current?.(),
    });
    registerShortcut({
      key: "k",
      ctrl: true,
      description: "Open file search",
      action: () => handleOpenRef.current?.(),
    });
  }, []);

  const value: FileSearchContextValue = { openFileSearch: handleOpen };

  return (
    <FileSearchContext.Provider value={value}>
      {children}
      <FileSearchDialog
        open={open}
        onClose={() => setOpen(false)}
        onSelect={(relPath) => { opts.onSelect?.(relPath); setOpen(false); }}
        owner={owner}
        project={project}
        scope={opts.scope}
        items={items}
      />
    </FileSearchContext.Provider>
  );
}

export function useFileSearch(): FileSearchContextValue {
  const ctx = useContext(FileSearchContext);
  if (!ctx) throw new Error("useFileSearch must be used inside FileSearchProvider");
  return ctx;
}

/** Returns null when not inside FileSearchProvider — use for optional integration. */
export function useFileSearchOptional(): FileSearchContextValue | null {
  return useContext(FileSearchContext);
}
