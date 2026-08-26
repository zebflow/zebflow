(function () {
  function parseSize(value, fallback) {
    var n = Number(value);
    if (!Number.isFinite(n) || n <= 0) {
      return fallback;
    }
    return n;
  }

  function resolveThree(explicit) {
    if (explicit) return explicit;
    if (typeof globalThis !== "undefined" && globalThis.THREE) {
      return globalThis.THREE;
    }
    throw new Error("zeb/threejs: global THREE is missing");
  }

  function createSceneRuntime(canvas, options) {
    if (!(canvas instanceof HTMLCanvasElement)) {
      throw new Error("zeb/threejs: canvas element is required");
    }
    var cfg = options || {};
    var THREE = resolveThree(cfg.THREE);

    var width = parseSize(cfg.width, canvas.clientWidth || 800);
    var height = parseSize(cfg.height, canvas.clientHeight || 450);

    var renderer = new THREE.WebGLRenderer({
      canvas: canvas,
      antialias: cfg.antialias !== false,
      alpha: cfg.alpha !== false,
    });
    renderer.setPixelRatio(typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1);
    renderer.setSize(width, height, false);

    var scene = new THREE.Scene();
    scene.background = new THREE.Color(cfg.background || "#0b1020");

    var camera = new THREE.PerspectiveCamera(parseSize(cfg.fov, 60), width / height, 0.1, 1000);
    camera.position.set(0, 0, parseSize(cfg.cameraZ, 4));

    var geometry = new THREE.BoxGeometry(1, 1, 1);
    var material = new THREE.MeshNormalMaterial();
    var cube = new THREE.Mesh(geometry, material);
    scene.add(cube);

    var light = new THREE.DirectionalLight("#ffffff", 1.2);
    light.position.set(2, 2, 3);
    scene.add(light);

    var raf = 0;
    var running = true;
    var animate = function () {
      if (!running) {
        return;
      }
      cube.rotation.x += 0.01;
      cube.rotation.y += 0.015;
      renderer.render(scene, camera);
      raf = requestAnimationFrame(animate);
    };
    animate();

    var resize = function (nextWidth, nextHeight) {
      var w = parseSize(nextWidth, canvas.clientWidth || width);
      var h = parseSize(nextHeight, canvas.clientHeight || height);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h, false);
    };

    return {
      THREE: THREE,
      scene: scene,
      camera: camera,
      renderer: renderer,
      cube: cube,
      resize: resize,
      destroy: function () {
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

  function mountThreeScene(host, options) {
    if (!(host instanceof Element)) {
      throw new Error("zeb/threejs: host element is required");
    }
    var cfg = options || {};
    var canvas = document.createElement("canvas");
    canvas.className = cfg.canvasClassName || "w-full h-full";
    canvas.style.width = cfg.canvasWidth || "100%";
    canvas.style.height = cfg.canvasHeight || "100%";
    host.replaceChildren(canvas);
    var runtime = createSceneRuntime(canvas, cfg);
    runtime.host = host;
    runtime.canvas = canvas;
    return runtime;
  }

  var api = {
    createSceneRuntime: createSceneRuntime,
    mountThreeScene: mountThreeScene,
  };

  if (typeof globalThis !== "undefined") {
    globalThis.zebThree = api;
  }
})();
