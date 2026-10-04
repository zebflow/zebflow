/**
 * zeb/potoru — compiler part (loaded on demand by entry.mjs on the first `compile()` call or when a
 * PotoruCompiler mounts; bundled into `compiler-*.mjs` chunks).
 *
 * Way A: a format v4 source folder (a files map) → validate → compile → `.poto` bytes, with the same
 *        Potoru format code the Studio and the `potoru` CLI use (`v4FromFiles`, `validateV4Project`,
 *        `compileV4Story`; linked libraries and font aliases through the authoring Project, as
 *        `potoru export` does). Option `profile: "presentation"` compiles every root State as a slide.
 * Way B (EXPERIMENTAL): JavaScript that uses the authoring API runs in a sandboxed iframe that holds
 *        only `potoru-authoring-sandbox.js`; it returns a Project, the iframe posts back the source
 *        folder's files map, and Way A compiles it.
 *
 * `@potoru-src/` is resolved by build/build.mjs to $POTORU_SRC (the Potoru `designer/e3` folder).
 * `fileOfPath` / `ownerFile` / `diagnosticFile` are adapted from $POTORU_SRC/authoring/src/cli.ts
 * (Apache-2.0), which cannot be bundled for the browser as a whole (it uses node:fs).
 */
import { FolderError, SPLIT_TRACKS, YamlError, isRootRef, storyClosure, v4FromFiles, validateV4Project } from "@potoru-src/app/src/format/index.ts";
import { compileV4Story } from "@potoru-src/app/src/format/compile-v4.ts";
import { compileLinkedPotoStory } from "@potoru-src/app/src/runtime/story-compile.ts";
import { encodePoto } from "@potoru-src/app/src/runtime/container.ts";
import { Project } from "@potoru-src/authoring/src/index.ts";
import { decodePotoLibrary } from "@potoru-src/app/src/runtime/library.ts";
import { fontAliasesUsed, resolveFontAliases } from "@potoru-src/app/src/runtime/font-aliases.ts";
import { loadActionCore } from "@potoru-src/app/src/runtime/action-core.ts";
import { isPotoruError } from "@potoru-src/app/src/core/diagnostics.ts";

export { createCompilerWidget } from "./widget.mjs";

const SANDBOX_FILE = "potoru-authoring-sandbox.js";
const DEFAULT_TIMEOUT_MS = 10000;
const SCRIPT_FILE = "script.js";

/* ── Public entry ─────────────────────────────────────────────────────────── */

export async function compile(input = {}) {
  const started = performance.now();
  try {
    if (typeof input.script === "string") {
      const run = await runScript(input.script, normalizeFiles(input.files ?? {}), input);
      if (!run.ok) return finish({ ok: false, diagnostics: run.diagnostics }, started, run.files);
      const result = await compileFiles(normalizeFiles(run.files), input);
      return finish({ ...result, files: run.files }, started, run.files);
    }
    if (input.files && typeof input.files === "object") {
      const files = normalizeFiles(input.files);
      return finish(await compileFiles(files, input), started, files);
    }
    return finish({ ok: false, diagnostics: [diag("usage", "compile() needs { files } (a source folder map) or { script } (authoring API JavaScript).", { fix: "pass { files: { \"potoru.project.yml\": \"…\", … } }" })] }, started);
  } catch (error) {
    return finish({ ok: false, diagnostics: [fromError(error, "compile-error")] }, started);
  }
}

function finish(result, started, files) {
  const sourceBytes = files ? Object.values(files).reduce((sum, value) => sum + (typeof value === "string" ? new TextEncoder().encode(value).length : value?.byteLength ?? 0), 0) : 0;
  return {
    ok: Boolean(result.ok),
    ...(result.poto ? { poto: result.poto } : {}),
    diagnostics: result.diagnostics ?? [],
    ...(result.files ? { files: result.files } : {}),
    ...(result.scenes ? { scenes: result.scenes } : {}),
    sizes: { poto: result.poto?.byteLength ?? 0, source: sourceBytes, files: files ? Object.keys(files).length : 0, ms: Math.round(performance.now() - started) }
  };
}

/* ── Way A ────────────────────────────────────────────────────────────────── */

function normalizeFiles(files) {
  const out = {};
  for (const [rawPath, value] of Object.entries(files ?? {})) {
    const path = String(rawPath).replace(/\\/g, "/").replace(/^\.?\//, "");
    if (!path || path.split("/").includes("..")) continue;
    if (typeof value === "string") out[path] = value;
    else if (value instanceof Uint8Array) out[path] = value;
    else if (value instanceof ArrayBuffer) out[path] = new Uint8Array(value);
    else if (ArrayBuffer.isView(value)) out[path] = new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  }
  return out;
}

async function compileFiles(files, options) {
  const diagnostics = [];
  let project;
  try {
    project = v4FromFiles(files);
  } catch (error) {
    return { ok: false, diagnostics: [fromError(error, "invalid-folder")] };
  }
  const located = (code, severity, message, path, file, extra = {}) => {
    const name = file ?? (path ? fileOfPath(project, path) : "potoru.project.yml");
    return { code, severity, message, file: name, line: lineOf(files[name], project, path), ...(path ? { path } : {}), ...extra };
  };
  for (const issue of validateV4Project(project)) diagnostics.push(located(issue.code, issue.severity === "warning" ? "warning" : "error", issue.message, issue.path, undefined, issue.fix ? { fix: issue.fix } : {}));
  if (project.kind !== "library") {
    try {
      const closure = storyClosure(project);
      for (const item of closure.missing) diagnostics.push({ code: "missing-reference", severity: "error", message: `missing ${item.kind} ${item.id} (named by ${item.from})`, file: item.file, line: lineOf(files[item.file], project, undefined, item.id) });
    } catch { /* the validator reports the cause */ }
  } else {
    diagnostics.push({ code: "library-source", severity: "error", message: "This is a library source (kind: library); compile() makes story packages (.poto).", file: "potoru.project.yml", line: lineOf(files["potoru.project.yml"], project, undefined, "kind"), fix: "export the library with the potoru CLI (potoru export <folder> <name>.potolib)" });
  }
  if (diagnostics.some((entry) => entry.severity === "error")) return { ok: false, diagnostics };

  const libraries = [];
  for (const [index, bytes] of (options.libraries ?? []).entries()) {
    try { libraries.push(await decodePotoLibrary(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes))); }
    catch (error) { diagnostics.push({ ...fromError(error, "library-file"), file: `libraries[${index}]` }); }
  }
  if (diagnostics.some((entry) => entry.severity === "error")) return { ok: false, diagnostics };

  const sceneId = options.scene ?? project.composition.activeSceneId ?? project.scenes[0]?.id;
  const scene = project.scenes.find((candidate) => candidate.id === sceneId);
  if (!scene) return { ok: false, diagnostics: [...diagnostics, { code: "missing-scene", severity: "error", message: `No Scene ${sceneId ?? "(none)"} in this project.`, file: "potoru.project.yml", line: 1, fix: "pass scene: <id> of one of the project's Scenes" }] };

  const attempt = async () => {
    // As `potoru export`: an inline Scene is opened as a Designer project (linked libraries attached,
    // alias fonts resolved); linked when it has dependencies or uses font aliases.
    let opened;
    if (!isRootRef(scene.root)) {
      opened = Project.fromV4(project);
      for (const library of libraries) {
        const linked = opened.data.libraries?.some((reference) => !reference.via && reference.slug === library.manifest.slug && reference.contentHash === library.contentHash);
        if (linked) opened.link(library); else opened.install(library);
      }
      const aliases = fontAliasesUsed(opened.build());
      if (aliases.length) opened.useFonts(await builtinFonts(aliases, libraries));
    }
    const fonts = opened?.fonts ?? [];
    // Same compile calls as the CLI's exportLinkedScenePoto / exportV4ScenePoto (authoring/src/preview.ts),
    // plus the optional experience `profile` ("presentation": every root State is a slide).
    const selection = { sceneId: scene.id, ...(options.state ? { animationId: options.state } : {}), ...(options.profile ? { profile: options.profile } : {}) };
    const compiled = (project.dependencies?.length || fonts.length) && opened
      ? compileLinkedPotoStory(opened.build(), opened.libraries, selection, fonts)
      : compileV4Story(project, { ...selection, ...(options.locale ? { locale: options.locale } : {}) });
    return compiled.runtime ? { bytes: await encodePoto(compiled.runtime), runtime: compiled.runtime, diagnostics: compiled.diagnostics } : { diagnostics: compiled.diagnostics };
  };
  let result = await attempt();
  // Interactive stories compile with the Rust action core: load its wasm once, then compile again.
  if (!result.bytes && result.diagnostics.some((entry) => entry.code === "interaction-core")) {
    await loadActionCore();
    result = await attempt();
  }
  for (const entry of result.diagnostics) {
    const severity = entry.severity === "warning" ? "warning" : entry.severity === "info" ? "info" : "error";
    const named = entry.path ? /^([^:]+\.yml): (.*)$/.exec(entry.path) : null;
    const file = diagnosticFile(project, scene.id, entry.path);
    diagnostics.push({ code: entry.code, severity, message: named ? `${named[2]}: ${entry.message}` : entry.message, file, line: lineOf(files[file], project, named ? undefined : entry.path, named ? lastKey(named[2]) : undefined), ...(entry.path ? { path: entry.path } : {}) });
  }
  if (!result.bytes) return { ok: false, diagnostics: diagnostics.some((entry) => entry.severity === "error") ? diagnostics : [...diagnostics, { code: "compile-error", severity: "error", message: "The package could not be compiled.", file: `scenes/${scene.id}/scene.yml`, line: 1 }] };
  const story = result.runtime?.story ? [result.runtime.story.entry, ...(result.runtime.scenes ?? []).map((entry) => entry.id ?? "?")] : [scene.id];
  return { ok: true, poto: result.bytes, diagnostics, scenes: story };
}

/** Built-in fonts for `font:` aliases, fetched from `fonts/` next to the bundle (offline, same origin). */
async function builtinFonts(aliases, installed) {
  const builtin = async (entry) => {
    try {
      const response = await fetch(new URL(`fonts/${entry.file}`, import.meta.url));
      return response.ok ? new Uint8Array(await response.arrayBuffer()) : undefined;
    } catch { return undefined; }
  };
  const { fonts, diagnostics } = await resolveFontAliases(aliases, { installed, builtin });
  const errors = diagnostics.filter((entry) => entry.severity === "error");
  if (errors.length) throw Object.assign(new Error(errors.map((entry) => `${entry.code}: ${entry.message}`).join("\n")), { code: errors[0].code });
  return fonts;
}

/* ── Locating problems (adapted from the potoru CLI) ──────────────────────── */

function fileOfPath(project, path) {
  const object = /^objects\[(\d+)\](.*)$/.exec(path);
  if (object) {
    const owner = project.objects[Number(object[1])];
    return owner ? ownerFile(`objects/${owner.id}/`, owner.states, object[2]) : "potoru.project.yml";
  }
  const scene = /^scenes\[(\d+)\](.*)$/.exec(path);
  if (scene) {
    const found = project.scenes[Number(scene[1])];
    if (!found) return "potoru.project.yml";
    const rest = scene[2];
    if (rest.startsWith(".root") && !isRootRef(found.root)) return ownerFile(`scenes/${found.id}/root/`, found.root.states, rest.slice(".root".length));
    return `scenes/${found.id}/scene.yml`;
  }
  const asset = /^assets\[(\d+)\]/.exec(path);
  if (asset) return `assets/${project.assets[Number(asset[1])]?.id ?? "?"}.asset.yml`;
  if (path.startsWith("speech")) return "speech/lines.yml";
  const camera = /^cameras(?:\.([^.[]+)|\[(\d+)\])/.exec(path);
  if (camera) return `cameras/${camera[1] ?? project.cameras?.[Number(camera[2])]?.id ?? "?"}.yml`;
  const strings = /^strings\.([^.]+)/.exec(path);
  if (strings) return `strings/${strings[1]}.yml`;
  return "potoru.project.yml";
}

function ownerFile(dir, states, rest) {
  const stroke = /^\.definitionTree\.nodesById\.([^.]+)\.freehand/.exec(rest);
  if (stroke) return `${dir}strokes/${stroke[1]}.yml`;
  const mesh = /^\.definitionTree\.nodesById\.([^.]+)\.mesh\.(?:vertices|triangles|size|outline)/.exec(rest);
  if (mesh) return `${dir}geometry/${mesh[1]}.mesh.yml`;
  const geometry = /^\.definitionTree\.nodesById\.([^.]+)\.drawable\.geometry/.exec(rest);
  if (geometry) return `${dir}geometry/${geometry[1]}.yml`;
  const state = /^\.states\[(\d+)\](?:\.timeline\[(\d+)\])?/.exec(rest);
  if (!state) return `${dir}object.yml`;
  const found = states[Number(state[1])];
  if (!found) return `${dir}object.yml`;
  const track = state[2] === undefined ? undefined : found.timeline[Number(state[2])];
  return track && found.timeline.length > SPLIT_TRACKS ? `${dir}states/${found.id}/${track.id}.yml` : `${dir}states/${found.id}.yml`;
}

function diagnosticFile(project, sceneId, path) {
  const named = path ? /^([^:]+\.yml): /.exec(path) : null;
  if (named) return named[1];
  if (path) {
    const owners = [
      ...project.scenes.flatMap((scene) => (isRootRef(scene.root) ? [] : [{ dir: `scenes/${scene.id}/root/`, states: scene.root.states }])),
      ...project.objects.map((object) => ({ dir: `objects/${object.id}/`, states: object.states }))
    ];
    let best;
    for (const owner of owners) {
      for (const state of owner.states) {
        for (const track of state.timeline) {
          if (!path.includes(track.id) || (best && best.length >= track.id.length)) continue;
          best = { file: state.timeline.length > SPLIT_TRACKS ? `${owner.dir}states/${state.id}/${track.id}.yml` : `${owner.dir}states/${state.id}.yml`, length: track.id.length };
        }
      }
    }
    if (best) return best.file;
  }
  return `scenes/${sceneId}/scene.yml`;
}

function lastKey(path) {
  const keys = String(path ?? "").split(/[.[\]"]+/).filter((key) => key && !/^\d+$/.test(key));
  return keys.at(-1);
}

const escapeRegExp = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Best-effort 1-based line of a model path in a YAML file: the line of the deepest object on the
 * path whose `id` the file names, then the path's last key at or after it; 1 when nothing matches.
 */
function lineOf(text, project, path, key) {
  if (typeof text !== "string" || !text) return 1;
  const lines = text.split("\n");
  const find = (pattern, from = 0) => { for (let i = from; i < lines.length; i += 1) if (pattern.test(lines[i])) return i; return -1; };
  let anchor = -1;
  if (path) {
    const ids = [];
    let node = project;
    for (const segment of path.match(/[^.[\]]+|\[\d+\]/g) ?? []) {
      if (node === undefined || node === null) break;
      node = segment.startsWith("[") ? node[Number(segment.slice(1, -1))] : node[segment];
      if (node && typeof node === "object" && typeof node.id === "string") ids.push(node.id);
      if (segment !== "nodesById" && /^[A-Za-z0-9_-]+$/.test(segment) && !segment.startsWith("[")) {
        const parent = path.includes(`nodesById.${segment}`);
        if (parent) ids.push(segment);
      }
    }
    for (const id of ids.reverse()) {
      const at = find(new RegExp(`(^|[{,\\s])id:\\s*["']?${escapeRegExp(id)}["']?\\s*(,|}|$)`));
      if (at >= 0) { anchor = at; break; }
    }
    key ??= lastKey(path);
  }
  if (key) {
    const at = find(new RegExp(`(^|[{,\\s-])["']?${escapeRegExp(key)}["']?\\s*:`), Math.max(anchor, 0));
    if (at >= 0 && (anchor < 0 || at - anchor < 40)) return at + 1;
    if (anchor < 0) {
      const anywhere = find(new RegExp(`\\b${escapeRegExp(key)}\\b`));
      if (anywhere >= 0) return anywhere + 1;
    }
  }
  return anchor >= 0 ? anchor + 1 : 1;
}

function diag(code, message, extra = {}) {
  return { code, severity: "error", message, ...extra };
}

function fromError(error, fallbackCode) {
  if (error instanceof YamlError) return { code: error.code, severity: "error", message: error.detail ?? error.message, file: error.file, line: error.line || 1, ...(error.path ? { path: error.path } : {}) };
  if (error instanceof FolderError) return { code: error.code, severity: "error", message: error.message, file: error.file ?? "potoru.project.yml", line: 1 };
  if (isPotoruError(error)) return { line: 1, ...error.diagnostic, severity: "error" };
  return diag(error?.code ?? fallbackCode, error instanceof Error ? error.message : String(error));
}

/* ── Way B: the authoring sandbox (EXPERIMENTAL) ──────────────────────────── */

let sandboxText;
async function sandboxApi() {
  sandboxText ??= fetch(new URL(SANDBOX_FILE, import.meta.url)).then((response) => {
    if (!response.ok) throw new Error(`${SANDBOX_FILE}: HTTP ${response.status}`);
    return response.text();
  }).catch((error) => { sandboxText = undefined; throw error; });
  return sandboxText;
}

/** `import … from "@potoru/authoring"` becomes a destructuring of the sandbox API (same line count). */
function rewriteScript(source) {
  const problems = [];
  const text = source
    .replace(/^([ \t]*)import\s+(\{[^}]*\})\s*from\s*["'](?:@potoru\/authoring|zeb\/potoru)["'];?/gm, (_all, indent, names) => `${indent}const ${names.replace(/\s+as\s+/g, ": ")} = Potoru;`)
    .replace(/^([ \t]*)import\s+\*\s+as\s+([A-Za-z_$][\w$]*)\s+from\s*["'](?:@potoru\/authoring|zeb\/potoru)["'];?/gm, "$1const $2 = Potoru;")
    .replace(/^([ \t]*)export\s+default\s+/gm, "$1return ");
  text.split("\n").forEach((line, index) => {
    if (/^\s*import\s/.test(line)) problems.push(diag("script-import", "Only the authoring API is available in the sandbox (import { Project } from \"@potoru/authoring\").", { file: SCRIPT_FILE, line: index + 1, fix: "remove this import; the API is in scope as Project, format and Potoru" }));
  });
  return { text, problems };
}

const escapeForScript = (text) => text.replace(/<\/(script)/gi, "<\\/$1").replace(/<!--/g, "<\\!--");
const looksLikeTypeScript = (source) => /\b(?:const|let|var)\s+[\w$]+\s*:\s*[\w$[{]|\)\s*:\s*[\w$]+\s*(?:=>|\{)|\binterface\s+\w+\s*\{|\btype\s+\w+\s*=|\bas\s+(?:const|string|number|any)\b|\benum\s+\w+\s*\{/.test(source);
const errorCode = (error) => (typeof error.code === "string" && error.code ? error.code : error.name && error.name !== "Error" ? `script-${error.name.replace(/Error$/, "").replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase() || "error"}` : "script-error");

async function runScript(source, files, options) {
  const { text, problems } = rewriteScript(source);
  if (problems.length) return { ok: false, diagnostics: problems };
  let api;
  try { api = await sandboxApi(); }
  catch (error) { return { ok: false, diagnostics: [diag("sandbox-load", `The authoring sandbox could not be loaded: ${error.message}`)] }; }
  const nonce = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  const head = [
    "<!doctype html><html><head><meta charset=\"utf-8\">",
    "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'unsafe-inline'\">",
    "</head><body>",
    "<script>window.__potoruErrors=[];window.onerror=function(m,s,l,c){window.__potoruErrors.push({message:String(m),line:l,column:c});};</script>",
    `<script>${escapeForScript(api)}</script>`,
    "<script>globalThis.__potoruUser = async function (Potoru, Project, format, files) { return (async () => {"
  ].join("\n");
  const userStart = head.split("\n").length + 1;
  const userLines = text.split("\n").length;
  const runner = `(function(){var N=${JSON.stringify(nonce)};function send(m){m.__potoruSandbox=N;parent.postMessage(m,"*");}
function plain(e){var d=[];try{d=(e&&e.diagnostics||[]).map(function(x){return {code:x.code,severity:x.severity,message:x.message,file:x.file,line:x.line,path:x.path,fix:x.fix};});}catch(_){}return {message:String(e&&e.message||e),stack:String(e&&e.stack||""),code:e&&e.code,name:e&&e.name,diagnostics:d};}
window.addEventListener("message",function(ev){var data=ev.data;if(!data||data.__potoruSandbox!==N||ev.source!==parent)return;
if(window.__potoruErrors.length||typeof globalThis.__potoruUser!=="function"){send({ok:false,parse:true,errors:window.__potoruErrors});return;}
var P=globalThis.PotoruAuthoring;Promise.resolve().then(function(){return globalThis.__potoruUser(P,P.Project,P.format,data.files||{});}).then(function(out){var files;
if(out&&typeof out.toV4==="function")files=P.format.v4ToFiles(out.toV4());else if(out&&out.files&&typeof out.files==="object")files=out.files;
else if(out&&typeof out==="object"&&Object.keys(out).some(function(k){return /\\.yml$/.test(k);}))files=out;
else throw new Error("The script must return a Project (or a source files map).");send({ok:true,files:files});}).catch(function(e){send({ok:false,error:plain(e)});});});
send({ready:true});})();`;
  const doc = `${head}\n${escapeForScript(text)}\n})(); };</script>\n<script>${runner}</script></body></html>`;
  const timeoutMs = Math.max(500, Number(options.timeoutMs) || DEFAULT_TIMEOUT_MS);

  return new Promise((resolve) => {
    const iframe = document.createElement("iframe");
    iframe.setAttribute("sandbox", "allow-scripts");
    iframe.setAttribute("referrerpolicy", "no-referrer");
    iframe.setAttribute("aria-hidden", "true");
    iframe.setAttribute("tabindex", "-1");
    iframe.title = "Potoru authoring sandbox";
    iframe.style.cssText = "position:absolute;width:0;height:0;border:0;visibility:hidden;";
    let done = false;
    const end = (result) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      window.removeEventListener("message", onMessage);
      iframe.remove();
      resolve(result);
    };
    const toLine = (line) => (typeof line === "number" && line >= userStart && line < userStart + userLines ? line - userStart + 1 : undefined);
    const stackLine = (stack) => {
      for (const match of String(stack).matchAll(/about:srcdoc:(\d+):(\d+)/g)) { const line = toLine(Number(match[1])); if (line) return line; }
      return undefined;
    };
    const onMessage = (event) => {
      const data = event.data;
      if (event.source !== iframe.contentWindow || !data || data.__potoruSandbox !== nonce) return;
      if (data.ready) { iframe.contentWindow.postMessage({ __potoruSandbox: nonce, files }, "*"); return; }
      if (data.ok) { end({ ok: true, files: data.files }); return; }
      if (data.parse) {
        const errors = data.errors?.length ? data.errors : [{ message: "The script could not be parsed." }];
        end({ ok: false, diagnostics: errors.map((entry) => diag("script-syntax", entry.message, { file: SCRIPT_FILE, line: toLine(entry.line) ?? 1, ...(entry.column ? { column: entry.column } : {}), ...(looksLikeTypeScript(source) ? { fix: "zeb/potoru 0.1 runs JavaScript only: remove TypeScript type annotations" } : {}) })) });
        return;
      }
      const error = data.error ?? {};
      const inner = (error.diagnostics ?? []).filter((entry) => entry && entry.message);
      const blocked = /Content Security Policy|Failed to fetch|NetworkError|Refused to/i.test(`${error.message} ${error.stack}`);
      end({ ok: false, diagnostics: inner.length ? inner.map((entry) => ({ severity: "error", ...entry, code: entry.code ?? errorCode(error) })) : [diag(blocked ? "script-network-blocked" : errorCode(error), blocked ? `The script tried to use the network, which the sandbox blocks: ${error.message}` : error.message || "The script failed.", { file: SCRIPT_FILE, line: stackLine(error.stack) ?? 1 })] });
    };
    const timer = setTimeout(() => end({ ok: false, diagnostics: [diag("script-timeout", `The script did not finish within ${timeoutMs} ms and was stopped.`, { file: SCRIPT_FILE, line: 1, fix: "look for an endless loop, or pass a larger timeoutMs" })] }), timeoutMs);
    window.addEventListener("message", onMessage);
    iframe.srcdoc = doc;
    (document.body ?? document.documentElement).append(iframe);
  });
}
