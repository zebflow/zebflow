// preact_ssr_init.js — minimal self-contained preact-compatible SSR runtime.
//
// Loaded once into the deno_core JsRuntime at startup.
// Sets up all globals that RWE templates expect: h, Fragment, React,
// useState, useEffect, useRef, useMemo, useCallback, useContext, useReducer,
// createContext, usePageState, useNavigate, Link, and the internal
// __rweRenderToString / __rweWrapWithPageState helpers called from Rust.

(function () {
  "use strict";

  // ---------------------------------------------------------------------------
  // HTML / attribute escaping
  // ---------------------------------------------------------------------------
  function escHtml(s) {
    if (s == null) return "";
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function escAttr(s) {
    if (s == null) return "";
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  // ---------------------------------------------------------------------------
  // Void elements (self-closing in HTML5)
  // ---------------------------------------------------------------------------
  var VOID_TAGS = {
    area: 1, base: 1, br: 1, col: 1, embed: 1, hr: 1, img: 1, input: 1,
    link: 1, meta: 1, param: 1, source: 1, track: 1, wbr: 1,
  };

  // ---------------------------------------------------------------------------
  // Fragment sentinel
  // ---------------------------------------------------------------------------
  var Fragment = Symbol.for("preact.fragment");

  // ---------------------------------------------------------------------------
  // h() / createElement() — builds a virtual-DOM node
  // ---------------------------------------------------------------------------
  function h(type, props) {
    var children = [];
    for (var i = 2; i < arguments.length; i++) {
      children.push(arguments[i]);
    }
    return { type: type, props: props || {}, children: children };
  }

  // Flatten nested arrays of children into a single flat array.
  function flatKids(arr) {
    var out = [];
    for (var i = 0; i < arr.length; i++) {
      if (Array.isArray(arr[i])) {
        var sub = flatKids(arr[i]);
        for (var j = 0; j < sub.length; j++) out.push(sub[j]);
      } else {
        out.push(arr[i]);
      }
    }
    return out;
  }

  // ---------------------------------------------------------------------------
  // Attribute serialisation
  // ---------------------------------------------------------------------------
  function renderAttrs(props) {
    var out = "";
    if (!props) return out;
    for (var key in props) {
      if (!Object.prototype.hasOwnProperty.call(props, key)) continue;
      var val = props[key];
      // Skip internal / non-DOM props.
      if (
        key === "children" ||
        key === "key" ||
        key === "ref" ||
        key === "dangerouslySetInnerHTML"
      )
        continue;
      if (typeof val === "function") continue; // event handlers
      if (val == null || val === false) continue;
      // Map React names → HTML names.
      if (key === "className") {
        out += ' class="' + escAttr(val) + '"';
        continue;
      }
      if (key === "htmlFor") {
        out += ' for="' + escAttr(val) + '"';
        continue;
      }
      if (val === true) {
        out += " " + key;
        continue;
      }
      out += ' ' + key + '="' + escAttr(String(val)) + '"';
    }
    return out;
  }

  // ---------------------------------------------------------------------------
  // Core recursive renderer
  // ---------------------------------------------------------------------------
  function renderNode(node) {
    if (node == null || node === false || node === true) return "";
    if (typeof node === "string") return escHtml(node);
    if (typeof node === "number") return String(node);
    if (Array.isArray(node)) return flatKids(node).map(renderNode).join("");

    // Opaque raw-HTML marker emitted by context Providers.
    if (typeof node === "object" && node.__rweRaw !== undefined) {
      return node.__rweRaw;
    }

    if (typeof node !== "object" || node.type === undefined) return "";

    var type = node.type;
    var props = node.props || {};
    var children = flatKids(node.children || []);

    // Fragment
    if (type === Fragment || type === "__fragment__") {
      var fKids =
        children.length > 0
          ? children
          : props.children == null
            ? []
            : Array.isArray(props.children)
              ? props.children
              : [props.children];
      return fKids.map(renderNode).join("");
    }

    // Functional component
    if (typeof type === "function") {
      var cProps = Object.assign({}, props);
      if (children.length === 1) {
        cProps.children = children[0];
      } else if (children.length > 1) {
        cProps.children = children;
      }
      try {
        return renderNode(type(cProps));
      } catch (e) {
        return "<!-- RWE component error: " + escHtml(String(e)) + " -->";
      }
    }

    // DOM element
    var tag = String(type);
    var attrs = renderAttrs(props);

    if (VOID_TAGS[tag]) {
      return "<" + tag + attrs + ">";
    }

    // Children: explicit args take priority over props.children.
    var innerParts = children.map(renderNode);
    if (!children.length && props.children != null) {
      var pc = props.children;
      innerParts = (Array.isArray(pc) ? pc : [pc]).map(renderNode);
    }

    // dangerouslySetInnerHTML override
    var inner = props.dangerouslySetInnerHTML
      ? String(props.dangerouslySetInnerHTML.__html || "")
      : innerParts.join("");

    return "<" + tag + attrs + ">" + inner + "</" + tag + ">";
  }

  function renderToString(vnode) {
    return renderNode(vnode);
  }

  // ---------------------------------------------------------------------------
  // createContext
  // ---------------------------------------------------------------------------
  function createContext(defaultValue) {
    var ctx = { _currentValue: defaultValue };

    ctx.Provider = function Provider(props) {
      // Set context value for the duration of child rendering (synchronous SSR).
      ctx._currentValue = props.value;
      var kids = props.children;
      if (kids == null) return { __rweRaw: "" };
      var html = renderNode(Array.isArray(kids) ? h(Fragment, null, ...kids) : kids);
      return { __rweRaw: html };
    };

    ctx.Consumer = function Consumer(props) {
      var fn = props.children;
      if (typeof fn === "function") return fn(ctx._currentValue);
      return null;
    };

    return ctx;
  }

  // ---------------------------------------------------------------------------
  // SSR-safe hooks — all state is frozen at initial values during SSR
  // ---------------------------------------------------------------------------
  function useState(initial) {
    var val = typeof initial === "function" ? initial() : initial;
    return [val, function () {}];
  }

  function useEffect() {} // no-op

  function useLayoutEffect() {} // no-op

  function useInsertionEffect() {} // no-op

  function useRef(initial) {
    return { current: initial };
  }

  function useMemo(fn) {
    return fn();
  }

  function useCallback(fn) {
    return fn;
  }

  function useContext(ctx) {
    return ctx ? ctx._currentValue : undefined;
  }

  function useReducer(reducer, initial, init) {
    var state = init ? init(initial) : initial;
    return [state, function () {}];
  }

  function useId() {
    return "rwe-ssr-id";
  }

  function useImperativeHandle() {}

  function forwardRef(render) {
    return function (props) {
      return render(props, null);
    };
  }

  function memo(Component) {
    return Component;
  }

  // ---------------------------------------------------------------------------
  // Page-state context — shared mutable state across all components on a page
  // ---------------------------------------------------------------------------
  var PageStateContext = createContext(null);

  function createUsePageState() {
    return function usePageState(initial) {
      initial = initial || {};
      var ctx = useContext(PageStateContext);
      if (ctx && typeof ctx === "object") return ctx;
      return Object.assign({}, initial, { setPageState: function () {} });
    };
  }

  // ---------------------------------------------------------------------------
  // Navigation — SSR no-ops; browser hydration script has real implementations
  // ---------------------------------------------------------------------------
  function useNavigate() {
    return function (_href) {}; // no-op in SSR
  }

  function Link(props) {
    // Render as plain <a> for SEO / SSR.
    var href = props.href;
    var children = props.children;
    var rest = {};
    for (var k in props) {
      if (k !== "href" && k !== "children") rest[k] = props[k];
    }
    return h("a", Object.assign({ href: href }, rest), children);
  }

  // ---------------------------------------------------------------------------
  // wrapWithPageState — wraps a Page component with page-state context
  // ---------------------------------------------------------------------------
  function wrapWithPageState(Page, input) {
    input = input || {};
    // Set up context so usePageState() in any child component works.
    var ctxValue = Object.assign({}, input, { setPageState: function () {} });
    PageStateContext._currentValue = ctxValue;
    return h(Page, input);
  }

  // ---------------------------------------------------------------------------
  // Install all globals
  // ---------------------------------------------------------------------------
  globalThis.h = h;
  globalThis.Fragment = Fragment;
  globalThis.React = { createElement: h, Fragment: Fragment };
  globalThis.createElement = h;

  globalThis.useState = useState;
  globalThis.useEffect = useEffect;
  globalThis.useLayoutEffect = useLayoutEffect;
  globalThis.useInsertionEffect = useInsertionEffect;
  globalThis.useRef = useRef;
  globalThis.useMemo = useMemo;
  globalThis.useCallback = useCallback;
  globalThis.useContext = useContext;
  globalThis.useReducer = useReducer;
  globalThis.useId = useId;
  globalThis.useImperativeHandle = useImperativeHandle;
  globalThis.forwardRef = forwardRef;
  globalThis.memo = memo;

  globalThis.createContext = createContext;
  globalThis.usePageState = createUsePageState();
  globalThis.useNavigate = useNavigate;
  globalThis.Link = Link;
  globalThis.cx = function cx() {
    var out = [];
    for (var i = 0; i < arguments.length; i++) {
      if (arguments[i]) out.push(arguments[i]);
    }
    return out.join(" ");
  };

  // Internal helpers called by Rust after loading each page module.
  globalThis.__rweRenderToString = renderToString;
  globalThis.__rweWrapWithPageState = wrapWithPageState;
})();
