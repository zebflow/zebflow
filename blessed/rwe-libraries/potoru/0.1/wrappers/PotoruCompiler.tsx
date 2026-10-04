/**
 * PotoruCompiler — RWE wrapper for the Potoru authoring widget (zeb/potoru).
 *
 * IMPORT in a TSX page:
 *   import { PotoruCompiler } from "zeb/potoru";
 *
 * ─── Basic usage ──────────────────────────────────────────────────────────
 *   <PotoruCompiler id="lab" files={sourceFiles} height="560px" />
 *
 * A file list, a plain editor, diagnostics linked to file and line, a live preview player, and
 * Compile / Download .poto buttons. The compiler code loads only when this widget mounts (or on
 * the first compile() call).
 *
 * ─── Props ────────────────────────────────────────────────────────────────
 *   files      { [path]: string }   a Potoru format v4 source folder (path → YAML text)
 *   mode       "files" | "script"    "script" (EXPERIMENTAL): JavaScript using the authoring API
 *   script     string               the script, in script mode
 *   height     string | number      CSS height (default "560px")
 *   id         string               container id for window.__zebPotoruCompiler.get(id)
 *   className  string               classes on the container
 *
 * ─── Events (on the container, bubbling) ──────────────────────────────────
 *   zeb:potoru-compiler:compiled  { id, ok, poto, diagnostics, files, sizes }
 *   zeb:potoru-compiler:error     { id, ok: false, diagnostics, sizes }
 *
 * ─── Imperative registry ──────────────────────────────────────────────────
 *   const lab = window.__zebPotoruCompiler.get("lab");
 *   lab.getFiles(); lab.setFiles(files); const result = await lab.compile();
 */
export const app = {};

export default function PotoruCompiler(props) {
  const p = props || {};
  const config = { mode: p.mode === "script" ? "script" : "files" };
  if (p.files) config.files = p.files;
  if (p.script) config.script = p.script;
  if (p.height) config.height = p.height;
  const height = p.height ? (typeof p.height === "number" ? `${p.height}px` : String(p.height)) : "560px";
  return (
    <div
      data-zeb-lib="potoru"
      data-zeb-wrapper="PotoruCompiler"
      data-config={JSON.stringify(config)}
      id={p.id}
      className={p.className}
      style={{ width: "100%", display: "block", position: "relative", height }}
    />
  );
}
