import { h, Fragment, createContext } from "npm:preact";
import renderToString from "npm:preact-render-to-string";
import { bundle, transpile } from "jsr:@deno/emit";
import {
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "npm:preact/hooks";

const enc = new TextEncoder();
const MAX_HTML_BYTES = 5_000_000;
const MAX_JS_BYTES = 5_000_000;

const PageStateContext = createContext(null);

function createUsePageState() {
  return function usePageState(initial = {}) {
    const context = useContext(PageStateContext);
    if (context) {
      return context;
    }

    const [state, setState] = useState(initial || {});
    const setPageState = (patch) => {
      if (typeof patch === "function") {
        setState((prev) => {
          const next = patch(prev || {});
          return { ...(prev || {}), ...(next || {}) };
        });
        return;
      }
      setState((prev) => ({ ...(prev || {}), ...(patch || {}) }));
    };
    return { ...(state || {}), setPageState };
  };
}

function installGlobals() {
  globalThis.h = h;
  globalThis.Fragment = Fragment;
  globalThis.React = { createElement: h, Fragment };
  globalThis.useState = useState;
  globalThis.useEffect = useEffect;
  globalThis.useRef = useRef;
  globalThis.useMemo = useMemo;
  globalThis.usePageState = createUsePageState();
  globalThis.useNavigate = function useNavigate() {
    // During SSR there is no navigation — return a no-op.
    return function (_href) {};
  };
  globalThis.Link = function Link({ href, children, ...props }) {
    // During SSR render as a plain anchor (SEO-friendly).
    return h("a", { href, ...props }, children);
  };
  globalThis.cx = function cx(...parts) {
    return parts.filter(Boolean).join(" ");
  };
}

function wrapWithPageState(Page, input) {
  function Root(props) {
    const [state, setState] = useState({});
    const setPageState = (patch) => {
      if (typeof patch === "function") {
        setState((prev) => {
          const next = patch(prev || {});
          return { ...(prev || {}), ...(next || {}) };
        });
        return;
      }
      setState((prev) => ({ ...(prev || {}), ...(patch || {}) }));
    };
    const value = useMemo(() => ({ ...(state || {}), setPageState }), [state]);
    return h(
      PageStateContext.Provider,
      { value },
      h(Page, props),
    );
  }

  return h(Root, input || {});
}

function writeLine(obj) {
  Deno.stdout.writeSync(enc.encode(`${JSON.stringify(obj)}\n`));
}

function withTimeout(promise, timeoutMs, code) {
  const ms = Number(timeoutMs || 0);
  if (!Number.isFinite(ms) || ms <= 0) return promise;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      setTimeout(() => reject(new Error(code)), ms);
    }),
  ]);
}

async function handleRender(req) {
  const tempFile = `${await Deno.makeTempFile({ suffix: ".tsx", prefix: "rwe-" })}`;
  try {
    const source = String(req.module_source || "");
    await Deno.writeTextFile(tempFile, source);
    installGlobals();
    // Expose server context as global `ctx` so `export const page` can use
    // ctx.seo.title, ctx.anything — evaluated at module load time.
    globalThis.ctx = req.ctx || {};

    const result = await withTimeout(
      (async () => {
        const mod = await import(`file://${tempFile}?v=${req.id}`);
        const Page = mod?.default;
        if (typeof Page !== "function") {
          throw new Error("default export is not a function component");
        }
        const html = renderToString(wrapWithPageState(Page, req.ctx || {}));
        const pageConfig = mod.page || null;
        return { html, pageConfig };
      })(),
      req.timeout_ms,
      "RWE_DENO_TIMEOUT",
    );
    if (enc.encode(result.html).length > MAX_HTML_BYTES) {
      throw new Error("RWE_DENO_HTML_TOO_LARGE");
    }
    writeLine({ id: req.id, ok: true, html: result.html, page_config: result.pageConfig });
  } catch (err) {
    writeLine({
      id: req.id,
      ok: false,
      error: String(err?.stack || err?.message || err || "render failed"),
    });
  } finally {
    try {
      await Deno.remove(tempFile);
    } catch {
      // no-op
    }
  }
}

async function handleTranspile(req) {
  const tempFile = `${await Deno.makeTempFile({ suffix: ".tsx", prefix: "rwe-client-" })}`;
  try {
    const source = String(req.module_source || "");
    await Deno.writeTextFile(tempFile, source);
    const entryUrl = new URL(`file://${tempFile}`);
    let code = "";

    try {
      const out = await withTimeout(
        bundle(entryUrl),
        req.timeout_ms,
        "RWE_DENO_TIMEOUT",
      );
      code = String(out?.code || "");
    } catch {
      // Fallback to transpile-only mode if bundling fails.
      const files = await withTimeout(
        transpile(entryUrl, {
          compilerOptions: {
            jsx: "react-jsx",
            jsxImportSource: "preact",
          },
        }),
        req.timeout_ms,
        "RWE_DENO_TIMEOUT",
      );
      code = String(files.get(entryUrl.toString()) || "");
    }

    if (!code) {
      throw new Error("RWE_DENO_TRANSPILE_EMPTY");
    }
    if (enc.encode(code).length > MAX_JS_BYTES) {
      throw new Error("RWE_DENO_JS_TOO_LARGE");
    }
    writeLine({ id: req.id, ok: true, js: code });
  } catch (err) {
    writeLine({
      id: req.id,
      ok: false,
      error: String(err?.stack || err?.message || err || "transpile failed"),
    });
  } finally {
    try {
      await Deno.remove(tempFile);
    } catch {
      // no-op
    }
  }
}

async function main() {
  const reader = Deno.stdin.readable.pipeThrough(new TextDecoderStream()).getReader();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += value;

    while (true) {
      const idx = buffer.indexOf("\n");
      if (idx === -1) break;
      const line = buffer.slice(0, idx).trim();
      buffer = buffer.slice(idx + 1);
      if (!line) continue;

      let req;
      try {
        req = JSON.parse(line);
      } catch (e) {
        writeLine({ id: 0, ok: false, error: `invalid_json: ${e}` });
        continue;
      }

      if (req.op === "render_ssr") {
        await handleRender(req);
        continue;
      }
      if (req.op === "transpile_client") {
        await handleTranspile(req);
        continue;
      }

      writeLine({ id: req.id || 0, ok: false, error: "unsupported_op" });
    }
  }
}

main().catch((err) => {
  writeLine({ id: 0, ok: false, error: String(err?.stack || err || "fatal") });
  Deno.exit(1);
});
