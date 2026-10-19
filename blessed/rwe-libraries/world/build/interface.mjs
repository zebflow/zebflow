#!/usr/bin/env node
/**
 * The interface guard: Zebflow's wrapper (src/react.ts) must have exactly the interface of aslnut's
 * `aslnut/react` (WorldProps both ways, World, useWorld, useElement, useWorldEvent, ViewChange, and the same names).
 * build.mjs runs it before bundling; on its own it checks a copy too:
 *
 *   ASLNUT_SRC=<aslnut checkout> node blessed/rwe-libraries/world/build/interface.mjs [wrapper.ts]
 *
 * It type-checks build/interface-check.ts with aslnut's own TypeScript (ASLNUT_SRC/node_modules/typescript) and
 * aslnut's compiler options. "zeb/react" is typed as React (Zeb React mirrors React's API); "@zeb-world/react" is the
 * wrapper. The generated tsconfig lives in a temporary folder and is removed afterwards.
 */
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));

/** true when the wrapper's interface is aslnut's; otherwise prints what differs and returns false */
export function checkInterface(source, wrapper = join(HERE, "../src/react.ts")) {
  const pkg = join(source, "packages/aslnut");
  const tsc = join(source, "node_modules/typescript/bin/tsc");
  if (!existsSync(tsc)) { console.error("TypeScript was not found in ASLNUT_SRC/node_modules; run `npm ci --ignore-scripts` there."); return false; }
  const dir = mkdtempSync(join(tmpdir(), "zeb-world-interface-"));
  try {
    const config = join(dir, "tsconfig.json");
    writeFileSync(config, JSON.stringify({
      extends: join(pkg, "tsconfig.json"),
      compilerOptions: {
        noEmit: true,
        rewriteRelativeImportExtensions: false,
        typeRoots: [join(source, "node_modules/@types"), join(source, "node_modules")],
        paths: {
          "@aslnut-src/*": [join(pkg, "src/*")],
          "@zeb-world/react": [resolve(wrapper)],
          "zeb/react": [join(source, "node_modules/@types/react/index.d.ts")],
        },
      },
      include: [],
      files: [join(HERE, "interface-check.ts")],
    }));
    const run = spawnSync(process.execPath, [tsc, "-p", config, "--pretty", "false"], { encoding: "utf8" });
    if (run.status === 0) return true;
    // Paths are shown relative to the two checkouts, never as this machine's absolute paths.
    const shown = `${run.stdout}${run.stderr}`.split(`${source}/`).join("<aslnut>/").split(`${resolve(HERE, "..")}/`).join("<zeb/world>/").split(dir).join("<tmp>");
    console.error(`zeb/world's wrapper drifted from aslnut/react:\n${shown.trim()}`);
    return false;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const source = process.env.ASLNUT_SRC && resolve(process.env.ASLNUT_SRC);
  if (!source) { console.error("Set ASLNUT_SRC to an aslnut checkout."); process.exit(1); }
  const ok = checkInterface(source, process.argv[2]);
  console.log(ok ? "zeb/world's wrapper has aslnut/react's interface." : "Interface check failed.");
  process.exit(ok ? 0 : 1);
}
