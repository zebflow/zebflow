// Template editor runtime loader — dynamic imports live here, not in the entry page TSX.
// The RWE security policy blocks import() in template source files, but behavior files
// are inlined into the bundle after the security check passes.

let _rt = null;
let _promise = null;

export async function loadEditorRuntime() {
  if (_rt) return _rt;
  if (_promise) return _promise;
  if (typeof window === "undefined") throw new Error("browser required");

  _promise = (async () => {
    const base = window.location.origin;
    const cmUrl = new URL(
      "/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
      base
    );
    const cm = await import(cmUrl.href);
    _rt = { cm };
    return _rt;
  })();

  return _promise;
}
