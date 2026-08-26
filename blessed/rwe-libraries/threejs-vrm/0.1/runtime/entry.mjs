/**
 * zeb/threejs-vrm 0.1 — VRM avatar viewer for RWE templates.
 *
 * ── FEATURES ────────────────────────────────────────────────────────────────
 *  • Offline: Three.js + @pixiv/three-vrm bundled inline — no CDN.
 *  • Supports VRM 0.x and VRM 1.0 avatar files.
 *  • MutationObserver auto-mounts [data-zeb-lib="threejs-vrm"] elements.
 *  • Preact VrmViewer component with useRef+useEffect (no hydration conflict).
 *  • Animation mixer, expression controls, spring bone simulation.
 *  • window.__zebVrm registry for imperative access.
 *
 * ── OFFLINE BUNDLE ───────────────────────────────────────────────────────────
 *  cd /tmp/zeb-threejs-vrm-build
 *  npm install
 *  node_modules/.bin/esbuild entry.mjs \
 *    --bundle --format=esm --minify \
 *    --outfile=threejs-vrm.bundle.mjs
 *  cp threejs-vrm.bundle.mjs libraries/zeb/threejs-vrm/0.1/runtime/
 *
 * ── QUICK REFERENCE ─────────────────────────────────────────────────────────
 *  TSX import:   import VrmViewer from "zeb/threejs-vrm";
 *  Imperative:   window.__zebVrm.get("viewer-id").playClip("idle")
 *  Event:        container.addEventListener("zeb:vrm:ready", e => e.detail.vrm)
 */

/* ── Bundled Three.js ── */
import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

/* ── Bundled @pixiv/three-vrm ── */
import {
  VRMLoaderPlugin,
  VRMUtils,
  VRMHumanBoneName,
  VRMExpressionPresetName,
} from "@pixiv/three-vrm";

/* ── Instance registry ── */
const _instances = new Map();

/* ── Default view config ── */
const DEFAULTS = {
  width:       "100%",
  height:      "400px",
  background:  "transparent",
  autoRotate:  false,
  cameraZ:     1.5,
};

/* ─────────────────────────────────────────────────────────────────────────────
 * mountVrmViewer — mount a VRM avatar viewer into a host element.
 *
 * Options:
 *   modelUrl    string   URL to .vrm file (required to show avatar)
 *   width       string   CSS width (default "100%")
 *   height      string   CSS height (default "400px")
 *   background  string   CSS color or "transparent" (default "transparent")
 *   autoRotate  boolean  auto-rotate the avatar (default false)
 *   cameraZ     number   camera distance (default 1.5)
 *   ambientColor string  ambient light colour (default "#ffffff")
 *   ambientIntensity number ambient intensity (default 0.6)
 *   dirColor    string   directional light colour (default "#ffffff")
 *   dirIntensity number  directional intensity (default 0.8)
 *
 * Returns:
 *   {
 *     scene, camera, renderer, clock, mixer, vrm,
 *     setExpression(name, value),
 *     playClip(animationClip),
 *     stopClip(),
 *     lookAt(targetVector3),
 *     destroy()
 *   }
 * ─────────────────────────────────────────────────────────────────────────── */
export async function mountVrmViewer(host, options = {}) {
  if (!(host instanceof Element)) throw new Error("zeb/threejs-vrm: host element is required");

  const cfg = { ...DEFAULTS, ...options };
  host.style.position = "relative";
  if (!host.style.width)  host.style.width  = cfg.width;
  if (!host.style.height) host.style.height = cfg.height;

  /* ── Renderer ── */
  const renderer = new THREE.WebGLRenderer({
    antialias: true,
    alpha:     cfg.background === "transparent",
  });

  const hostW = host.clientWidth  || 480;
  const hostH = host.clientHeight || 400;
  renderer.setPixelRatio(window.devicePixelRatio || 1);
  renderer.setSize(hostW, hostH);
  renderer.outputColorSpace = THREE.SRGBColorSpace;

  if (cfg.background !== "transparent") {
    renderer.setClearColor(new THREE.Color(cfg.background));
  }

  renderer.domElement.style.cssText = "position:absolute;top:0;left:0;width:100%;height:100%";
  host.appendChild(renderer.domElement);

  /* ── Scene ── */
  const scene = new THREE.Scene();

  /* Lighting */
  const ambient = new THREE.AmbientLight(
    cfg.ambientColor || "#ffffff",
    cfg.ambientIntensity ?? 0.6,
  );
  scene.add(ambient);

  const dir = new THREE.DirectionalLight(
    cfg.dirColor || "#ffffff",
    cfg.dirIntensity ?? 0.8,
  );
  dir.position.set(1, 2, 2);
  scene.add(dir);

  /* ── Camera ── */
  const camera = new THREE.PerspectiveCamera(35, hostW / hostH, 0.1, 100);
  camera.position.set(0, 1.0, cfg.cameraZ);

  /* ── Orbit controls ── */
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping  = true;
  controls.dampingFactor  = 0.05;
  controls.minDistance    = 0.3;
  controls.maxDistance    = 5.0;
  controls.target.set(0, 1.0, 0);
  controls.update();

  /* ── Resize observer ── */
  let animFrameId = null;
  const ro = new ResizeObserver(() => {
    const w = host.clientWidth  || 480;
    const h = host.clientHeight || 400;
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderer.setSize(w, h);
  });
  ro.observe(host);

  /* ── Clock ── */
  const clock = new THREE.Clock();

  /* ── VRM loading ── */
  let vrmModel = null;
  let mixer    = null;
  let currentAction = null;
  const lookAtTarget = new THREE.Vector3();
  let _useLookAt = false;

  const modelUrl = String(cfg.modelUrl || cfg.model_url || "").trim();

  if (modelUrl) {
    try {
      const loader = new GLTFLoader();
      loader.register(parser => new VRMLoaderPlugin(parser));
      const gltf = await loader.loadAsync(modelUrl);

      vrmModel = gltf.userData.vrm;
      if (vrmModel) {
        VRMUtils.removeUnnecessaryVertices(gltf.scene);
        VRMUtils.removeUnnecessaryJoints(gltf.scene);
        scene.add(vrmModel.scene);
        mixer = new THREE.AnimationMixer(vrmModel.scene);

        /* Auto-center camera on avatar bounding box */
        const box = new THREE.Box3().setFromObject(vrmModel.scene);
        const center = box.getCenter(new THREE.Vector3());
        const size   = box.getSize(new THREE.Vector3());
        camera.position.set(0, center.y, size.z * cfg.cameraZ + 0.5);
        camera.lookAt(center);
        lookAtTarget.copy(center);
        controls.target.copy(center);
        controls.update();
      }
    } catch (err) {
      console.warn("zeb/threejs-vrm: failed to load model:", err);
    }
  }

  /* ── Render loop ── */
  let rotY = 0;
  function animate() {
    animFrameId = requestAnimationFrame(animate);
    const delta = clock.getDelta();

    if (vrmModel) {
      if (cfg.autoRotate) {
        rotY += delta * 0.5;
        vrmModel.scene.rotation.y = rotY;
      }
      if (_useLookAt && vrmModel.lookAt) {
        vrmModel.lookAt.target = lookAtTarget;
      }
      vrmModel.update(delta);
    }

    if (mixer) mixer.update(delta);
    controls.update();
    renderer.render(scene, camera);
  }
  animate();

  /* ── Public API ── */
  const instance = {
    scene, camera, renderer, clock, mixer, controls,
    get vrm() { return vrmModel; },

    /** Set a VRM expression (blendshape) value 0–1. */
    setExpression(name, value) {
      if (!vrmModel?.expressionManager) return;
      vrmModel.expressionManager.setValue(name, value);
      vrmModel.expressionManager.update();
    },

    /** Play a THREE.AnimationClip on the avatar. */
    playClip(clip) {
      if (!mixer) return;
      if (currentAction) currentAction.stop();
      currentAction = mixer.clipAction(clip);
      currentAction.play();
    },

    /** Stop current animation. */
    stopClip() {
      if (currentAction) { currentAction.stop(); currentAction = null; }
    },

    /** Point the avatar's look-at target at a Vector3. */
    lookAt(target) {
      lookAtTarget.copy(target);
      _useLookAt = true;
    },

    destroy() {
      if (animFrameId) cancelAnimationFrame(animFrameId);
      ro.disconnect();
      controls.dispose();
      if (vrmModel) {
        scene.remove(vrmModel.scene);
        VRMUtils.deepDispose(vrmModel.scene);
      }
      renderer.dispose();
      renderer.domElement.remove();
    },
  };

  return instance;
}

/* ── Auto-mount system ──────────────────────────────────────────────────────
 * Watches for [data-zeb-lib="threejs-vrm"] in the DOM.
 * Parses data-config and calls mountVrmViewer.
 */
async function mountVrmCanvas(container) {
  if (container._zvMounted) return;
  container._zvMounted = true;

  let config = {};
  try { config = JSON.parse(container.dataset.config || "{}"); } catch {}

  if (!container.id) container.id = `zvrm-${Math.random().toString(36).slice(2, 8)}`;
  const instanceId = container.id;

  let instance = null;
  try {
    instance = await mountVrmViewer(container, config);
  } catch (err) {
    console.error("zeb/threejs-vrm: mount failed:", err);
    container._zvMounted = false;
    return;
  }

  _instances.set(instanceId, instance);

  container.dispatchEvent(new CustomEvent("zeb:vrm:ready", {
    bubbles: true,
    detail: { instance, vrm: instance.vrm, id: instanceId },
  }));
}

function destroyVrmCanvas(node) {
  if (node.nodeType !== 1) return;
  if (node._zvMounted && node.id) _instances.get(node.id)?.destroy();
  node.querySelectorAll?.("[data-zeb-lib='threejs-vrm']").forEach((el) => {
    if (el._zvMounted && el.id) _instances.get(el.id)?.destroy();
  });
}

const _observer = new MutationObserver((mutations) => {
  for (const mut of mutations) {
    for (const node of mut.addedNodes) {
      if (node.nodeType !== 1) continue;
      if (node.matches?.("[data-zeb-lib='threejs-vrm']")) mountVrmCanvas(node);
      node.querySelectorAll?.("[data-zeb-lib='threejs-vrm']").forEach(mountVrmCanvas);
    }
    for (const node of mut.removedNodes) destroyVrmCanvas(node);
  }
});

if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      _observer.observe(document.body, { childList: true, subtree: true });
      document.querySelectorAll("[data-zeb-lib='threejs-vrm']").forEach(mountVrmCanvas);
    });
  } else {
    _observer.observe(document.body, { childList: true, subtree: true });
    document.querySelectorAll("[data-zeb-lib='threejs-vrm']").forEach(mountVrmCanvas);
  }
}

/* ── Public surface ── */
if (typeof window !== "undefined") {
  window.__zebVrm = {
    get(id) { return _instances.get(id); },
    THREE,
    GLTFLoader,
    VRMLoaderPlugin,
    VRMUtils,
    VRMHumanBoneName,
    VRMExpressionPresetName,
    mountVrmViewer,
  };
}

/* ── Exports ── */

export const vrm = {
  mountVrmViewer,
  THREE,
  GLTFLoader,
  VRMLoaderPlugin,
  VRMUtils,
  VRMHumanBoneName,
  VRMExpressionPresetName,
};

/**
 * VrmViewer — Preact component for VRM avatar display in RWE templates.
 *
 * Uses useRef + useEffect to prevent Preact hydration conflicts.
 *
 * Props:
 *   modelUrl    string   URL to .vrm file
 *   height      string   CSS height (default "400px")
 *   background  string   "transparent" or CSS color (default "transparent")
 *   autoRotate  boolean  auto-rotate (default false)
 *   cameraZ     number   camera Z distance (default 1.5)
 *   id          string   container id for window.__zebVrm.get(id)
 *   className   string   Tailwind classes on container
 */
export function VrmViewer(props) {
  const _h         = globalThis.h;
  const _useRef    = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = {
    modelUrl:    props.modelUrl || props.model_url || "",
    height:      props.height || "400px",
    background:  props.background || "transparent",
    autoRotate:  props.autoRotate ?? false,
    cameraZ:     props.cameraZ   ?? 1.5,
    ambientColor:     props.ambientColor,
    ambientIntensity: props.ambientIntensity,
    dirColor:         props.dirColor,
    dirIntensity:     props.dirIntensity,
  };

  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const inner = document.createElement("div");
      inner.setAttribute("data-zeb-lib", "threejs-vrm");
      inner.setAttribute("data-config", JSON.stringify(config));
      if (props.id) inner.id = props.id;
      inner.style.width  = "100%";
      inner.style.height = config.height;
      if (props.className) inner.className = props.className;
      wrap.appendChild(inner);

      return () => {
        inner._zvMounted && _instances.get(inner.id)?.destroy();
        inner.remove();
      };
    }, []);

    return _h("div", {
      ref:                wrapRef,
      "data-zeb-wrapper": "VrmViewer",
      style:              { display: "contents" },
    });
  }

  /* SSR fallback */
  return _h("div", {
    "data-zeb-lib":     "threejs-vrm",
    "data-zeb-wrapper": "VrmViewer",
    "data-config":      JSON.stringify(config),
    id:                 props.id,
    style:              { width: "100%", height: config.height },
    class:              props.className,
  });
}
