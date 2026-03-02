import { mountThreeScene } from "../../../threejs/0.1/runtime/threejs.bundle.mjs";

function resolveGlobal(name) {
  return typeof globalThis !== "undefined" ? globalThis[name] : undefined;
}

function readConfig(host, options) {
  if (options && typeof options === "object") {
    return options;
  }
  const raw = host.getAttribute("data-config") || "";
  if (!raw.trim()) {
    return {};
  }
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === "object") {
      return parsed;
    }
  } catch (_err) {
    // ignore parse error and continue with defaults
  }
  return {};
}

export async function mountVrmViewer(host, options = {}) {
  if (!(host instanceof Element)) {
    throw new Error("zeb/threejs-vrm: host element is required");
  }

  const cfg = readConfig(host, options);
  const runtime = mountThreeScene(host, cfg);

  const THREE = runtime.THREE;
  const GLTFLoader = cfg.GLTFLoader || resolveGlobal("GLTFLoader");
  const VRM = cfg.VRM || resolveGlobal("VRM");
  const VRMUtils = cfg.VRMUtils || resolveGlobal("VRMUtils");

  if (!GLTFLoader || !VRM || !VRMUtils) {
    const note = document.createElement("div");
    note.style.position = "absolute";
    note.style.left = "12px";
    note.style.bottom = "12px";
    note.style.padding = "6px 8px";
    note.style.fontSize = "12px";
    note.style.background = "rgba(0,0,0,0.55)";
    note.style.color = "#dbeafe";
    note.style.borderRadius = "8px";
    note.textContent = "VRM loaders missing. Provide global GLTFLoader + VRM + VRMUtils.";
    host.appendChild(note);
    return {
      ...runtime,
      model: null,
      destroy() {
        note.remove();
        runtime.destroy();
      },
    };
  }

  const loader = new GLTFLoader();
  const modelUrl = String(cfg.model_url || cfg.modelUrl || "").trim();
  let current = null;

  if (modelUrl) {
    const gltf = await loader.loadAsync(modelUrl);
    VRMUtils.removeUnnecessaryVertices(gltf.scene);
    VRMUtils.removeUnnecessaryJoints(gltf.scene);
    const vrmModel = await VRM.from(gltf);
    runtime.scene.add(vrmModel.scene);
    current = vrmModel;

    const prevDestroy = runtime.destroy.bind(runtime);
    runtime.destroy = () => {
      runtime.scene.remove(vrmModel.scene);
      prevDestroy();
    };
  }

  return {
    ...runtime,
    THREE,
    model: current,
  };
}

export const vrm = {
  mountVrmViewer,
};
