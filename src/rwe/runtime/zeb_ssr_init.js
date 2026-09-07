// zeb_ssr_init.js — Zebflow SSR integration and browser-library placeholders.
//
// Loaded once into the deno_core JsRuntime at startup.
// Sets up all globals that RWE templates expect: h, Fragment, React,
// useState, useEffect, useRef, useMemo, useCallback, useContext, useReducer,
// createContext, usePageState, useRouter, Link, and the internal
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

  function base64EncodeUtf8(text) {
    if (typeof btoa !== "undefined") {
      return btoa(unescape(encodeURIComponent(text)));
    }
    if (typeof Buffer !== "undefined") {
      return Buffer.from(text, "utf-8").toString("base64");
    }
    var encodedText = encodeURIComponent(String(text || ""));
    var bytes = [];
    for (var j = 0; j < encodedText.length; j++) {
      var ch = encodedText.charAt(j);
      if (ch === "%") {
        bytes.push(parseInt(encodedText.slice(j + 1, j + 3), 16));
        j += 2;
      } else {
        bytes.push(encodedText.charCodeAt(j));
      }
    }
    var chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    var out = "";
    for (var i = 0; i < bytes.length; i += 3) {
      var b1 = bytes[i];
      var b2 = i + 1 < bytes.length ? bytes[i + 1] : 0;
      var b3 = i + 2 < bytes.length ? bytes[i + 2] : 0;
      var triplet = (b1 << 16) | (b2 << 8) | b3;
      out += chars[(triplet >> 18) & 63];
      out += chars[(triplet >> 12) & 63];
      out += i + 1 < bytes.length ? chars[(triplet >> 6) & 63] : "=";
      out += i + 2 < bytes.length ? chars[triplet & 63] : "=";
    }
    return out;
  }

  // Shared Zeb React primitives; loaded before this adapter in every V8 worker.
  var { h, Fragment, renderToString, createContext, useState, useEffect,
    useLayoutEffect, useRef, useMemo, useCallback, useContext, useReducer,
    useId, useImperativeHandle, useSyncExternalStore, forwardRef, memo,
    createPortal } = globalThis.__zebReact;
  function useInsertionEffect() {} // SSR-only legacy no-op.

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
  // Where the request arrived. The server is rendering one fixed URL, so these
  // read the payload the pipeline injected rather than subscribing to anything
  // — there is nothing here that can change mid-render.
  function usePathname() {
    var ctx = globalThis.ctx || {};
    return typeof ctx.route === "string" && ctx.route ? ctx.route : "/";
  }

  function useSearchParams() {
    var ctx = globalThis.ctx || {};
    var entries = [];
    var query = ctx.query;
    if (query && typeof query === "object") {
      for (var key in query) {
        if (Object.prototype.hasOwnProperty.call(query, key)) {
          entries.push([key, String(query[key])]);
        }
      }
    }
    // deno_core installs no browser URLSearchParams. Keep this a local,
    // read-only query view rather than installing a partial web API globally.
    // Rust's URL implementation owns decoding and canonical form encoding.
    var snapshot = Deno.core.ops.op_rwe_search_params_snapshot(
      typeof ctx.search === "string" ? ctx.search : null,
      entries
    );
    var pairs = snapshot.entries;
    var scalarString = function(value) {
      if (typeof value === "symbol") throw new TypeError("Cannot convert a Symbol value to a string");
      return String(value).toWellFormed();
    };
    var readonly = function() { throw new TypeError("Search params are read-only; use router.push or router.replace."); };
    var view = {
      get: function(name) {
        name = scalarString(name);
        for (var pair of pairs) if (pair[0] === name) return pair[1];
        return null;
      },
      getAll: function(name) {
        name = scalarString(name);
        return pairs.filter(function(pair) { return pair[0] === name; }).map(function(pair) { return pair[1]; });
      },
      has: function(name, value) {
        name = scalarString(name);
        var matchValue = value !== undefined;
        if (matchValue) value = scalarString(value);
        return pairs.some(function(pair) { return pair[0] === name && (!matchValue || pair[1] === value); });
      },
      entries: function() { return pairs.map(function(pair) { return pair.slice(); })[Symbol.iterator](); },
      keys: function() { return pairs.map(function(pair) { return pair[0]; })[Symbol.iterator](); },
      values: function() { return pairs.map(function(pair) { return pair[1]; })[Symbol.iterator](); },
      forEach: function(callback, thisArg) {
        if (typeof callback !== "function") throw new TypeError("Search params callback must be a function");
        pairs.forEach(function(pair) { callback.call(thisArg, pair[1], pair[0], view); });
      },
      toString: function() { return snapshot.encoded; },
      size: pairs.length,
      append: readonly,
      delete: readonly,
      set: readonly,
      sort: readonly,
    };
    view[Symbol.iterator] = view.entries;
    return Object.freeze(view);
  }

  // The server has no history and nobody clicking, so every method is a
  // no-op. That is the correct behaviour for a server, not a placeholder:
  // the same component code runs in both places and must not branch.
  function useRouter() {
    return {
      push: function (_href) {},
      replace: function (_href) {},
      back: function () {},
      forward: function () {},
      refresh: function () {},
      prefetch: function (_href) {},
    };
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
    return h(PageStateContext.Provider, { value: ctxValue }, h(Page, input));
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
  globalThis.useSyncExternalStore = useSyncExternalStore;
  globalThis.forwardRef = forwardRef;
  globalThis.memo = memo;
  globalThis.createPortal = createPortal;

  globalThis.createContext = createContext;
  globalThis.usePageState = createUsePageState();
  globalThis.useRouter = useRouter;
  globalThis.usePathname = usePathname;
  globalThis.useSearchParams = useSearchParams;
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
  // useSearchParams deliberately absent here: it belongs to zeb/react and is
  // installed above. A second one with a [value, setter] shape used to be
  // declared later in this file and silently replaced it.
  globalThis.useSplitPane = function() { return { current: null }; };
  globalThis.useClickAway = function() { return { current: null }; };
  globalThis.useInterval = function() {};
  globalThis.useGeolocation = function() { return { loading: true, error: null, coords: null }; };
  globalThis.useTree = function() {
    return { expanded: new Set(), isExpanded: function() { return false; }, toggle: function() {}, expand: function() {}, collapse: function() {}, expandAll: function() {}, collapseAll: function() {} };
  };

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
  // Devicon helpers — no-ops during SSR. The marks are a platform
  // stylesheet now; these remain so a template calling them still renders.
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
    var encoded = base64EncodeUtf8(text);
    return h('div', {
      'data-zeb-lib': 'markdown',
      'data-rwe-md': encoded,
      className: 'rwe-md-placeholder' + ((props && props.className) ? (' ' + props.className) : ''),
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
  // Keep RWE's existing partial-render error contract. Direct engine consumers
  // get exceptions; platform SSR retains its observable component-error marker.
  globalThis.__rweRenderToString = function (node) {
    return renderToString(node, { onError: function (error) {
      return '<!-- RWE component error: ' + escHtml(String(error)).replace(/--/g, '&#45;&#45;') + ' -->';
    } });
  };
  globalThis.__rweWrapWithPageState = wrapWithPageState;

  // ---------------------------------------------------------------------------
  // Island support for SSR — mirrors the client-side h() interceptor.
  // SSR versions always render children but emit the data-island-id marker so
  // the DOM structure matches the client vdom (preventing hydration mismatches).
  // __islandCounter is also reset from Rust in deno_worker.rs before each render.
  // ---------------------------------------------------------------------------
  globalThis.__islandCounter = 0;

  var __origSSRh = globalThis.h;

  function __IslandOff(p) { return p.children; }

  function __IslandOnView(p) {
    return __origSSRh('div', { 'data-island-id': p.id, 'data-hydrate': 'onview' }, p.children);
  }

  function __IslandOnInteract(p) {
    return __origSSRh('div', { 'data-island-id': p.id, 'data-hydrate': 'oninteract' }, p.children);
  }

  globalThis.h = function(type, props) {
    var args = Array.prototype.slice.call(arguments, 2);
    if (props && props.hydrate && props.hydrate !== 'onload') {
      var mode = props.hydrate;
      var id = 'island-' + (globalThis.__islandCounter++);
      var newProps = Object.assign({}, props);
      delete newProps.hydrate;
      newProps['data-island-id'] = id;
      var el = __origSSRh.apply(null, [type, newProps].concat(args));
      if (mode === 'off')        return __IslandOff({ children: el });
      if (mode === 'onview')     return __IslandOnView({ id: id, children: el });
      if (mode === 'oninteract') return __IslandOnInteract({ id: id, children: el });
    }
    return __origSSRh.apply(null, [type, props].concat(args));
  };
})();
