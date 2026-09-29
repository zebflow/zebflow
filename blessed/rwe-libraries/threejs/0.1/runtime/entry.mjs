/**
 * zeb/threejs 0.1 — every Three.js export, plus a scene runtime a page can
 * mount without writing a render loop.
 *
 * ── OFFLINE BUNDLE ───────────────────────────────────────────────────────────
 *  npm install three@<version> esbuild@0.25.12
 *  node_modules/.bin/esbuild entry.mjs --bundle --format=esm \
 *    --banner:js="// zeb/threejs — three r<rev> + scene runtime utilities" \
 *    --outfile=threejs.bundle.mjs
 */
import * as THREE from "three";
import {
  BoxGeometry, Color, DirectionalLight, Mesh, MeshNormalMaterial,
  PerspectiveCamera, Scene, WebGLRenderer,
} from "three";

export * from "three";

function parseSize(value, fallback) {
  const n = Number(value);
  if (!Number.isFinite(n) || n <= 0) return fallback;
  return n;
}
function createSceneRuntime(canvas, options = {}) {
  if (!(canvas instanceof HTMLCanvasElement)) {
    throw new Error("zeb/threejs: canvas element is required");
  }
  const width = parseSize(options.width, canvas.clientWidth || 800);
  const height = parseSize(options.height, canvas.clientHeight || 450);
  const renderer = new WebGLRenderer({
    canvas,
    antialias: options.antialias !== false,
    alpha: options.alpha !== false
  });
  renderer.setPixelRatio(typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1);
  renderer.setSize(width, height, false);
  const scene = new Scene();
  scene.background = new Color(options.background || "#0b1020");
  const camera = new PerspectiveCamera(
    parseSize(options.fov, 60),
    width / height,
    0.1,
    1e3
  );
  camera.position.set(0, 0, parseSize(options.cameraZ, 4));
  const geometry = new BoxGeometry(1, 1, 1);
  const material = new MeshNormalMaterial();
  const cube = new Mesh(geometry, material);
  scene.add(cube);
  const light = new DirectionalLight("#ffffff", 1.2);
  light.position.set(2, 2, 3);
  scene.add(light);
  let raf = 0;
  let running = true;
  const animate = () => {
    if (!running) return;
    cube.rotation.x += 0.01;
    cube.rotation.y += 0.015;
    renderer.render(scene, camera);
    raf = requestAnimationFrame(animate);
  };
  animate();
  const resize = (nextWidth, nextHeight) => {
    const w = parseSize(nextWidth, canvas.clientWidth || width);
    const h = parseSize(nextHeight, canvas.clientHeight || height);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderer.setSize(w, h, false);
  };
  return {
    THREE,
    scene,
    camera,
    renderer,
    cube,
    resize,
    destroy() {
      running = false;
      if (raf) cancelAnimationFrame(raf);
      geometry.dispose();
      material.dispose();
      renderer.dispose();
    }
  };
}
function mountThreeScene(host, options = {}) {
  if (!(host instanceof Element)) {
    throw new Error("zeb/threejs: host element is required");
  }
  const canvas = document.createElement("canvas");
  canvas.className = options.canvasClassName || "w-full h-full";
  canvas.style.width = options.canvasWidth || "100%";
  canvas.style.height = options.canvasHeight || "100%";
  host.replaceChildren(canvas);
  const runtime = createSceneRuntime(canvas, options);
  return { ...runtime, host, canvas };
}
export const threejs = { createSceneRuntime, mountThreeScene };
export { createSceneRuntime, mountThreeScene };
