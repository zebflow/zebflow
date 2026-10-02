import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";

/**
 * The explorer's folder: where the reader is, what is in it, and the verbs on
 * one entry. `onChanged` is told after anything that may change exposure, so
 * the page can re-read the totals.
 */
export function useFileBrowser({ api, browser, onChanged }) {
  const [currentPath, setCurrentPath] = useState(browser?.path ?? "");
  const [folders, setFolders] = useState(Array.isArray(browser?.folders) ? browser.folders : []);
  const [files, setFiles] = useState(Array.isArray(browser?.files) ? browser.files : []);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");
  const [messageTone, setMessageTone] = useState("muted");

  function say(text: string, tone = "muted") {
    setMessage(text);
    setMessageTone(tone);
  }

  async function refresh(path = currentPath) {
    if (!api.list) return;
    const suffix = path ? `?path=${encodeURIComponent(path)}` : "";
    const payload = await requestJson(`${api.list}${suffix}`);
    setCurrentPath(payload?.path ?? path ?? "");
    setFolders(Array.isArray(payload?.folders) ? payload.folders : []);
    setFiles(Array.isArray(payload?.files) ? payload.files : []);
  }

  async function run(label: string, failure: string, work: () => Promise<string | void>) {
    setBusy(label);
    setMessage("");
    try {
      const done = await work();
      if (done) say(done, "ok");
    } catch (err) {
      say(`${failure}: ${err?.message || String(err)}`, "error");
    } finally {
      setBusy("");
    }
  }

  const navigate = (path: string) => run("list", "Load failed", () => refresh(path));

  const createFolder = (name: string) =>
    run("mkdir", "Create failed", async () => {
      const clean = name.trim();
      if (!clean || !api.mkdir) return;
      await requestJson(api.mkdir, {
        method: "POST",
        body: JSON.stringify({ path: currentPath ? `${currentPath}/${clean}` : clean }),
      });
      await refresh(currentPath);
      return "Folder created.";
    });

  const setAccess = (item: any, scope: string, access: string, serve: string[] = []) =>
    run(`access:${item?.path}`, "Access update failed", async () => {
      if (!item?.path || !api.access) return;
      await requestJson(api.access, {
        method: "PUT",
        body: JSON.stringify({ path: item.path, access, scope, serve }),
      });
      await refresh(currentPath);
      onChanged?.();
      if (access === "public_execute") return "Runs as a site on the listed addresses.";
      return access === "public_read" ? "Public read enabled." : "Path is private.";
    });

  const toggleAccess = (item: any, scope: string) =>
    setAccess(item, scope, item?.access && item.access !== "private" ? "private" : "public_read");

  const deletePath = (item: any) =>
    run(`delete:${item?.path}`, "Delete failed", async () => {
      if (!api.rm || !item?.path) return;
      await requestJson(api.rm, { method: "POST", body: JSON.stringify({ path: item.path }) });
      await refresh(currentPath);
      onChanged?.();
      return "Deleted.";
    });

  return {
    currentPath, folders, files, busy, setBusy, message, messageTone, say,
    refresh, navigate, createFolder, setAccess, toggleAccess, deletePath,
  };
}
