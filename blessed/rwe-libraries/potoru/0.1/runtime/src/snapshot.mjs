/**
 * zeb/potoru — snapshot part (loaded on demand by entry.mjs; bundled into `snapshot-*.mjs` chunks).
 *
 * One still frame of a story on a canvas: fetch → decode → resolve linked libraries and font
 * aliases → evaluate the frame at `time` → draw with Potoru's Canvas renderer. No `<potoru-player>`
 * element, no clock, no sound, and never the action (interaction) or score engines: an interactive
 * story is drawn as its timeline shows it at `time`, before any input. Path guides (render wasm),
 * effects and generators load only when the story declares them, as in the player.
 * It draws again when `src` or `time` change, and when the box resizes or an image/font arrives.
 * `@potoru-src/` is resolved by build/build.mjs to $POTORU_SRC (the Potoru `designer/e3` folder).
 */
import { decodePoto } from "@potoru-src/app/src/runtime/container.ts";
import { evaluateRuntimeFrame, runtimeDurationSeconds } from "@potoru-src/app/src/runtime/evaluator.ts";
import { isStoryPackage, sceneRuntime } from "@potoru-src/app/src/runtime/story-package.ts";
import { currentPathCore } from "@potoru-src/app/src/runtime/path-core.ts";
import { packageCameras, viewStageSize, withCamera } from "@potoru-src/app/src/runtime/camera-view.ts";
import { storySceneIds } from "@potoru-src/app/src/runtime/story-package.ts";
import { CanvasSurface } from "@potoru-src/app/src/player/surfaces.ts";
import { browserCanvasHost } from "@potoru-src/app/src/player/canvas-host.ts";

const SHADOW_STYLE = ":host{display:block;position:relative;overflow:hidden}canvas{display:block;width:100%;height:100%}";

async function fetchBytes(url) {
  const response = await fetch(url, { mode: "cors", credentials: "omit", redirect: "follow" });
  if (!response.ok) throw Object.assign(new Error(`Request for ${new URL(url).pathname.split("/").pop()} failed (${response.status}).`), { code: "fetch" });
  return new Uint8Array(await response.arrayBuffer());
}

/*
 * `builtinFontLoader` and `resolvePackage` are adapted from $POTORU_SRC/app/src/player/controller.ts
 * (Apache-2.0): importing the controller would pull the whole player (clock, sound, sessions)
 * into the snapshot. Same rules: built-in fonts from `fonts/` next to this chunk (byte length
 * checked), libraries from the host list then the story's safe relative hrefs, strict lock.
 */
function builtinFontLoader() {
  const bases = [new URL("fonts/", import.meta.url).href, ...(globalThis.document ? [new URL("fonts/", globalThis.document.baseURI).href] : [])];
  return async (entry) => {
    for (const base of bases) {
      try {
        const response = await fetch(new URL(entry.file, base).href, { mode: "cors", credentials: "omit" });
        const bytes = response.ok ? new Uint8Array(await response.arrayBuffer()) : undefined;
        if (bytes?.byteLength === entry.byteLength) return bytes;
      } catch { /* next base */ }
    }
    return undefined;
  };
}

const isSafeRelativeHref = (href) => href.length > 0 && href.length <= 200 && /^[A-Za-z0-9._-]+(?:\/[A-Za-z0-9._-]+)*$/.test(href) && !href.split("/").some((segment) => segment === "." || segment === "..") && /\.(poto|potolib)$/i.test(href);

async function resolvePackage(runtime, { libraries, baseUrl }) {
  const scenes = isStoryPackage(runtime) ? storySceneIds(runtime).map((id) => sceneRuntime(runtime, id)) : [runtime];
  if (!scenes.some((scene) => scene.imports?.length || scene.fontAliases?.length)) return runtime;
  const [{ decodePotoLibrary }, { resolveWithPolicy, packageImports }, { resolveFontAliases }] = await Promise.all([
    import("@potoru-src/app/src/runtime/library.ts"), import("@potoru-src/app/src/runtime/resolver.ts"), import("@potoru-src/app/src/runtime/font-aliases.ts")
  ]);
  const installed = [];
  const add = async (source) => {
    const bytes = source instanceof Uint8Array ? source : await fetchBytes(new URL(String(source), baseUrl ?? globalThis.location?.href).href);
    const library = await decodePotoLibrary(bytes);
    if (!installed.some((known) => known.contentHash === library.contentHash)) installed.push(library);
  };
  await Promise.all(libraries.map(add));
  if (baseUrl) {
    for (const entry of packageImports(runtime)) {
      if (installed.some((library) => library.manifest.slug === entry.slug) || !entry.href || !isSafeRelativeHref(entry.href)) continue;
      await add(new URL(entry.href, baseUrl).href).catch(() => undefined);
    }
  }
  const aliases = [...new Set(scenes.flatMap((scene) => scene.fontAliases ?? []))];
  const fonts = aliases.length ? await resolveFontAliases(aliases, { installed, builtin: builtinFontLoader() }) : { fonts: [], diagnostics: [] };
  const errors = fonts.diagnostics.filter((entry) => entry.severity === "error");
  if (errors.length) throw Object.assign(new Error(errors.map((entry) => entry.message).join("\n")), { code: "needs-library" });
  return resolveWithPolicy(runtime, installed, { policy: "strict", fonts: fonts.fonts }).runtime;
}

/** The optional drawing families the story declares (never the action or score engines). */
async function loadDrawingEngines(pkg) {
  const scenes = [pkg, ...(pkg.scenes ?? []).map((scene) => scene.runtime)];
  const capabilities = new Set(scenes.flatMap((scene) => scene.capabilities ?? []));
  const loads = [];
  if (capabilities.has("path-guides") && !currentPathCore()) loads.push(import("@potoru-src/app/src/runtime/path-core-loader.ts").then(({ loadPathCoreFor }) => loadPathCoreFor(pkg)));
  if (capabilities.has("effects")) loads.push(import("@potoru-src/app/src/player/canvas-effects-registry.ts").then(({ loadCanvasEffects }) => loadCanvasEffects()));
  if (capabilities.has("generators")) loads.push(import("@potoru-src/app/src/runtime/generator-registry.ts").then(({ loadGenerators }) => loadGenerators()));
  await Promise.all(loads);
}

export function createSnapshot(host, config, { id, emit }) {
  const root = host.shadowRoot ?? host.attachShadow({ mode: "open" });
  root.textContent = "";
  const style = document.createElement("style");
  style.textContent = SHADOW_STYLE;
  const canvas = document.createElement("canvas");
  canvas.setAttribute("part", "snapshot");
  canvas.setAttribute("role", "img");
  root.append(style, canvas);
  let current = { ...config };
  let surface;
  let entry;        // the drawn Scene's runtime (camera applied)
  let animation = 0;
  let loadedKey = "";
  let loadId = 0;

  const makeSurface = () => {
    surface?.dispose();
    surface = new CanvasSurface(canvas, { host: browserCanvasHost(), fit: current.fit === "cover" || current.fit === "fill" ? current.fit : "contain" });
  };

  const draw = () => {
    if (!entry || !surface) return;
    const duration = runtimeDurationSeconds(entry, animation);
    const time = Math.min(Math.max(0, Number(current.time) || 0), Number.isFinite(duration) ? duration : Infinity);
    surface.present(evaluateRuntimeFrame(entry, time, animation));
    return { time, duration };
  };

  async function load() {
    const id_ = (loadId += 1);
    const libraries = current.libraries ?? [];
    const key = JSON.stringify([current.src, libraries, current.camera ?? null]);
    try {
      let bytes;
      let baseUrl;
      if (current.bytes instanceof Uint8Array) bytes = current.bytes;
      else {
        if (!current.src) return;
        baseUrl = new URL(String(current.src), globalThis.location?.href).href;
        bytes = await fetchBytes(baseUrl);
      }
      const decoded = await decodePoto(bytes, undefined, { fonts: builtinFontLoader() });
      const runtime = await resolvePackage(decoded.runtime, { libraries, baseUrl });
      const scene = isStoryPackage(runtime) ? sceneRuntime(runtime) : runtime;
      await loadDrawingEngines(runtime);
      if (id_ !== loadId) return;
      const camera = current.camera === "stage" ? null : current.camera;
      entry = camera !== undefined && scene.capabilities.includes("cameras") && (camera === null || packageCameras(scene).some((entry_) => entry_.id === camera)) ? withCamera(scene, camera) : scene;
      animation = entry.experience?.entry.animation ?? entry.defaultAnimation;
      loadedKey = key;
      if (!surface) makeSurface();
      const drawn = draw();
      const stage = viewStageSize(entry);
      if (!current.height && !current.aspect && stage) host.style.aspectRatio = `${stage.width} / ${stage.height}`;
      canvas.setAttribute("aria-label", entry.name || "Potoru story");
      emit("ready", { id, kind: "snapshot", time: drawn?.time ?? 0, duration: drawn?.duration ?? 0, name: entry.name, width: stage?.width, height: stage?.height, capabilities: entry.capabilities ?? [] });
    } catch (error) {
      if (id_ !== loadId) return;
      emit("error", { id, code: error?.code === "fetch" || error?.code === "needs-library" ? error.code : "decode", message: error instanceof Error ? error.message : String(error) });
    }
  }

  void load();

  return {
    id,
    kind: "snapshot",
    host,
    canvas,
    /** Draws the frame at another time (seconds) without reloading. */
    seek(seconds) { current = { ...current, time: Number(seconds) || 0 }; draw(); },
    /** Loads another story: a URL, or `.poto` bytes (Uint8Array). */
    load(src, options = {}) {
      current = { ...current, ...(src instanceof Uint8Array ? { bytes: src, src: "" } : { src: String(src), bytes: undefined }), ...(options.libraries ? { libraries: options.libraries } : {}) };
      return load();
    },
    currentTime: () => Math.max(0, Number(current.time) || 0),
    duration: () => (entry ? runtimeDurationSeconds(entry, animation) : 0),
    configure(next) {
      const merged = { ...next };
      const fitChanged = (merged.fit ?? "contain") !== (current.fit ?? "contain");
      current = { ...merged, ...(current.bytes && !merged.src ? { bytes: current.bytes } : {}) };
      if (fitChanged && surface) makeSurface();
      const key = JSON.stringify([current.src, current.libraries ?? [], current.camera ?? null]);
      if (key !== loadedKey && current.src) void load();
      else draw();
    },
    destroy() { loadId += 1; surface?.dispose(); root.textContent = ""; }
  };
}
