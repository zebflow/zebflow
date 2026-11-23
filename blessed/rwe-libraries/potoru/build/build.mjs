#!/usr/bin/env node
/**
 * Builds zeb/potoru 0.1 (`0.1/runtime/`) from a Potoru source checkout.
 *
 *   POTORU_SRC=<potoru checkout>/designer/e3 node blessed/rwe-libraries/potoru/build/build.mjs
 *
 * POTORU_SRC must have its dependencies installed (`npm ci`); esbuild is taken from there. Steps:
 * 1. the entry and its lazy parts: esbuild --bundle --splitting --format=esm --minify
 *    --target=es2022 of `0.1/runtime/entry.mjs` (→ `potoru.bundle.mjs`), with `@potoru-src/`
 *    resolved to POTORU_SRC and Node built-ins replaced by stubs that throw (the browser never
 *    calls them); chunks are `[name]-[hash].mjs` next to the entry;
 * 2. the music Worker (`score-worker.js`, from app/src/runtime/score-worker.ts);
 * 3. the Script-mode sandbox (`potoru-authoring-sandbox.js`, an IIFE of `src/sandbox-api.mjs`
 *    that sets `globalThis.PotoruAuthoring`);
 * 4. the engines' wasm (action, render, score, g2p) and the built-in fonts next to the entry;
 * 5. FILES-SHA256.txt, the manifest's size and integrity, and the embedded file list in
 *    src/platform/web/embedded.rs.
 * It refuses to finish when a built file names a local path.
 */
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const LIBRARY = resolve(HERE, "..");
const RUNTIME = join(LIBRARY, "0.1", "runtime");
const ZEBFLOW = resolve(LIBRARY, "../../..");
const SOURCE = process.env.POTORU_SRC;
if (!SOURCE || !existsSync(join(SOURCE, "app/src/player/index.ts"))) {
  console.error("Set POTORU_SRC to a Potoru checkout's designer/e3 folder (dependencies installed).");
  process.exit(1);
}
const { build } = createRequire(join(SOURCE, "package.json"))("esbuild");
const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

// ---- clean: everything generated (sources, wrappers, package.json and legal files stay) ----
for (const name of readdirSync(RUNTIME)) {
  const path = join(RUNTIME, name);
  if (name === "entry.mjs" || name === "src" || name === "package.json") continue;
  if (/\.(mjs|js|wasm)$/.test(name) || name === "fonts" || name === "assets") rmSync(path, { recursive: true, force: true });
}

// The lean player: Potoru's own `play.js` build (packages/web/dist, `npm run build` there), served
// exactly as built. It is one module for a linear piece, with lazy chunks that import from it, so it
// must never be re-bundled: esbuild splitting would move it into a shared chunk and bring back a
// chain of fetches. The player bridge imports it as `potoru-play`, kept external below.
const PLAY_DIST = join(SOURCE, "packages/web/dist");
if (!existsSync(join(PLAY_DIST, "play.js"))) {
  console.error("No packages/web/dist/play.js under POTORU_SRC: build Potoru's web package first (npm run build in packages/web).");
  process.exit(1);
}
// The player's version, read from its source: play.js does not export it by name.
const controller = readFileSync(join(SOURCE, "app/src/player/controller.ts"), "utf8");
const PLAYER_VERSION = controller.match(/export const PLAYER_VERSION = "([^"]+)"/)?.[1];
if (!PLAYER_VERSION) { console.error("PLAYER_VERSION not found in app/src/player/controller.ts."); process.exit(1); }

const STUB = /^node:|\/folder-fs\.(js|ts)$|^@xmldom\/xmldom$/;
const stubSource = (path) => `const fail = () => { throw new Error(${JSON.stringify(`${path.replace(/^.*\//, "")} is not available in the browser`)}); };
export default new Proxy({}, { get: fail });
export const readFileSync = fail, writeFileSync = fail, existsSync = fail, mkdirSync = fail, statSync = fail, readdirSync = fail, rmSync = fail, cpSync = fail,
  mkdtempSync = fail, basename = fail, dirname = fail, join = fail, relative = fail, resolve = fail, extname = fail, sep = "/", fileURLToPath = fail, pathToFileURL = fail,
  tmpdir = fail, homedir = fail, readFolderFiles = fail, writeFolderFiles = fail, DOMParser = fail, XMLSerializer = fail;`;
const stubs = {
  name: "potoru-src",
  setup(context) {
    context.onResolve({ filter: /^@potoru-src\// }, (args) => ({ path: join(SOURCE, args.path.slice("@potoru-src/".length)) }));
    // The lean player sits next to the chunks, as built (see PLAY_DIST).
    context.onResolve({ filter: /^potoru-play$/ }, () => ({ path: "./play.js", external: true }));
    // TypeScript sources import siblings as `./x.js`: resolve them to the `.ts` file.
    context.onResolve({ filter: /^\.\.?\/.*\.js$/ }, (args) => {
      if (!args.importer.startsWith(SOURCE)) return undefined;
      const ts = resolve(args.resolveDir, args.path.replace(/\.js$/, ".ts"));
      if (STUB.test(ts)) return { path: ts.split("/").pop().replace(/\.ts$/, ""), namespace: "node-stub" };
      return existsSync(ts) ? { path: ts } : undefined;
    });
    context.onResolve({ filter: STUB }, (args) => ({ path: args.path.replace(/^node:/, "node_"), namespace: "node-stub" }));
    context.onLoad({ filter: /.*/, namespace: "node-stub" }, (args) => ({ contents: stubSource(args.path), loader: "js" }));
  }
};
const common = { bundle: true, minify: true, target: "es2022", platform: "browser", legalComments: "none", charset: "utf8", logLevel: "warning", define: { __POTORU_BUNDLE__: "true", __POTORU_PLAYER_VERSION__: JSON.stringify(PLAYER_VERSION) }, plugins: [stubs] };

// 1. Entry and lazy chunks.
await build({
  ...common,
  absWorkingDir: RUNTIME,
  entryPoints: { "potoru.bundle": join(RUNTIME, "entry.mjs") },
  outdir: RUNTIME,
  format: "esm",
  splitting: true,
  entryNames: "[name]",
  chunkNames: "[name]-[hash]",
  outExtension: { ".js": ".mjs" },
  loader: { ".wasm": "file" }
});
// 2. The music Worker.
await build({ ...common, entryPoints: [join(SOURCE, "app/src/runtime/score-worker.ts")], outfile: join(RUNTIME, "score-worker.js"), format: "esm" });
// 3. The Script-mode sandbox.
await build({ ...common, entryPoints: [join(RUNTIME, "src/sandbox-api.mjs")], outfile: join(RUNTIME, "potoru-authoring-sandbox.js"), format: "iife", globalName: "PotoruAuthoring", keepNames: true, define: { ...common.define, "import.meta.url": JSON.stringify("https://sandbox.invalid/") } });
// 4. Engines and fonts.
for (const engine of ["action", "render", "score", "g2p"]) {
  const file = `potoru_${engine}_wasm_bg.wasm`;
  copyFileSync(join(SOURCE, `app/src/runtime/${engine}-wasm-pkg`, file), join(RUNTIME, file));
}
mkdirSync(join(RUNTIME, "fonts"), { recursive: true });
for (const file of readdirSync(join(SOURCE, "app/public/fonts"))) copyFileSync(join(SOURCE, "app/public/fonts", file), join(RUNTIME, "fonts", file));

// 4b. The lean player: play.js and every file it reaches, copied as built. Its engines and fonts are
// the ones copied above (same names, next to it), so they must be the very same bytes.
const playFiles = new Set();
const reach = ["play.js"];
while (reach.length) {
  const name = reach.pop();
  if (playFiles.has(name)) continue;
  const text = readFileSync(join(PLAY_DIST, name), "utf8");
  playFiles.add(name);
  for (const [, path] of text.matchAll(/(?:from|import)\s*\(?\s*["']\.\/([^"']+\.js)["']/g)) reach.push(path);
  for (const [, path] of text.matchAll(/new URL\(\s*["']([^"']+)["']/g)) {
    if (path.endsWith(".wasm")) {
      if (sha256(join(PLAY_DIST, path)) !== sha256(join(RUNTIME, path))) {
        console.error(`play.js wants ${path} from another build than app/src/runtime: rebuild packages/web from the same source.`);
        process.exit(1);
      }
    } else if (path.startsWith("assets/")) playFiles.add(path);
  }
}
for (const name of playFiles) {
  mkdirSync(dirname(join(RUNTIME, name)), { recursive: true });
  copyFileSync(join(PLAY_DIST, name), join(RUNTIME, name));
}

// No local path may ship (home folders, temp folders, toolchains, this machine's checkouts).
const forbidden = [/\/Users\/[^/\s"']+/, /\/home\/[a-z][^/\s"']*/, /[A-Z]:\\Users\\/, /\/private\/(?:tmp|var)\//, /\/var\/folders\//, /\.cargo\/registry/, /rustup\/toolchains/];
const files = (folder) => readdirSync(folder).flatMap((name) => { const path = join(folder, name); return statSync(path).isDirectory() ? files(path) : [path]; });
for (const path of files(RUNTIME)) {
  const text = readFileSync(path, "latin1");
  if (text.includes(SOURCE) || text.includes(ZEBFLOW) || forbidden.some((pattern) => pattern.test(text))) {
    console.error(`A built file names a local path: ${relative(LIBRARY, path)}`);
    process.exit(1);
  }
}

// 5. Checksums, manifest, embedded list.
const listed = files(LIBRARY).map((path) => relative(LIBRARY, path)).filter((path) => path !== "FILES-SHA256.txt" && !path.startsWith("build/")).sort();
writeFileSync(join(LIBRARY, "FILES-SHA256.txt"), listed.map((path) => `${sha256(join(LIBRARY, path))}  ${path}`).join("\n") + "\n");

const bundle = join(RUNTIME, "potoru.bundle.mjs");
const manifestPath = join(LIBRARY, "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const version = manifest.spec.versions["0.1"];
version.size_bytes = statSync(bundle).size;
version.integrity = `sha256:${sha256(bundle)}`;
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

const embeddedPath = join(ZEBFLOW, "src/platform/web/embedded.rs");
const embedded = readFileSync(embeddedPath, "utf8");
// Every file of the package is served, the checksum list too.
const served = [...listed, "FILES-SHA256.txt"].sort();
const entries = served.map((path) => `    EmbeddedAsset {\n        path: "zeb/potoru/${path}",\n        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/${path}"),\n    },\n`).join("");
const block = /(    \/\/ zeb\/potoru: every file of the package\.[^\n]*\n(?:    \/\/[^\n]*\n)*)((?:    EmbeddedAsset \{\n        path: "zeb\/potoru\/[^"]+",\n        bytes: include_bytes!\("[^"]+"\),\n    \},\n)+)/;
if (!block.test(embedded)) { console.error("The zeb/potoru block in embedded.rs was not found; update it by hand."); process.exit(1); }
writeFileSync(embeddedPath, embedded.replace(block, (_all, comment) => comment + entries));

console.log(`zeb/potoru built: ${served.length} served files, entry ${version.size_bytes} bytes, ${version.integrity}`);
