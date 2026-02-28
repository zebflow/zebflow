const [scriptPath, configPath, inputPath] = Deno.args;
if (!scriptPath || !configPath || !inputPath) {
  console.error("usage: secure_js_runner.js <script> <config.json> <input.json>");
  Deno.exit(2);
}

const config = JSON.parse(await Deno.readTextFile(configPath));
const input = JSON.parse(await Deno.readTextFile(inputPath));
const nativeFetch = globalThis.fetch.bind(globalThis);

const allowedExternalFetchHosts = new Set(
  (config.allowList?.externalFetchHosts || [])
    .map((h) => String(h).trim().toLowerCase())
    .filter((h) => h.length > 0),
);
const localFetchRoot = String(config.localFetchRoot || ".");

const deadline = performance.now() + (Number(config.timeoutMs) || 100);
let opsLeft = Number(config.maxOps) || 1_000_000;

function budgetTick() {
  opsLeft -= 1;
  if (opsLeft < 0) {
    throw new Error("DenoSandboxError: op budget exceeded");
  }
  if (performance.now() > deadline) {
    throw new Error("DenoSandboxError: timeout exceeded");
  }
}

const blocked = (name) => () => {
  throw new Error(`DenoSandboxError: ${name} is disabled`);
};

function fetchInputUrl(input) {
  if (typeof input === "string") return input.trim();
  if (input && typeof input.url === "string") return String(input.url).trim();
  throw new Error("DenoSandboxError: unsupported fetch input");
}

function looksLocalPath(url) {
  return url.startsWith("/");
}

function allowedExternalHost(url) {
  const host = url.hostname.toLowerCase();
  const explicitPort = url.port ? `${host}:${url.port}` : "";
  const defaultPort = url.protocol === "https:"
    ? `${host}:443`
    : url.protocol === "http:"
    ? `${host}:80`
    : "";
  return (
    allowedExternalFetchHosts.has(host) ||
    (explicitPort && allowedExternalFetchHosts.has(explicitPort)) ||
    (defaultPort && allowedExternalFetchHosts.has(defaultPort))
  );
}

function guessContentType(path) {
  const p = path.toLowerCase();
  if (p.endsWith(".json")) return "application/json";
  if (p.endsWith(".html")) return "text/html; charset=utf-8";
  if (p.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (p.endsWith(".css")) return "text/css; charset=utf-8";
  if (p.endsWith(".txt") || p.endsWith(".md")) return "text/plain; charset=utf-8";
  return "application/octet-stream";
}

async function localFetch(pathname) {
  const rel = decodeURIComponent(pathname).replace(/^\/+/, "");
  if (!rel || rel.includes("\0") || rel.split("/").includes("..")) {
    return new Response("Not Found", { status: 404 });
  }
  const candidate = `${localFetchRoot}/${rel}`;
  const data = await Deno.readFile(candidate).catch(() => null);
  if (!data) return new Response("Not Found", { status: 404 });
  return new Response(data, {
    status: 200,
    headers: { "content-type": guessContentType(candidate) },
  });
}

async function secureFetch(input, init) {
  budgetTick();
  const raw = fetchInputUrl(input);

  if (looksLocalPath(raw)) return localFetch(raw);

  let parsed;
  try {
    parsed = new URL(raw);
  } catch {
    throw new Error(`DenoSandboxError: unsupported fetch url '${raw}'`);
  }

  if (parsed.protocol === "http:" || parsed.protocol === "https:") {
    if (!allowedExternalHost(parsed)) {
      throw new Error(
        `DenoSandboxError: external fetch denied for ${parsed.hostname}. add it to allowList.externalFetchHosts`,
      );
    }
    return nativeFetch(input, init);
  }

  throw new Error(`DenoSandboxError: fetch protocol denied (${parsed.protocol})`);
}

Object.defineProperty(globalThis, "fetch", {
  value: secureFetch,
  writable: false,
  configurable: false,
});

if (!config.dangerZone?.allowDynamicCode) {
  Object.defineProperty(globalThis, "eval", {
    value: blocked("eval"),
    writable: false,
    configurable: false,
  });
  Object.defineProperty(globalThis, "Function", {
    value: blocked("Function"),
    writable: false,
    configurable: false,
  });
}

if (!config.dangerZone?.allowTimers) {
  Object.defineProperty(globalThis, "setTimeout", {
    value: blocked("setTimeout"),
    writable: false,
    configurable: false,
  });
  Object.defineProperty(globalThis, "setInterval", {
    value: blocked("setInterval"),
    writable: false,
    configurable: false,
  });
}

const allowedCaps = new Set(config.capabilities || []);
function buildCapabilities() {
  const n = {};
  if (allowedCaps.has("time.now")) {
    n.time = Object.freeze({ now: () => Date.now() });
  }
  if (allowedCaps.has("math.imul") || allowedCaps.has("math.u32")) {
    n.math = Object.freeze({
      imul: (a, b) => Math.imul(a | 0, b | 0),
      u32: (v) => Number(v) >>> 0,
    });
  }
  return Object.freeze(n);
}

Object.defineProperty(globalThis, "__tj_tick", {
  value: budgetTick,
  writable: false,
  configurable: false,
});

const runtimeCtx = Object.freeze({
  tick: budgetTick,
  budget: Object.freeze({
    timeoutMs: Number(config.timeoutMs) || 100,
    maxOps: Number(config.maxOps) || 1_000_000,
  }),
  fetchPolicy: Object.freeze({
    localFetchRoot,
    externalHosts: Array.from(allowedExternalFetchHosts.values()),
  }),
});

const capabilities = buildCapabilities();

try {
  const scriptUrl = new URL(scriptPath, `file://${Deno.cwd()}/`).href;
  const mod = await import(scriptUrl);
  const entry = mod.default || mod.run;

  if (typeof entry !== "function") {
    throw new Error("DenoSandboxError: script must export default or run function");
  }

  const result = await entry(input, capabilities, runtimeCtx);
  console.log(JSON.stringify({ ok: true, result }));
} catch (err) {
  console.log(
    JSON.stringify({
      ok: false,
      error: String(err?.message || err),
      name: String(err?.name || "Error"),
    }),
  );
  Deno.exit(1);
}
