export const app = {};

/**
 * ThreeScene — Preact component for Three.js scenes in RWE templates.
 *
 * Uses useRef + useEffect to prevent Preact hydration conflicts.
 * The auto-mount system in threejs.bundle.mjs watches [data-zeb-lib="threejs"]
 * and calls mountThreeScene with the parsed data-config.
 *
 * Props:
 *   config      object   scene config: { background, cameraZ, fov, width, height, ... }
 *   height      string   CSS height (default "400px")
 *   id          string   container id for window.__zebThree.get(id)
 *   className   string   Tailwind classes on container
 */
export default function ThreeScene(props) {
  const _h         = globalThis.h;
  const _useRef    = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = Object.assign({}, props.config || {});
  const height = props.height || "400px";

  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const inner = document.createElement("div");
      inner.setAttribute("data-zeb-lib", "threejs");
      inner.setAttribute("data-config", JSON.stringify(config));
      if (props.id) inner.id = props.id;
      inner.style.width  = "100%";
      inner.style.height = height;
      if (props.className) inner.className = props.className;
      wrap.appendChild(inner);

      return () => { inner.remove(); };
    }, []);

    return _h("div", {
      ref:                wrapRef,
      "data-zeb-wrapper": "ThreeScene",
      style:              { display: "contents" },
    });
  }

  /* SSR fallback */
  return _h("div", {
    "data-zeb-lib":     "threejs",
    "data-zeb-wrapper": "ThreeScene",
    "data-config":      JSON.stringify(config),
    id:                 props.id,
    style:              { width: "100%", height },
    class:              props.className,
  });
}
