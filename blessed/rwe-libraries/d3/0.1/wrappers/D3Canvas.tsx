export const app = {};

/**
 * D3Canvas — general-purpose D3 canvas for arbitrary d3 code.
 *
 * After mount, the container element fires `zeb:d3:ready` with:
 *   { d3, container, id, instance }
 * Use `d3` to call any d3 function. No chart type is assumed.
 *
 * Props:
 *   height      string   CSS height (default "300px")
 *   config      object   arbitrary config passed through data-config
 *   id          string   container id
 *   className   string   Tailwind classes on container div
 */
export default function D3Canvas(props) {
  const _h         = globalThis.h;
  const _useRef    = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = Object.assign({ type: "raw" }, props.config || {});

  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const inner = document.createElement("div");
      inner.setAttribute("data-zeb-lib", "d3");
      inner.setAttribute("data-config", JSON.stringify(config));
      if (props.id) inner.id = props.id;
      inner.style.width  = "100%";
      inner.style.height = props.height || "300px";
      if (props.className) inner.className = props.className;
      wrap.appendChild(inner);

      return () => { inner.remove(); };
    }, []);

    return _h("div", {
      ref:                wrapRef,
      "data-zeb-wrapper": "D3Canvas",
      style:              { display: "contents" },
    });
  }

  /* SSR fallback */
  return _h("div", {
    "data-zeb-lib":     "d3",
    "data-zeb-wrapper": "D3Canvas",
    "data-config":      JSON.stringify(config),
    id:                 props.id,
    style:              { width: "100%", height: props.height || "300px" },
    class:              props.className,
  });
}
