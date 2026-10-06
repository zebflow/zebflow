import { h, useState } from "zeb/react";

/**
 * The libraries a Potoru story block hands PotoPlayer (`libraries`: `.potolib`
 * addresses), and the list the editor edits them with.
 *
 * A library address is an `https:` URL or a path on the project's own site
 * (`/_files/libs/basic.potolib`); anything else — `http:`, `javascript:`,
 * `//host/…`, a bare relative path, whitespace or quotes — is refused.
 *
 * The page's defaults (`potoruExtension({ libraries })`) merge with a block's
 * own list. Two entries are the same library when their file names match
 * (`…/basic.potolib`): the block's entry wins, and the default is left out.
 */

/** The address, trimmed, if a block may pass it to PotoPlayer; otherwise null. */
export function potoruLibraryUrl(url) {
  const value = typeof url === "string" ? url.trim() : "";
  if (!value || value.length > 2048 || /[\s"'<>\\`]/.test(value)) return null;
  if (/^https:\/\/[^/?#@]+/i.test(value)) return value;
  if (value.startsWith("/") && !value.startsWith("//")) return value;
  return null;
}

/** A library's file name: what makes two addresses the same library. */
export function potoruLibraryName(url) {
  const path = String(url).split(/[?#]/)[0];
  return path.slice(path.lastIndexOf("/") + 1) || path;
}

function distinct(list) {
  const seen = new Set();
  const out = [];
  for (const raw of Array.isArray(list) ? list : []) {
    const url = potoruLibraryUrl(raw);
    if (!url || seen.has(potoruLibraryName(url))) continue;
    seen.add(potoruLibraryName(url));
    out.push(url);
  }
  return out;
}

/** The page's defaults, then the block's own; the block wins a file name both have. */
export function mergePotoruLibraries(defaults, own) {
  const mine = distinct(own);
  const taken = new Set(mine.map(potoruLibraryName));
  return [...distinct(defaults).filter((url) => !taken.has(potoruLibraryName(url))), ...mine];
}

const CHIP = "inline-flex max-w-[16rem] items-center gap-1 truncate rounded border border-border px-1.5 py-0.5";

/**
 * The block's library list in the editor: the page's defaults (read-only,
 * struck through when the block replaces one), the block's own with a remove
 * button each, and an address field that refuses what PotoPlayer may not load.
 */
export function PotoruLibraries({ defaults, libraries, readOnly, onChange }) {
  const [draft, setDraft] = useState("");
  const [error, setError] = useState("");
  const own = distinct(libraries);
  const replaced = new Set(own.map(potoruLibraryName));
  function add() {
    const url = potoruLibraryUrl(draft);
    if (!url) { setError("An https:// address, or a path on this site starting with /"); return; }
    setDraft("");
    setError("");
    onChange([...own.filter((item) => potoruLibraryName(item) !== potoruLibraryName(url)), url]);
  }
  return h("div", { "data-slot": "potoru-libraries", className: "mt-2 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground" },
    h("span", { className: "font-medium" }, "Libraries"),
    ...distinct(defaults).map((url) => h("span", {
      key: `d:${url}`, "data-library-default": url, title: `${url} (set by the page)`,
      className: `${CHIP} ${replaced.has(potoruLibraryName(url)) ? "line-through opacity-60" : ""}`,
    }, potoruLibraryName(url))),
    ...own.map((url) => h("span", { key: `o:${url}`, "data-library": url, title: url, className: `${CHIP} bg-accent text-accent-foreground` },
      potoruLibraryName(url),
      readOnly ? null : h("button", {
        type: "button", "aria-label": `Remove ${potoruLibraryName(url)}`, className: "leading-none hover:text-destructive",
        onClick: () => onChange(own.filter((item) => item !== url)),
      }, "×"))),
    readOnly ? null : h("span", { className: "inline-flex items-center gap-1" },
      h("input", {
        name: "potoru-library", value: draft, placeholder: "https://… or /….potolib", "aria-label": "Library address",
        className: "h-6 w-56 rounded border border-input bg-background px-1.5 text-xs text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
        onInput: (event) => { setDraft(event.target.value); setError(""); },
        onKeyDown: (event) => { if (event.key === "Enter") { event.preventDefault(); add(); } },
      }),
      h("button", { type: "button", className: "h-6 rounded bg-secondary px-2 text-secondary-foreground hover:bg-secondary/80", onClick: add }, "Add library")),
    error ? h("span", { role: "alert", className: "w-full text-destructive" }, error) : null,
  );
}

export default PotoruLibraries;
