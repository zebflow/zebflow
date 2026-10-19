#!/usr/bin/env node
/**
 * Builds zeb/world 0.1 (`0.1/runtime/`) from an aslnut source checkout.
 *
 *   ASLNUT_SRC=<aslnut checkout> node blessed/rwe-libraries/world/build/build.mjs
 *
 * ASLNUT_SRC must have its dependencies installed (`npm ci --ignore-scripts`). aslnut does not
 * depend on esbuild; it builds with Rolldown (through Vite), so the bundler is taken from
 * ASLNUT_SRC/node_modules/rolldown. Steps:
 * 0. The interface guard (`interface.mjs`): Zebflow's wrapper (`../src/react.ts`) must have
 *    exactly aslnut/react's interface, checked with aslnut's own TypeScript.
 * 1. `entry.ts`: aslnut's root API (`packages/aslnut/src/index.ts`) and Zebflow's Zeb React
 *    wrapper (`../src/react.ts`), bundled as one ES module with code splitting
 *    (`world.bundle.mjs` and the lazy chunks next to it), minified, es2022, for the browser.
 *    `@aslnut-src/` resolves to `packages/aslnut/src`, `node:*` stays external (aslnut reaches
 *    it only in Node), and the wrapper's `"zeb/react"` is aliased to `zeb-react.mjs`, which
 *    reads Zeb React's global. aslnut's own React shell is not bundled (checked on the module
 *    graph). The wrapper's `World` wins over the root's `World` class (an explicit export beats
 *    `export *`); the finished bundle is loaded in Node to prove it.
 * 2. `tiles.worker.ts`: the street and road tile worker (`tiles.worker.mjs`), served by
 *    `entry.ts` as `tilesWorkerUrl`.
 * 3. LICENSE, NOTICE and DATA-LICENCES.md copied unchanged from aslnut's package, and
 *    THIRD-PARTY-NOTICES.txt written from the npm packages the bundle really contains.
 * 4. exports.json and keywords.json, the manifest (exports, size, integrity, licence),
 *    FILES-SHA256.txt, and the embedded file list in src/platform/web/embedded.rs.
 * It refuses to finish when a built file names a local path.
 */
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { checkInterface } from "./interface.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const LIBRARY = resolve(HERE, "..");
const RUNTIME = join(LIBRARY, "0.1", "runtime");
const ZEBFLOW = resolve(LIBRARY, "../../..");
const SOURCE = process.env.ASLNUT_SRC && resolve(process.env.ASLNUT_SRC);
const PACKAGE = SOURCE && join(SOURCE, "packages/aslnut");
if (!SOURCE || !existsSync(join(PACKAGE, "src/index.ts"))) {
  console.error("Set ASLNUT_SRC to an aslnut checkout (dependencies installed with `npm ci --ignore-scripts`).");
  process.exit(1);
}
let rolldownPath;
try { rolldownPath = createRequire(join(SOURCE, "package.json")).resolve("rolldown"); } catch {
  console.error("rolldown was not found in ASLNUT_SRC/node_modules; run `npm ci --ignore-scripts` there.");
  process.exit(1);
}
const { rolldown } = await import(pathToFileURL(rolldownPath).href);
const rolldownVersion = JSON.parse(readFileSync(join(dirname(rolldownPath), "../package.json"), "utf8")).version;
const aslnut = JSON.parse(readFileSync(join(PACKAGE, "package.json"), "utf8"));
const commit = execFileSync("git", ["-C", SOURCE, "rev-parse", "--short=7", "HEAD"], { encoding: "utf8" }).trim();

// The shim names what the wrapper imports from "zeb/react"; Zeb React must offer each name.
const shimNames = /export const \{([^}]*)\} = R;/.exec(readFileSync(join(HERE, "zeb-react.mjs"), "utf8"))[1].split(",").map((name) => name.trim());
const zebReact = /globalThis\.__zebReact = \{([^}]*)\}/.exec(readFileSync(join(ZEBFLOW, "src/rwe/runtime/zeb_react.js"), "utf8"))[1];
const missing = shimNames.filter((name) => !new RegExp(`\\b${name}\\b`).test(zebReact));
if (missing.length) { console.error(`Zeb React does not export what the wrapper needs: ${missing.join(", ")}`); process.exit(1); }

// 0. The wrapper's interface is aslnut/react's, or nothing is built.
if (!checkInterface(SOURCE)) process.exit(1);

// ---- clean: everything under 0.1/runtime is generated ----
rmSync(RUNTIME, { recursive: true, force: true });
mkdirSync(RUNTIME, { recursive: true });

// `@aslnut-src/x` is aslnut's `packages/aslnut/src/x`.
const aslnutSrc = {
  name: "aslnut-src",
  resolveId(id) { return id.startsWith("@aslnut-src/") ? join(PACKAGE, "src", id.slice("@aslnut-src/".length)) : null; }
};
const warnings = [];
const common = {
  platform: "browser",
  external: [/^node:/],
  tsconfig: join(PACKAGE, "tsconfig.json"),
  transform: { target: "es2022" },
  plugins: [aslnutSrc],
  onLog(level, log, handler) {
    if (level === "warn") warnings.push(`${log.code ?? "warning"}: ${log.message}`);
    handler(level, log);
  }
};
const output = {
  format: "esm",
  // Legal comments (@license, @preserve, /*!) are kept in the output (checked with a probe file). aslnut's sources
  // carry none today; its notices travel as LICENSE, NOTICE and THIRD-PARTY-NOTICES.txt beside the bundle.
  minify: { codegen: { legalComments: "eof" } },
  sourcemap: false,
  comments: { legal: true, annotation: true, jsdoc: false },
  entryFileNames: "[name].mjs",
  chunkFileNames: "[name]-[hash].mjs"
};

// 1. The entry and its lazy chunks.
const main = await rolldown({ ...common, input: { "world.bundle": join(HERE, "entry.ts") }, resolve: { alias: { "zeb/react": join(HERE, "zeb-react.mjs") } } });
const mainOut = (await main.write({ ...output, dir: RUNTIME })).output.filter((item) => item.type === "chunk");
await main.close();
// 2. The tile worker.
const lane = await rolldown({ ...common, input: { "tiles.worker": join(HERE, "tiles.worker.ts") } });
const laneOut = (await lane.write({ ...output, dir: RUNTIME, codeSplitting: false })).output.filter((item) => item.type === "chunk");
await lane.close();
if (warnings.length) {
  console.error(`The bundler warned ${warnings.length} time(s):\n${[...new Set(warnings)].map((line) => `  ${line}`).join("\n")}`);
  const fatal = warnings.filter((line) => /AMBIGUOUS|CONFLICT|UNRESOLVED_IMPORT|MISSING_EXPORT|CIRCULAR_REEXPORT/i.test(line));
  if (fatal.length) { console.error("Refusing a bundle with ambiguous, conflicting or missing exports."); process.exit(1); }
}
// Zebflow's wrapper is in the bundle, aslnut's own React shell is not.
const mainModules = mainOut.flatMap((chunk) => chunk.moduleIds.map((id) => id.split(sep).join("/")));
if (mainModules.some((id) => id.endsWith("/core/world/host/react.ts") || id.endsWith("/aslnut/src/react.ts"))) {
  console.error("aslnut's own React shell was bundled; the entry must export Zebflow's wrapper (../src/react.ts).");
  process.exit(1);
}
if (!mainModules.includes(join(LIBRARY, "src/react.ts").split(sep).join("/"))) { console.error("Zebflow's wrapper (src/react.ts) is not in the bundle."); process.exit(1); }
const entryChunk = mainOut.find((item) => item.isEntry && item.name === "world.bundle");
const exportNames = [...entryChunk.exports].filter((name) => name !== "default").sort();

// 3. Notices. LICENSE, NOTICE and DATA-LICENCES.md are aslnut's own, unchanged.
for (const file of ["LICENSE", "NOTICE", "DATA-LICENCES.md"]) copyFileSync(join(PACKAGE, file), join(LIBRARY, file));
const bundled = new Map();
for (const chunk of [...mainOut, ...laneOut]) {
  for (const id of chunk.moduleIds) {
    const parts = id.split(sep === "\\" ? /[\\/]/ : "/");
    const at = parts.lastIndexOf("node_modules");
    if (at < 0) continue;
    const name = parts[at + 1].startsWith("@") ? `${parts[at + 1]}/${parts[at + 2]}` : parts[at + 1];
    if (!bundled.has(name)) bundled.set(name, parts.slice(0, at + 1 + (name.includes("/") ? 2 : 1)).join("/"));
  }
}
const people = (author) => String(typeof author === "string" ? author : author?.name ?? "").replace(/\s*[<(].*$/, "").trim();
const third = [...bundled].sort(([a], [b]) => a.localeCompare(b)).map(([name, dir]) => {
  const meta = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  const licenceFile = readdirSync(dir).find((file) => /^licen[cs]e(\.md|\.txt)?$/i.test(file));
  return { name, version: meta.version, spdx: meta.license, holder: people(meta.author ?? meta.contributors?.[0]), text: licenceFile ? readFileSync(join(dir, licenceFile), "utf8").trim() : "" };
});
writeFileSync(join(LIBRARY, "THIRD-PARTY-NOTICES.txt"), [
  "Third-party code inside zeb/world's bundle (the npm packages aslnut depends on).",
  "aslnut's own notices are in LICENSE and NOTICE; the MIT notices NOTICE carries (the UTCI coefficients and the sky shader) travel there.",
  "",
  ...third.flatMap((pkg) => ["=".repeat(78), `${pkg.name} ${pkg.version} (${pkg.spdx})`, "=".repeat(78), "", pkg.text || `(${pkg.name} ships no licence file; its package.json declares ${pkg.spdx})`, ""])
].join("\n"));

// A local path may not ship: home folders, temp folders, toolchains, this machine's checkouts.
const files = (folder) => readdirSync(folder).flatMap((name) => { const path = join(folder, name); return statSync(path).isDirectory() ? files(path) : [path]; });
const forbidden = [/\/Users\/[^/\s"']+/, /\/home\/[a-z][^/\s"']*/, /[A-Z]:\\Users\\/, /\/private\/(?:tmp|var)\//, /\/var\/folders\//, /\.cargo\/registry/, /rustup\/toolchains/];
const machine = [SOURCE, ZEBFLOW, homedir()].filter((path) => path.length > 3);
const scan = (paths) => {
  for (const path of paths) {
    const text = readFileSync(path, "latin1");
    if (machine.some((local) => text.includes(local)) || forbidden.some((pattern) => pattern.test(text))) {
      console.error(`A built file names a local path: ${relative(LIBRARY, path)}`);
      process.exit(1);
    }
  }
};
scan(files(RUNTIME));
scan([join(LIBRARY, "THIRD-PARTY-NOTICES.txt")]);

// Prove the bundle: the wrapper's World (not the root's class) is the one named `World`, and the page-facing names exist.
globalThis.__zebReact = { createContext: (value) => ({ defaultValue: value, Provider: function Provider() {} }), createElement() {}, useCallback() {}, useContext() {}, useEffect() {}, useRef() {}, useState() {}, useSyncExternalStore() {} };
const loaded = await import(pathToFileURL(join(RUNTIME, "world.bundle.mjs")).href);
delete globalThis.__zebReact;
const isClass = (value) => typeof value === "function" && /^class[\s{]/.test(Function.prototype.toString.call(value));
for (const name of ["World", "useWorld", "useElement", "useWorldEvent", "world", "earth", "plane", "layer", "data", "scale", "liveGraphics", "worker", "tilesWorkerUrl"]) {
  if (!(name in loaded)) { console.error(`The bundle does not export ${name}.`); process.exit(1); }
}
if (typeof loaded.World !== "function" || isClass(loaded.World) || loaded.World.length !== 1) {
  console.error("The bundle's World is not the wrapper's component (the root's World class won).");
  process.exit(1);
}
if (!loaded.tilesWorkerUrl.endsWith("/tiles.worker.mjs")) { console.error(`tilesWorkerUrl is ${loaded.tilesWorkerUrl}`); process.exit(1); }
if (JSON.stringify(Object.keys(loaded).sort()) !== JSON.stringify(exportNames)) { console.error("The bundle's exports differ from the entry chunk's."); process.exit(1); }

// 4. exports.json, keywords.json.
const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
const ENTRY = "runtime/world.bundle.mjs";
const described = {
  World: ["component", "Zebflow's Zeb React wrapper, with aslnut/react's interface: <World options elements onReady viewChange headless className style>{overlays}</World>. Makes a world on its own canvas on mount, keeps `elements` in step by id, disposes on unmount (aslnut's shell(w) keeps it in step)."],
  useWorld: ["hook", "The aslnut world of the <World> round it (null until ready, and after it is gone)."],
  useElement: ["hook", "An element of the world by id (null until added, and after removed)."],
  useWorldEvent: ["hook", "Hear a world's event (tap, hover, select, elements, level, frame, error, long, governor) while mounted."],
  tilesWorkerUrl: ["value", "URL of the tile worker next to this bundle: worker(tilesWorkerUrl) runs aslnut's street and road meshing off the main thread."],
  liveGraphics: ["function", "What graphics memory aslnut holds now: graphics, buffers, textures, targets, programs, bytes."]
};
const raw = Object.fromEntries(exportNames.map((name) => {
  const [kind, description] = (Object.hasOwn(described, name) ? described[name] : null) ?? [/^use[A-Z]/.test(name) ? "hook" : null, null];
  return [name, { ...(kind ? { kind } : {}), entry: ENTRY, ...(description ? { description } : {}) }];
}));
writeFileSync(join(LIBRARY, "0.1/exports.json"), `${JSON.stringify({ package: "zeb/world", version: "0.1", raw, wrappers: {} }, null, 2)}\n`);
const keywords = ["World", "useWorld", "useElement", "useWorldEvent", "tilesWorkerUrl", "aslnut", "world", "globe", "earth", "gis", "digital twin", "webgpu", "webgl2", "map", "3d", "layer"];
writeFileSync(join(LIBRARY, "0.1/keywords.json"), `${JSON.stringify([...new Set(keywords)], null, 2)}\n`);

// library.json is written by hand; only the aslnut version it names follows the build.
const libraryPath = join(LIBRARY, "0.1/library.json");
const library = JSON.parse(readFileSync(libraryPath, "utf8"));
library.source.package_version = aslnut.version;
writeFileSync(libraryPath, `${JSON.stringify(library, null, 2)}\n`);

// The manifest: its description is kept; the rest follows the build.
const manifestPath = join(LIBRARY, "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const bundle = join(RUNTIME, "world.bundle.mjs");
const runtimeFiles = files(RUNTIME);
const runtimeBytes = runtimeFiles.reduce((sum, path) => sum + statSync(path).size, 0);
manifest.spec.exports = exportNames;
manifest.spec.versions = {
  "0.1": {
    entry: "0.1/runtime/world.bundle.mjs",
    source: "offline",
    package_version: aslnut.version,
    size_bytes: statSync(bundle).size,
    integrity: `sha256:${sha256(bundle)}`,
    note: `aslnut ${aslnut.version} (commit ${commit}, https://github.com/aslnut/aslnut), unmodified, via Rolldown ${rolldownVersion}: one ES module with lazy chunks, minified, es2022. World, useWorld, useElement and useWorldEvent are Zebflow's own Zeb React wrapper (src/react.ts, MIT) with aslnut/react's interface, built on aslnut's shell(w); its World wins over the root's World class. Chunks and tiles.worker.mjs sit next to the entry (new URL(..., import.meta.url)). Build: ASLNUT_SRC=<aslnut checkout> node blessed/rwe-libraries/world/build/build.mjs. Every file: FILES-SHA256.txt.`
  }
};
const holders = ["The aslnut authors", "Zebflow (the Zeb React wrapper, src/react.ts, MIT)", "Federico Tartarini (pythermalcomfort, UTCI coefficients, MIT)", "The three.js authors (Sky example, MIT)"];
for (const pkg of third) if (pkg.holder) holders.push(`${pkg.holder} (${pkg.name}, ${pkg.spdx})`);
manifest.spec.license = {
  spdx: ["Apache-2.0", "MIT", ...third.map((pkg) => pkg.spdx)].filter((id, at, all) => all.indexOf(id) === at).join(" AND "),
  holders,
  notices: ["LICENSE", "NOTICE", "DATA-LICENCES.md", "THIRD-PARTY-NOTICES.txt"],
  modified: false
};
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
scan([manifestPath, join(LIBRARY, "0.1/exports.json")]);

// 5. Checksums, then the embedded list.
const listed = files(LIBRARY).map((path) => relative(LIBRARY, path).split(sep).join("/")).filter((path) => path !== "FILES-SHA256.txt" && !path.startsWith("build/") && !path.startsWith("src/")).sort();
writeFileSync(join(LIBRARY, "FILES-SHA256.txt"), listed.map((path) => `${sha256(join(LIBRARY, path))}  ${path}`).join("\n") + "\n");

const embeddedPath = join(ZEBFLOW, "src/platform/web/embedded.rs");
const embedded = readFileSync(embeddedPath, "utf8");
// Every file of the package is served, the checksum list too.
const served = [...listed, "FILES-SHA256.txt"].sort();
const entries = served.map((path) => `    EmbeddedAsset {\n        path: "zeb/world/${path}",\n        bytes: include_bytes!("../../../blessed/rwe-libraries/world/${path}"),\n    },\n`).join("");
const comment = "    // zeb/world: every file of the package. The entry finds its lazy chunks and the tile\n    // worker with `new URL(…, import.meta.url)`, so each one has to answer next to it under\n    // `0.1/runtime/`. Chunk names carry content hashes: a rebuild of the library rewrites this list.\n";
const block = /(    \/\/ zeb\/world: every file of the package\.[^\n]*\n(?:    \/\/[^\n]*\n)*)((?:    EmbeddedAsset \{\n        path: "zeb\/world\/[^"]+",\n        bytes: include_bytes!\("[^"]+"\),\n    \},\n)+)/;
const anchor = "    // No library.json for zeb/react.";
let rewritten;
if (block.test(embedded)) rewritten = embedded.replace(block, (_all, head) => head + entries);
else if (embedded.includes(anchor)) rewritten = embedded.replace(anchor, () => comment + entries + anchor);
else { console.error("Neither the zeb/world block nor its anchor was found in embedded.rs; update it by hand."); process.exit(1); }
writeFileSync(embeddedPath, rewritten);

const chunks = runtimeFiles.filter((path) => /\.mjs$/.test(path) && path !== bundle && basename(path) !== "tiles.worker.mjs");
console.log(`zeb/world built from aslnut ${aslnut.version} (${commit}): ${exportNames.length} exports, entry ${statSync(bundle).size} bytes, ${chunks.length} chunk(s), tile worker ${statSync(join(RUNTIME, "tiles.worker.mjs")).size} bytes, runtime ${runtimeBytes} bytes in ${runtimeFiles.length} files, ${served.length} served files, ${manifest.spec.versions["0.1"].integrity}`);
