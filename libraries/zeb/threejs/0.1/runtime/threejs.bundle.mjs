function parseSize(value, fallback) {
  const n = Number(value);
  if (!Number.isFinite(n) || n <= 0) {
    return fallback;
  }
  return n;
}

export function ensureThree(explicit) {
  if (explicit) {
    return explicit;
  }
  if (typeof globalThis !== "undefined" && globalThis.THREE) {
    return globalThis.THREE;
  }
  throw new Error(
    "zeb/threejs: Three.js runtime is missing. Provide globalThis.THREE or pass explicit THREE object.",
  );
}

export function createSceneRuntime(canvas, options = {}) {
  if (!(canvas instanceof HTMLCanvasElement)) {
    throw new Error("zeb/threejs: canvas element is required");
  }

  const THREE = ensureThree(options.THREE);

  const width = parseSize(options.width, canvas.clientWidth || 800);
  const height = parseSize(options.height, canvas.clientHeight || 450);

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: options.antialias !== false,
    alpha: options.alpha !== false,
  });
  renderer.setPixelRatio(typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1);
  renderer.setSize(width, height, false);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(options.background || "#0b1020");

  const camera = new THREE.PerspectiveCamera(
    parseSize(options.fov, 60),
    width / height,
    0.1,
    1000,
  );
  camera.position.set(0, 0, parseSize(options.cameraZ, 4));

  const geometry = new THREE.BoxGeometry(1, 1, 1);
  const material = new THREE.MeshNormalMaterial();
  const cube = new THREE.Mesh(geometry, material);
  scene.add(cube);

  const light = new THREE.DirectionalLight("#ffffff", 1.2);
  light.position.set(2, 2, 3);
  scene.add(light);

  let raf = 0;
  let running = true;
  const animate = () => {
    if (!running) {
      return;
    }
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
      if (raf) {
        cancelAnimationFrame(raf);
      }
      geometry.dispose();
      material.dispose();
      renderer.dispose();
    },
  };
}

export function mountThreeScene(host, options = {}) {
  if (!(host instanceof Element)) {
    throw new Error("zeb/threejs: host element is required");
  }
  const canvas = document.createElement("canvas");
  canvas.className = options.canvasClassName || "w-full h-full";
  canvas.style.width = options.canvasWidth || "100%";
  canvas.style.height = options.canvasHeight || "100%";
  host.replaceChildren(canvas);
  const runtime = createSceneRuntime(canvas, options);
  return {
    ...runtime,
    host,
    canvas,
  };
}

export const threejs = {
  ensureThree,
  createSceneRuntime,
  mountThreeScene,
};
