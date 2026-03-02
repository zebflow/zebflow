## MILESTONE 1

How React + Vite keeps only needed code:

Dev mode: Vite serves native ESM modules directly (no giant full bundle).
Build mode: Vite uses Rollup (with esbuild/terser) to build a module graph.
Tree-shaking: only imported/used exports are kept, unused exports are dropped.
Code-splitting: dynamic imports/route splits create separate chunks, so pages load only their chunks.
Minification + dead-code elimination remove unreachable branches and shrink output.