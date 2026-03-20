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
    return function usePageState(keyOrInitial, defaultValue) {
      var isKeyed = typeof keyOrInitial === "string";
      var ctx = useContext(PageStateContext);
      if (isKeyed) {
        var key = keyOrInitial;
        if (ctx && typeof ctx === "object") {
          var value = key in ctx ? ctx[key] : defaultValue;
          var setter = function (v) { if (ctx.setPageState) ctx.setPageState(function (p) { var o = {}; o[key] = v; return o; }); };
          return [value, setter];
        }
        // SSR root: no-op setter, just return default
        return [defaultValue, function () {}];
      }
      if (ctx && typeof ctx === "object") return ctx;
      return Object.assign({}, keyOrInitial || {}, { setPageState: function () {} });
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

  // ---------------------------------------------------------------------------
  // zeb/use SSR stubs — safe server-side fallbacks (bundle only runs client-side)
  // ---------------------------------------------------------------------------
  globalThis.useDebounce = function(value) { return value; };
  globalThis.useThrottle = function(value) { return value; };
  globalThis.useLocalStorage = function(key, init) { return [init, function() {}]; };
  globalThis.useClipboard = function() { return { copied: false, copy: function() {} }; };
  globalThis.useTemporaryState = function(init) { return [init, function() {}]; };
  globalThis.useWindowEvent = function() {};
  globalThis.useLazyModule = function() { return [null, true, null]; };
  globalThis.useSearchParams = function() { return [new URLSearchParams(), function() {}]; };
  globalThis.useSplitPane = function() { return { current: null }; };
  globalThis.useClickAway = function() { return { current: null }; };
  globalThis.useInterval = function() {};
  globalThis.useGeolocation = function() { return { loading: true, error: null, coords: null }; };
  globalThis.useTree = function() {
    return { expanded: new Set(), isExpanded: function() { return false; }, toggle: function() {}, expand: function() {}, collapse: function() {}, expandAll: function() {}, collapseAll: function() {} };
  };

  // ---------------------------------------------------------------------------
  // zeb/icons SSR stubs — icon components render null during SSR
  // ---------------------------------------------------------------------------
  (function() {
    var __nullIcon = function() { return null; };
    var __icons = [
      'ChevronLeft','ChevronRight','ChevronDown','ChevronUp',
      'ChevronsLeft','ChevronsRight','ChevronsUpDown',
      'ArrowLeft','ArrowRight','ArrowUp','ArrowDown',
      'Plus','Minus','X','Check','Search','Filter','RefreshCw','Pencil',
      'Trash2','Copy','Clipboard','Save','Download','Upload','ExternalLink',
      'Undo2','Redo2',
      'Eye','EyeOff','Lock','Unlock','Settings','Menu',
      'MoreHorizontal','MoreVertical','Maximize2','Minimize2',
      'PanelLeft','PanelRight','SidebarOpen','SidebarClose',
      'AlertCircle','AlertTriangle','Info','CheckCircle','CheckCircle2','XCircle','Loader2',
      'Database','TableIcon','Columns2','BarChart2','PieChart','TrendingUp','TrendingDown',
      'File','FileText','Folder','FolderOpen','Code2','Terminal',
      'User','Users','KeyRound','LogIn','LogOut',
      'Globe','Package','Zap','Star','Layers','LayoutGrid','ListIcon',
      'Cpu','Cloud','Wifi','Bell','BellOff','Tag','Bookmark','Hash','Slash','Sparkles'
    ];
    for (var i = 0; i < __icons.length; i++) {
      globalThis[__icons[i]] = __nullIcon;
    }
  })();

  // ---------------------------------------------------------------------------
  // zeb/prosemirror SSR stubs — ProseEditor renders a placeholder div
  // ---------------------------------------------------------------------------
  globalThis.mountProseEditor = function() { return Promise.resolve(null); };
  globalThis.prosemirror = { mountProseEditor: globalThis.mountProseEditor };
  globalThis.ProseEditor = function ProseEditor(props) {
    /* SSR stub — renders the sentinel div with the full data-config so the
     * client-side MutationObserver and bundle can pick up the correct config
     * on hydration.  Mirrors the ProseEditor export in prosemirror.bundle.mjs. */
    var config = JSON.stringify({
      content:     props.content,
      stateKey:    props.stateKey,
      statsKey:    props.statsKey,
      editable:    props.editable !== false,
      autofocus:   props.autofocus || false,
      placeholder: props.placeholder,
      toolbar:     props.toolbar !== undefined ? props.toolbar : 'basic',
      toolbarMode: props.toolbarMode || 'inline',
    });
    return globalThis.h('div', {
      'data-zeb-lib':     'prosemirror',
      'data-zeb-wrapper': 'ProseEditor',
      'data-config':      config,
      id:                 props.id,
      class:              props.className || 'w-full min-h-[200px]',
    });
  };

  // ---------------------------------------------------------------------------
  // zeb/icons devicons helpers — no-ops during SSR
  // ---------------------------------------------------------------------------
  globalThis.ensureDevicons = function() {};
  globalThis.dbKindIconClass = function() { return ""; };
  globalThis.dbObjectIconClass = function() { return ""; };

  // ---------------------------------------------------------------------------
  // zeb/threejs SSR stubs — Three.js is WebGL/browser-only.
  // Canvas/ThreeCanvas/ThreeScene render placeholder divs.
  // Three.js classes are empty constructors — only ever called inside useEffect,
  // which does not run during SSR renderToString.
  // ---------------------------------------------------------------------------
  globalThis.Canvas = function(props) {
    return h('div', { className: (props && props.className) || 'w-full h-full' });
  };
  globalThis.ThreeCanvas = globalThis.Canvas;
  globalThis.ThreeContext = createContext(null);
  globalThis.ThreeScene = function(props) {
    return h('div', {
      'data-zeb-lib': 'threejs',
      'data-zeb-wrapper': 'ThreeScene',
      'data-config': JSON.stringify((props && props.config) || {}),
      id: props && props.id,
      className: (props && props.className) || 'w-full h-full',
    });
  };
  globalThis.useThree = function() { return {}; };
  globalThis.useFrame = function() {};
  globalThis.OrbitControls = function() { return null; };
  globalThis.createSceneRuntime = function() { return {}; };
  globalThis.mountThreeScene = function() {};
  globalThis.ensureThree = function() { return {}; };
  globalThis.MathUtils = {};
  globalThis.REVISION = '183';
  (function() {
    var _cls = function() {};
    var _names = [
      'Scene','PerspectiveCamera','OrthographicCamera','WebGLRenderer',
      'Mesh','Group','Object3D','InstancedMesh','Points','Line',
      'BoxGeometry','SphereGeometry','PlaneGeometry','CylinderGeometry',
      'TorusGeometry','TorusKnotGeometry','ConeGeometry','RingGeometry','CircleGeometry','BufferGeometry',
      'MeshStandardMaterial','MeshBasicMaterial','MeshPhongMaterial','MeshLambertMaterial',
      'MeshNormalMaterial','MeshToonMaterial','MeshPhysicalMaterial','ShaderMaterial',
      'DirectionalLight','PointLight','SpotLight','AmbientLight','HemisphereLight',
      'Vector2','Vector3','Vector4','Quaternion','Euler','Matrix4','Color',
      'Raycaster','Clock','AnimationMixer','TextureLoader','CubeTextureLoader','Texture'
    ];
    for (var i = 0; i < _names.length; i++) { globalThis[_names[i]] = _cls; }
  })();

  // ---------------------------------------------------------------------------
  // zeb/threejs-vrm SSR stubs — VRM viewer is WebGL/browser-only.
  // ---------------------------------------------------------------------------
  globalThis.VrmViewer = function(props) {
    var cfg = JSON.stringify({
      modelUrl: (props && (props.modelUrl || props.model_url)) || '',
      height: (props && props.height) || '400px',
      background: (props && props.background) || 'transparent',
      autoRotate: !!(props && props.autoRotate),
      cameraZ: (props && props.cameraZ) || 1.5,
    });
    return h('div', {
      'data-zeb-lib': 'threejs-vrm',
      'data-zeb-wrapper': 'VrmViewer',
      'data-config': cfg,
      id: props && props.id,
      className: (props && props.className) || 'w-full h-full',
      style: { width: '100%', height: (props && props.height) || '400px' },
    });
  };
  globalThis.mountVrmViewer = function() {};

  // ---------------------------------------------------------------------------
  // zeb/deckgl SSR stubs — Deck.gl is WebGL/browser-only.
  // ---------------------------------------------------------------------------
  globalThis.DeckMap = function(props) {
    var cfg = JSON.stringify({
      initialViewState: props && props.initialViewState,
      controller: !props || props.controller !== false,
      layers: (props && props.layers) || [],
      stateKey: (props && props.stateKey) || null,
      layerKey: (props && props.layerKey) || null,
      background: (props && props.background) || 'transparent',
    });
    return h('div', {
      'data-zeb-lib': 'deckgl',
      'data-zeb-wrapper': 'DeckMap',
      'data-config': cfg,
      id: props && props.id,
      className: props && props.className,
      style: { width: '100%', height: (props && props.height) || '400px' },
    });
  };
  globalThis.deckgl = {};
  globalThis.buildLayer = function() { return null; };
  globalThis.buildLayers = function() { return []; };
  globalThis.mountDeckMap = function() {};
  globalThis.ensureDeck = function() {};
  globalThis.createDeckMapRuntime = function() { return {}; };

  // ---------------------------------------------------------------------------
  // zeb/d3 SSR stubs — D3 chart components render placeholder divs.
  // ---------------------------------------------------------------------------
  globalThis.d3 = {};
  globalThis.useD3 = function(callback, deps) {
    var ref = useRef(null);
    useEffect(function() {
      if (!ref.current) return;
      return callback(ref.current, {});
    }, deps || []);
    return ref;
  };
  globalThis.D3Bars = function(props) {
    var cfg = JSON.stringify({
      type: (props && props.type) || 'bar',
      data: (props && props.data) || [],
      xKey: props && props.xKey,
      yKey: props && props.yKey,
      stateKey: props && props.stateKey,
      height: (props && props.height) || '260px',
      colorScheme: props && props.colorScheme,
      area: !!(props && props.area),
    });
    return h('div', {
      'data-zeb-lib': 'd3',
      'data-zeb-wrapper': 'D3Bars',
      'data-config': cfg,
      id: props && props.id,
      className: props && props.className,
      style: { width: '100%', height: (props && props.height) || '260px' },
    });
  };

  // ---------------------------------------------------------------------------
  // zeb/graphui SSR stubs — graph canvas is browser-only.
  // ---------------------------------------------------------------------------
  globalThis.GraphCanvas = function(props) {
    return h('div', {
      'data-zeb-lib': 'graphui',
      'data-zeb-wrapper': 'GraphCanvas',
      id: props && props.id,
      className: (props && props.className) || 'w-full h-full',
    });
  };
  globalThis.PipelineGraph = function PipelineGraph(props) {
    return h('div', {
      'data-zeb-lib': 'graphui',
      'data-zeb-wrapper': 'PipelineGraph',
      id: props && props.id,
      className: (props && props.className) || 'w-full h-full',
    });
  };

  // ---------------------------------------------------------------------------
  // zeb/codemirror SSR stubs — code editor is browser-only.
  // ---------------------------------------------------------------------------
  globalThis.CodeEditor = function(props) {
    return h('div', {
      'data-zeb-lib': 'codemirror',
      'data-zeb-wrapper': 'CodeEditor',
      id: props && props.id,
      className: (props && props.className) || 'w-full h-full',
    });
  };

  // ---------------------------------------------------------------------------
  // zeb/markdown SSR stubs — Markdown component renders an encoded placeholder.
  // ---------------------------------------------------------------------------
  globalThis.Markdown = function(props) {
    var text = (props && props.content) || (typeof (props && props.children) === 'string' ? props.children : '') || '';
    var encoded = typeof btoa !== 'undefined' ? btoa(unescape(encodeURIComponent(text))) : text;
    return h('div', {
      'data-zeb-lib': 'markdown',
      'data-encoded': encoded,
      className: props && props.className,
    });
  };

  // ---------------------------------------------------------------------------
  // Page-state bridge — SSR no-ops.
  // The real implementations are installed by build_client_module in render.rs
  // inside __RweRoot after hydration. These stubs prevent ReferenceError when
  // zeb/* library bundles (e.g. zeb/prosemirror) call the bridge during SSR.
  // ---------------------------------------------------------------------------
  globalThis.__rweSetPageState = function() {};
  globalThis.__rwePageState = {};

  // Internal helpers called by Rust after loading each page module.
  globalThis.__rweRenderToString = renderToString;
  globalThis.__rweWrapWithPageState = wrapWithPageState;
})();
