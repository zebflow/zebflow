/**
 * zeb/potoru — the authoring API as it runs inside the Script-mode sandbox (built by build/build.mjs into
 * `potoru-authoring-sandbox.js`, an IIFE that sets `globalThis.PotoruAuthoring`).
 *
 * The compiler fetches that file as text and inlines it into a sandboxed `srcdoc` iframe (opaque
 * origin, CSP `default-src 'none'; script-src 'unsafe-inline'`), so the script it runs can reach
 * nothing but this API. Node-only helpers of the API (save / load to disk) are stubbed at build time.
 */
export * from "@potoru-src/authoring/src/index.ts";
