/**
 * zeb/potoru — the PotoruCompiler authoring widget (part of the compiler chunks).
 *
 * A file list, a plain textarea editor (0.1; `zeb/codemirror` is the planned upgrade), a
 * diagnostics list linked to file and line, a live preview player and Compile / Download buttons.
 * Everything lives in the placeholder's shadow root, so a VDOM re-render never touches it.
 */
import { compile } from "./compiler.mjs";

const STYLE = `
:host{display:block;position:relative;font:13px/1.4 system-ui,sans-serif;color:#1f2430;}
*{box-sizing:border-box}
.root{display:grid;grid-template-columns:minmax(120px,180px) minmax(0,1fr) minmax(0,1fr);grid-template-rows:auto minmax(0,1fr) auto;height:100%;border:1px solid #d6d9e0;border-radius:8px;overflow:hidden;background:#fff}
.bar{grid-column:1/-1;display:flex;gap:8px;align-items:center;padding:6px 8px;border-bottom:1px solid #e3e5ea;background:#f6f7f9}
.bar .grow{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#596070}
button{font:inherit;padding:4px 10px;border:1px solid #c5c9d3;border-radius:6px;background:#fff;cursor:pointer}
button:hover{background:#eef0f4}
button.primary{background:#2f5bd3;border-color:#2f5bd3;color:#fff}
button:disabled{opacity:.5;cursor:default}
.files{overflow:auto;border-right:1px solid #e3e5ea;padding:4px 0;margin:0;list-style:none}
.files button{display:block;width:100%;text-align:left;border:0;border-radius:0;padding:3px 8px;background:none;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.files button[aria-current="true"]{background:#e6ecfb;font-weight:600}
.files button[data-problem]{color:#b42318}
textarea{width:100%;height:100%;border:0;resize:none;padding:8px;font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;tab-size:2;outline:none}
textarea[readonly]{background:#f6f7f9;color:#596070}
.side{display:grid;grid-template-rows:auto minmax(0,1fr);border-left:1px solid #e3e5ea;min-height:0}
.preview{background:#111;min-height:120px}
.diagnostics{overflow:auto;margin:0;padding:4px 0;list-style:none;border-top:1px solid #e3e5ea}
.diagnostics button{display:block;width:100%;text-align:left;border:0;border-radius:0;background:none;padding:3px 8px}
.diagnostics .error{color:#b42318}.diagnostics .warning{color:#9a6700}
.diagnostics .fix{display:block;color:#596070;font-size:12px}
.empty{padding:8px;color:#596070}
.foot{grid-column:1/-1;padding:4px 8px;border-top:1px solid #e3e5ea;color:#596070;font-size:12px;background:#f6f7f9}
`;

const SCRIPT_FILE = "script.js";

function el(tag, attributes = {}, text) {
  const node = document.createElement(tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
  if (text !== undefined) node.textContent = text;
  return node;
}

export async function createCompilerWidget(host, config, { id, emit }) {
  const root = host.shadowRoot ?? host.attachShadow({ mode: "open" });
  root.textContent = "";
  const style = el("style");
  style.textContent = STYLE;
  const frame = el("div", { class: "root", part: "compiler" });
  const bar = el("div", { class: "bar", role: "toolbar", "aria-label": "Potoru compiler" });
  const compileButton = el("button", { type: "button", class: "primary" }, "Compile");
  const downloadButton = el("button", { type: "button", disabled: "" }, "Download .poto");
  const status = el("span", { class: "grow", role: "status" }, "");
  bar.append(compileButton, downloadButton, status);
  const fileList = el("ul", { class: "files", "aria-label": "Files" });
  const editor = el("textarea", { spellcheck: "false", "aria-label": "File contents", wrap: "off" });
  const side = el("div", { class: "side" });
  const preview = el("div", { class: "preview" });
  const diagnosticList = el("ul", { class: "diagnostics", "aria-label": "Diagnostics" });
  side.append(preview, diagnosticList);
  const foot = el("div", { class: "foot" }, config.mode === "script" ? "Script mode (experimental): JavaScript using the authoring API runs in a sandbox with no network; it returns a Project." : "Files mode: a Potoru format v4 source folder.");
  frame.append(bar, fileList, editor, side, foot);
  root.append(style, frame);

  let mode = config.mode === "script" ? "script" : "files";
  let files = { ...(config.files ?? {}) };
  let script = typeof config.script === "string" ? config.script : "";
  let selected = "";
  let lastResult;
  let player;

  const byRole = (a, b) => (a === "potoru.project.yml" ? -1 : b === "potoru.project.yml" ? 1 : a.localeCompare(b));
  const paths = () => (mode === "script" ? [SCRIPT_FILE, ...Object.keys(files).sort(byRole)] : Object.keys(files).sort(byRole));
  const read = (path) => (mode === "script" && path === SCRIPT_FILE ? script : files[path]);

  function renderFiles() {
    fileList.textContent = "";
    const problems = new Set((lastResult?.diagnostics ?? []).filter((entry) => entry.severity === "error").map((entry) => entry.file));
    for (const path of paths()) {
      const item = el("li");
      const button = el("button", { type: "button", title: path, "data-path": path }, path);
      if (path === selected) button.setAttribute("aria-current", "true");
      if (problems.has(path)) button.setAttribute("data-problem", "");
      button.addEventListener("click", () => select(path));
      item.append(button);
      fileList.append(item);
    }
    if (!paths().length) fileList.append(el("li", { class: "empty" }, "No files"));
  }

  function select(path, line) {
    selected = path;
    const value = read(path);
    if (typeof value === "string") { editor.readOnly = false; editor.value = value; }
    else { editor.readOnly = true; editor.value = value ? `(binary file, ${value.byteLength} bytes)` : ""; }
    renderFiles();
    if (line && typeof value === "string") {
      const lines = value.split("\n");
      const start = lines.slice(0, line - 1).reduce((sum, text) => sum + text.length + 1, 0);
      editor.focus();
      editor.setSelectionRange(start, start + (lines[line - 1]?.length ?? 0));
      editor.scrollTop = Math.max(0, (line - 3) * 18);
    }
  }

  editor.addEventListener("input", () => {
    if (editor.readOnly || !selected) return;
    if (mode === "script" && selected === SCRIPT_FILE) script = editor.value; else files[selected] = editor.value;
  });

  function renderDiagnostics(result) {
    diagnosticList.textContent = "";
    const entries = result.diagnostics ?? [];
    if (!entries.length) { diagnosticList.append(el("li", { class: "empty" }, result.ok ? "No problems." : "")); return; }
    for (const entry of entries) {
      const item = el("li");
      const where = entry.file ? `${entry.file}${entry.line ? `:${entry.line}` : ""}` : "";
      const button = el("button", { type: "button", class: entry.severity ?? "error" }, `${where ? `${where}  ` : ""}${entry.code}: ${entry.message}`);
      if (entry.fix) button.append(el("span", { class: "fix" }, entry.fix));
      if (entry.file && (read(entry.file) !== undefined)) button.addEventListener("click", () => select(entry.file, entry.line));
      item.append(button);
      diagnosticList.append(item);
    }
  }

  async function showPreview(poto) {
    if (!player) {
      const { createPlayer } = await import("./player.mjs");
      player = createPlayer(preview, { src: "", controls: true, autoplay: true, loop: true, muted: true }, { id: `${id}-preview`, emit: () => {} });
    }
    await player.load(poto);
  }

  // Compiles run one after another: a click (or compile()) during a run is queued, never dropped.
  let queue = Promise.resolve();
  function run() {
    const next = queue.then(runOnce, runOnce);
    queue = next.catch(() => {});
    return next;
  }

  async function runOnce() {
    compileButton.disabled = true;
    status.textContent = mode === "script" ? "Running the script…" : "Compiling…";
    try {
      const result = await compile(mode === "script" ? { script, files } : { files });
      lastResult = result;
      if (mode === "script" && result.files) generated = result.files;
      renderDiagnostics(result);
      renderFiles();
      downloadButton.disabled = !result.poto;
      const errors = result.diagnostics.filter((entry) => entry.severity === "error").length;
      status.textContent = result.ok ? `Compiled: ${result.sizes.poto.toLocaleString()} bytes, ${result.sizes.ms} ms${result.diagnostics.length ? `, ${result.diagnostics.length} warnings` : ""}` : `${errors} problem${errors === 1 ? "" : "s"}`;
      if (result.ok) {
        emit("compiled", { id, ok: true, poto: result.poto, diagnostics: result.diagnostics, files: result.files ?? { ...files }, sizes: result.sizes });
        await showPreview(result.poto).catch((error) => { status.textContent = `Preview failed: ${error.message}`; });
      } else {
        emit("error", { id, ok: false, diagnostics: result.diagnostics, sizes: result.sizes });
      }
      return result;
    } finally {
      compileButton.disabled = false;
    }
  }

  let generated;
  function download() {
    if (!lastResult?.poto) return false;
    const url = URL.createObjectURL(new Blob([lastResult.poto], { type: "application/octet-stream" }));
    const link = el("a", { href: url, download: `${(lastResult.scenes?.[0] ?? "story").replace(/[^\w-]+/g, "-")}.poto` });
    root.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return true;
  }

  compileButton.addEventListener("click", () => { void run(); });
  downloadButton.addEventListener("click", download);

  select(paths()[0] ?? "");
  if (paths().length && (mode === "files" ? Object.keys(files).length : script.trim())) void run();

  return {
    id,
    host,
    getFiles: () => ({ ...files }),
    setFiles(next = {}) { files = { ...next }; select(paths().includes(selected) ? selected : paths()[0] ?? ""); },
    getScript: () => script,
    setScript(next = "") { script = String(next); if (selected === SCRIPT_FILE) select(SCRIPT_FILE); },
    /** The source folder the last Way B run produced (script mode). */
    getGeneratedFiles: () => (generated ? { ...generated } : undefined),
    compile: run,
    download,
    get lastResult() { return lastResult; },
    configure(next = {}) {
      if (next.mode && next.mode !== mode) mode = next.mode === "script" ? "script" : "files";
      if (next.files) files = { ...next.files };
      if (typeof next.script === "string") script = next.script;
      select(paths()[0] ?? "");
    },
    destroy() { player?.destroy(); root.textContent = ""; }
  };
}
