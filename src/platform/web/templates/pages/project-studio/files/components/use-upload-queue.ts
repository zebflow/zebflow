import { useEffect, useState } from "zeb/react";
import {
  extensionForMime,
  extensionForUploadFile,
  sanitizeUploadName,
  uniqueUploadName,
} from "@/pages/project-studio/files/components/upload-names";

/**
 * Files staged for upload into the folder the browser is in: named and
 * checked here, sent only when the reader confirms.
 */
export function useUploadQueue({ api, browser }) {
  const [open, setOpen] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [items, setItems] = useState([]);
  const targetPath = browser.currentPath || "uploads";

  function stage(rawFiles: any, source = "file") {
    const picked = Array.from(rawFiles ?? []).filter(Boolean) as any[];
    if (picked.length === 0) return;
    setItems((prev) => {
      const used = new Set(
        [...browser.files.map((file) => file?.name), ...prev.map((item) => item?.name)]
          .filter(Boolean)
          .map((name) => String(name).toLowerCase()),
      );
      const now = Date.now();
      const staged = picked.map((file, index) => {
        const ext = extensionForUploadFile(file);
        const fallback = source === "paste" ? `pasted-content.${ext}` : `upload-${now}-${index}.${ext}`;
        const candidate = source === "paste" ? fallback : (file?.name || fallback);
        const name = uniqueUploadName(sanitizeUploadName(candidate, fallback), used);
        used.add(name.toLowerCase());
        return {
          id: `${now}-${index}-${Math.random().toString(36).slice(2)}`,
          file, name, originalName: file?.name || name, size: file?.size || 0, type: file?.type || "", source,
        };
      });
      return [...prev, ...staged];
    });
    setOpen(true);
    const plural = picked.length === 1 ? "" : "s";
    browser.say(`${picked.length} file${plural} ready. Review the filename${plural} before uploading.`);
  }

  function stageClipboardItems(clipItems: any) {
    const found = [];
    for (const item of Array.from(clipItems ?? []) as any[]) {
      if (item?.kind === "file") {
        const file = item.getAsFile?.();
        if (file) found.push(file);
      }
    }
    if (found.length > 0) stage(found, "paste");
  }

  function handlePaste(event: any) {
    const clipItems = event?.clipboardData?.items;
    if (!clipItems || clipItems.length === 0) return;
    event?.preventDefault?.();
    stageClipboardItems(clipItems);
  }

  useEffect(() => {
    if (!open) return;
    const listener = (event: any) => handlePaste(event);
    window.addEventListener("paste", listener);
    return () => window.removeEventListener("paste", listener);
  }, [open, browser.currentPath]);

  function handleDrop(event: any) {
    event?.preventDefault?.();
    event?.stopPropagation?.();
    setDragActive(false);
    const dropped = event?.dataTransfer?.files;
    if (dropped && dropped.length > 0) stage(dropped, "drop");
  }

  async function pasteFromClipboard() {
    if (!navigator?.clipboard?.read) {
      browser.say("Focus the files panel and paste a screenshot.");
      return;
    }
    browser.setBusy("paste");
    try {
      const found = [];
      for (const item of await navigator.clipboard.read()) {
        for (const type of item.types ?? []) {
          if (!String(type).startsWith("image/")) continue;
          const blob = await item.getType(type);
          found.push(new globalThis.File([blob], `screenshot-${Date.now()}.${extensionForMime(type)}`, { type }));
        }
      }
      if (found.length === 0) browser.say("Clipboard has no image file.");
      else stage(found, "paste");
    } catch (err) {
      const detail = err?.message || String(err);
      if (/not allowed|denied|permission/i.test(detail)) {
        browser.say("Clipboard permission is blocked here. Press Cmd+V or Ctrl+V while this dialog is open.");
      } else {
        browser.say(`Paste failed: ${detail}`, "error");
      }
    } finally {
      browser.setBusy("");
    }
  }

  async function upload() {
    if (items.length === 0 || !api.upload) return;
    const used = new Set();
    const normalized = [];
    for (const item of items) {
      const name = sanitizeUploadName(item.name, item.originalName || "upload.bin");
      if (!name || used.has(name.toLowerCase())) {
        browser.say("Upload needs unique filenames before saving.", "error");
        return;
      }
      used.add(name.toLowerCase());
      normalized.push({ ...item, name });
    }
    const plural = items.length === 1 ? "" : "s";
    browser.setBusy("upload");
    browser.say(`Uploading ${items.length} file${plural}...`);
    try {
      for (const item of normalized) {
        const form = new FormData();
        form.append("file", item.file, item.name);
        const response = await fetch(`${api.upload}?path=${encodeURIComponent(targetPath)}`, {
          method: "POST", body: form, credentials: "same-origin",
        });
        const payload = await response.json().catch(() => null);
        if (!response.ok) {
          throw new Error(payload?.error?.message || payload?.message || payload?.error || `${response.status} ${response.statusText}`);
        }
      }
      await browser.refresh(targetPath);
      browser.say(`Uploaded ${items.length} file${plural}.`, "ok");
      setItems([]);
      setOpen(false);
    } catch (err) {
      browser.say(`Upload failed: ${err?.message || String(err)}`, "error");
    } finally {
      browser.setBusy("");
    }
  }

  return {
    open, setOpen, dragActive, setDragActive, items, targetPath,
    stage, handlePaste, handleDrop, pasteFromClipboard, upload,
    rename: (id: string, name: string) => setItems((prev) => prev.map((item) => item.id === id ? { ...item, name } : item)),
    remove: (id: string) => setItems((prev) => prev.filter((item) => item.id !== id)),
    clear: () => setItems([]),
  };
}
