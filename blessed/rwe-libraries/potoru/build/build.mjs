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

// ---- clean: everything generated (sources, wrappers, package.json and legal files stay) ----
for (const name of readdirSync(RUNTIME)) {
  const path = join(RUNTIME, name);
  if (name === "entry.mjs" || name === "src" || name === "package.json") continue;
  if (/\.(mjs|js|wasm)$/.test(name) || name === "fonts") rmSync(path, { recursive: true, force: true });
}

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
const common = { bundle: true, minify: true, target: "es2022", platform: "browser", legalComments: "none", charset: "utf8", logLevel: "warning", define: { __POTORU_BUNDLE__: "true" }, plugins: [stubs] };

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
const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
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
