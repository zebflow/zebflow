/**
 * PotoEditor — RWE wrapper for the Potoru editor (zeb/potoru).
 *
 * IMPORT in a TSX page:
 *   import { PotoEditor } from "zeb/potoru";
 *
 * ─── Basic usage ──────────────────────────────────────────────────────────
 *   <PotoEditor id="lab" yaml={sourceFiles} height="560px" />
 *
 * A YAML / Script toggle, a file list, a plain editor, diagnostics linked to file and line, a live
 * preview player, and Compile / Download .poto buttons. The compiler code loads only when this
 * editor mounts (or on the first compile() call).
 *
 * ─── Props ────────────────────────────────────────────────────────────────
 *   yaml       { [path]: string }   a Potoru format v4 source folder (path → YAML text)
 *   mode       "yaml" | "script"    "script" (EXPERIMENTAL): JavaScript using the authoring API
 *                                   (TypeScript later)
 *   script     string               the script, in Script mode
 *   height     string | number      CSS height (default "560px")
 *   id         string               container id for window.__zebPotoEditor.get(id)
 *   className  string               classes on the container
 *
 * ─── Events (on the container, bubbling) ──────────────────────────────────
 *   zeb:potoru-editor:compiled  { id, ok, poto, diagnostics, files, sizes }
 *   zeb:potoru-editor:error     { id, ok: false, diagnostics, sizes }
 *
 * ─── Imperative registry ──────────────────────────────────────────────────
 *   const lab = window.__zebPotoEditor.get("lab");
 *   lab.getFiles(); lab.setFiles(files); lab.setMode("script"); const result = await lab.compile();
 */
export const app = {};

export default function PotoEditor(props) {
  const p = props || {};
  const config = { mode: p.mode === "script" ? "script" : "yaml" };
  if (p.yaml) config.yaml = p.yaml;
  if (p.script) config.script = p.script;
  if (p.height) config.height = p.height;
  const height = p.height ? (typeof p.height === "number" ? `${p.height}px` : String(p.height)) : "560px";
  return (
    <div
      data-zeb-lib="potoru"
      data-zeb-wrapper="PotoEditor"
      data-config={JSON.stringify(config)}
      id={p.id}
      className={p.className}
      style={{ width: "100%", display: "block", position: "relative", height }}
    />
  );
}
