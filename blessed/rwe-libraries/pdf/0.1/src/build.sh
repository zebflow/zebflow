#!/usr/bin/env bash
set -e

OUT=dist/pdf-creation.mjs
OUT_MIN=dist/pdf-creation.min.mjs

mkdir -p dist

echo "Bundling..."
npx --yes esbuild index.mjs \
  --bundle \
  --format=esm \
  --outfile="$OUT"

echo "Minifying..."
npx --yes esbuild index.mjs \
  --bundle \
  --format=esm \
  --minify \
  --outfile="$OUT_MIN"

echo ""
echo "--- Build output ---"
for f in "$OUT" "$OUT_MIN"; do
  size=$(wc -c < "$f")
  kb=$(echo "scale=1; $size / 1024" | bc)
  gzsize=$(gzip -c "$f" | wc -c)
  gzkb=$(echo "scale=1; $gzsize / 1024" | bc)
  echo "  $f"
  echo "    raw:  ${kb} KB (${size} bytes)"
  echo "    gzip: ${gzkb} KB (${gzsize} bytes)"
done
