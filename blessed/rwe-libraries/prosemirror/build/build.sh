#!/usr/bin/env bash
# Builds the zeb/prosemirror runtime bundle from entry.mjs. Deterministic for a
# given package-lock.json; the manifest's integrity hash is updated to match.
set -euo pipefail
cd "$(dirname "$0")"
npm ci --no-audit --no-fund --silent
OUT=../0.1/runtime/prosemirror.bundle.mjs
node_modules/.bin/esbuild entry.mjs --bundle --format=esm --target=es2020 --minify --legal-comments=none --outfile="$OUT"
SIZE=$(wc -c < "$OUT" | tr -d ' ')
SHA=$(shasum -a 256 "$OUT" | cut -d' ' -f1)
node - "$SIZE" "$SHA" <<'JS'
const fs = require("fs");
const [size, sha] = process.argv.slice(2);
const p = "../manifest.json";
const m = JSON.parse(fs.readFileSync(p, "utf8"));
for (const v of Object.values(m.spec.versions)) { v.size_bytes = Number(size); v.integrity = `sha256:${sha}`; }
fs.writeFileSync(p, JSON.stringify(m, null, 2) + "\n");
JS
echo "built $OUT ($SIZE bytes, sha256:$SHA)"
