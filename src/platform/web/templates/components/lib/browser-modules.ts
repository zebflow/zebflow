/**
 * Browser modules in the file tree: what a row is to its project's browser
 * module folders (the `module` field the repository listing carries), and the
 * one action the tree offers on them — making a file an entry or not.
 *
 * The folders and their entries live in `zebflow.yaml` (`rwe.browser_modules`),
 * which only Zebflow writes, so the action goes through the project's RWE
 * settings: read the section, change the one folder, write it back.
 */

import { requestJson } from "@/components/lib/http";

/** How a row looks: its background tint, a short mark, and the hover text. */
export function moduleLook(item: any): { tint: string; mark: string; hint: string } | null {
  const module = item?.module;
  if (!module) return null;
  if (module.role === "folder") {
    return {
      tint: "bg-primary/20",
      mark: "module",
      hint: `Browser module folder. Pages may import its entries: ${(module.entries || []).join(", ")}`,
    };
  }
  if (module.role === "entry") {
    return {
      tint: "bg-primary/25",
      mark: module.lazy ? "entry · lazy" : "entry",
      hint: module.lazy
        ? "Entry, loaded on demand: built with everything it imports into one script, fetched when another file asks for it."
        : "Entry: built with everything it imports into one script. Pages can import it.",
    };
  }
  const into = (module.bundled_into || []).map((path: string) => path.split("/").pop()).join(", ");
  return {
    tint: "bg-primary/15",
    mark: "",
    hint: into ? `Internal: bundled into ${into}` : "Inside a browser module folder",
  };
}

/** The path of `item` inside its module folder: `objects/ship.ts`. */
function insideFolder(item: any): string {
  const folder = String(item.module.folder || "");
  const rel = String(item.rel_path || "");
  if (rel.startsWith(`${folder}/`)) return rel.slice(folder.length + 1);
  const at = rel.indexOf(`/${folder}/`);
  return at === -1 ? rel : rel.slice(at + folder.length + 2);
}

/** What the tree may offer on `item`: make it an entry, stop it being one, or nothing. */
export function entryAction(item: any): { label: string; make: boolean; disabled?: boolean } | null {
  const module = item?.module;
  if (!module || item.kind !== "file") return null;
  if (module.role === "internal") {
    return /\.(ts|mts|js|mjs)$/.test(String(item.rel_path || "")) ? { label: "Make browser module entry", make: true } : null;
  }
  if (module.role === "entry") return { label: "Remove browser module entry", make: false };
  return null;
}

/**
 * Makes `item` an entry of its folder (`make`), or stops it being one, and
 * answers why not when it cannot. An entry named by a pattern (`islands/*.ts`)
 * is changed in the pattern, not file by file.
 */
export async function setEntry(owner: string, project: string, item: any, make: boolean): Promise<string | null> {
  const base = `/api/projects/${owner}/${project}/settings/rwe`;
  const section = await requestJson(base);
  const rwe = section?.data || {};
  const folders = (rwe.browser_modules || []).map((folder: any) => ({ path: folder.path, entries: [...(folder.entries || [])] }));
  const folder = folders.find((entry: any) => entry.path === item.module.folder);
  if (!folder) return "This folder is no longer a browser module folder.";
  const inside = insideFolder(item);
  if (make) {
    if (!folder.entries.includes(inside)) folder.entries.push(inside);
  } else {
    if (!folder.entries.includes(inside)) {
      return `This file is an entry through a pattern (${folder.entries.join(", ")}); change the pattern in the project's RWE settings.`;
    }
    if (folder.entries.length === 1) return "A browser module folder needs at least one entry.";
    folder.entries = folder.entries.filter((entry: string) => entry !== inside);
  }
  await requestJson(base, {
    method: "PUT",
    body: JSON.stringify({
      commit_message: `${make ? "Make" : "Stop"} ${inside} ${make ? "an entry" : "being an entry"} of ${folder.path}`,
      data: {
        allow_list: rwe.allow_list || [],
        minify_html: !!rwe.minify_html,
        strict_mode: rwe.strict_mode !== false,
        deployment_asset_base: rwe.deployment_asset_base || null,
        browser_modules: folders,
      },
    }),
  });
  return null;
}
