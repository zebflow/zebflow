/**
 * The repository tree, fetched one folder at a time and remembered.
 *
 * A sidebar asks for a folder's children when the reader opens it, and the
 * answer is kept until something changes inside that folder. Adding a file then
 * costs one request for its parent — not another walk of the whole repository,
 * which is what the single unscoped `/repo` call used to cost every screen that
 * drew a tree.
 *
 * The cache is module-scope on purpose: the RWE compiler inlines a module into
 * each page's bundle, so one page gets one cache and closing the page forgets
 * it. Nothing here is shared between pages.
 */

import { requestJson } from "@/components/lib/http";

/** `pipelines/api/orders.zf.json` → `pipelines/api`; a root file → `""`. */
export function parentFolderOf(relPath: string): string {
  const clean = String(relPath || "").replace(/^\/+|\/+$/g, "");
  const cut = clean.lastIndexOf("/");
  return cut === -1 ? "" : clean.slice(0, cut);
}

/** Every folder from the root down to `relPath`, nearest last. */
export function ancestorsOf(relPath: string): string[] {
  const parts = String(relPath || "").replace(/^\/+|\/+$/g, "").split("/");
  const out: string[] = [];
  for (let i = 1; i < parts.length; i += 1) {
    out.push(parts.slice(0, i).join("/"));
  }
  return out;
}

type Cache = {
  folders: Map<string, any[]>;
  pending: Map<string, Promise<any[]>>;
  paths: string[] | null;
  git: Record<string, string> | null;
};

const CACHES = new Map<string, Cache>();

function cacheFor(key: string): Cache {
  let cache = CACHES.get(key);
  if (!cache) {
    cache = { folders: new Map(), pending: new Map(), paths: null, git: null };
    CACHES.set(key, cache);
  }
  return cache;
}

/**
 * A reader over one project's repository tree.
 *
 * `children` is the only way in, and it answers from memory when it can. Two
 * components opening the same folder in the same tick share one request rather
 * than racing.
 */
export function repoTree(owner: string, project: string) {
  const base = `/api/projects/${owner}/${project}/repo`;
  const cache = cacheFor(`${owner}/${project}`);

  async function children(path = ""): Promise<any[]> {
    const key = String(path || "");
    const known = cache.folders.get(key);
    if (known) return known;

    const inflight = cache.pending.get(key);
    if (inflight) return inflight;

    const request = requestJson(
      `${base}?depth=1${key ? `&path=${encodeURIComponent(key)}` : ""}`,
    )
      .then((payload) => {
        const items = Array.isArray(payload?.items) ? payload.items : [];
        cache.folders.set(key, items);
        return items;
      })
      .finally(() => {
        cache.pending.delete(key);
      });

    cache.pending.set(key, request);
    return request;
  }

  /** File paths only, for matching against. Read once per page. */
  async function paths(): Promise<string[]> {
    if (cache.paths) return cache.paths;
    const payload = await requestJson(`${base}?fields=path`);
    cache.paths = Array.isArray(payload?.paths) ? payload.paths : [];
    return cache.paths;
  }

  /**
   * Which files git considers changed, keyed by path.
   *
   * Asked for the whole repository at once because that is the only shape the
   * question has — git does not answer it per folder, and pretending otherwise
   * would mean one `git status` per expansion.
   */
  async function gitStatus(): Promise<Record<string, string>> {
    if (cache.git) return cache.git;
    const items = await requestJson(
      `/api/projects/${owner}/${project}/templates/git-status`,
    );
    const map: Record<string, string> = {};
    for (const item of Array.isArray(items) ? items : []) {
      if (item?.rel_path) map[String(item.rel_path)] = String(item.code || "");
    }
    cache.git = map;
    return map;
  }

  /** Forget one folder, so the next open re-reads it. */
  function invalidate(path = "") {
    cache.folders.delete(String(path || ""));
    cache.paths = null;
    cache.git = null;
  }

  /**
   * Forget the folder a path lives in.
   *
   * This is what a create, a delete or a move calls: the change is inside one
   * folder, so that folder is the only thing that has gone stale. A move is two
   * calls, one per end.
   */
  function invalidateParentOf(relPath: string) {
    invalidate(parentFolderOf(relPath));
  }

  /** Everything, for the rare case that the whole repository moved under us. */
  function invalidateAll() {
    cache.folders.clear();
    cache.paths = null;
    cache.git = null;
  }

  /** What is already known about a folder, without asking for it. */
  function known(path = ""): any[] | null {
    return cache.folders.get(String(path || "")) ?? null;
  }

  return { children, paths, gitStatus, invalidate, invalidateParentOf, invalidateAll, known };
}
